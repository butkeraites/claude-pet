//! O laço sem janela (decisão 0040): o backend do Windows até o M8, e o modo
//! sem NSPanel do macOS (`PET_SEM_JANELA=1`, usado pelos testes do daemon;
//! o backend de verdade do macOS é o `laco_macos`, decisão 0101).
//!
//! Só a `std`: a caixa acorda o `recv_timeout`, o prazo mais próximo do
//! Motor vira o tempo de espera, e o cérebro e o `/v1/estado` funcionam como
//! no Linux sem compositor (as reações ficam só no estado). No Linux este laço
//! só roda no teste que prova o [`Nucleo`] sem Wayland nem calloop.

#![cfg_attr(target_os = "linux", allow(dead_code))]

use std::net::TcpListener;
use std::process::ExitCode;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use pet_core::config::ConfigEfetiva;
use pet_core::plataforma::{Caixa, SemDespertador};

use crate::ambiente::Ambiente;
use crate::comando::{self, Comando};
use crate::estado::Compartilhado;
use crate::nucleo::Nucleo;
use crate::{daemon, vigia};

/// Espera máxima entre duas voltas: o batimento que o vigia e o `/saude`
/// leem.
const BATIMENTO: Duration = Duration::from_millis(crate::nucleo::BATIMENTO_MS);

/// O daemon sem janela, até a caixa fechar (o processo é morto antes disso).
pub fn rodar(
    ambiente: Ambiente,
    config: ConfigEfetiva,
    comp: Arc<Compartilhado>,
    ouvinte: TcpListener,
    motivo: &str,
) -> ExitCode {
    aviso!("{motivo}: seguindo sem janela (o cérebro e o /v1/estado funcionam)");
    let (caixa, recebe) = Caixa::nova(comando::CAPACIDADE, Arc::new(SemDespertador));
    // Os sinais acordam o laço pela própria caixa (não há calloop aqui): um
    // SIGTERM manda `Encerrar`, o laço sai e grava a memória das sessões antes
    // de o processo morrer (decisões 0040 e 0093).
    #[cfg(unix)]
    instalar_sinais(caixa.clone());
    if let Err(e) = daemon::iniciar_entrada(&ambiente, &comp, ouvinte, caixa) {
        erro!("{e}");
        return ExitCode::FAILURE;
    }
    vigia::iniciar(Arc::clone(&comp));
    let mut nucleo = Nucleo::novo(
        Arc::clone(&comp),
        ambiente.onde(),
        Some(ambiente.pasta_config.clone()),
        &config,
        Instant::now(),
    );
    nucleo.iniciar_cerebro();
    info!(
        "bichinho {} escutando em {} (porta pública {}), sem janela",
        pet_core::VERSAO,
        ambiente.escuta,
        ambiente.porta_publica
    );
    laco(&mut nucleo, &recebe);
    // Na saída (sinal ou caixa fechada), a memória das sessões vai para o
    // disco, como o `encerrar` do laço do Linux faz (decisão 0093).
    nucleo.guardar_memoria(true);
    ExitCode::SUCCESS
}

/// Uma thread que espera SIGTERM/SIGINT e manda `Encerrar` pela caixa (uma
/// vez). Sem calloop, é o jeito de o `recv` do laço acordar na hora do sinal.
#[cfg(unix)]
fn instalar_sinais(caixa: Caixa<Comando>) {
    use signal_hook::consts::{SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;

    let mut sinais = match Signals::new([SIGTERM, SIGINT]) {
        Ok(s) => s,
        Err(e) => {
            aviso!("não consegui tratar SIGTERM/SIGINT: {e}");
            return;
        }
    };
    let feito = std::thread::Builder::new()
        .name("sinais".into())
        .spawn(move || {
            if sinais.forever().next().is_some() {
                info!("encerrando: sinal recebido");
                // Se a caixa já fechou, o laço já está saindo: nada a fazer.
                let _ = caixa.mandar(Comando::Encerrar);
            }
        });
    if let Err(e) = feito {
        aviso!("não consegui iniciar a thread de sinais: {e}");
    }
}

/// Roda até a caixa fechar: os comandos da caixa primeiro, depois o prazo
/// que venceu (decisão 0032), e o painel a cada volta.
pub fn laco(nucleo: &mut Nucleo, recebe: &mpsc::Receiver<Comando>) {
    // O estado inicial (as sessões restauradas e as intenções da partida) sai
    // antes da primeira espera: sem eventos, o `recv` abaixo dormiria até o
    // batimento, e o `/v1/estado` só mostraria a restauração 5 s depois. O laço
    // do Linux já publica no `assentar` da partida.
    nucleo.publicar(None);
    loop {
        nucleo.comp.bater();
        let espera = nucleo
            .proximo_prazo(None)
            .map_or(BATIMENTO, |prazo| {
                Duration::from_millis(prazo.saturating_sub(nucleo.agora_ms()))
            })
            .min(BATIMENTO);
        match recebe.recv_timeout(espera) {
            Ok(comando) => {
                if matches!(comando, Comando::Encerrar) {
                    return;
                }
                nucleo.comando(comando, None);
                while let Ok(comando) = recebe.try_recv() {
                    if matches!(comando, Comando::Encerrar) {
                        return;
                    }
                    nucleo.comando(comando, None);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        if nucleo
            .proximo_prazo(None)
            .is_some_and(|prazo| prazo <= nucleo.agora_ms())
        {
            nucleo.vencer(None);
        }
        nucleo.publicar(None);
        // A memória das sessões, se mudou (no máximo a cada 4 s; decisão
        // 0093).
        nucleo.guardar_memoria(false);
    }
}

#[cfg(test)]
mod testes {
    use std::path::PathBuf;
    use std::thread;

    use pet_core::evento::Evento;

    use super::*;
    use crate::comando::Recebido;
    use crate::ingress::agora_desde_1970_ms;
    use crate::personagem::Onde;

    fn evento(e: &str) -> Comando {
        Comando::Evento(Box::new(Recebido {
            evento: Evento {
                e: e.into(),
                sid: Some("c0ffee00-sem-janela".into()),
                turno: Some("p1".into()),
                ent: Some("cli".into()),
                ..Evento::default()
            },
            recebido_ms: agora_desde_1970_ms(),
            chegada: Instant::now(),
        }))
    }

    #[test]
    fn cerebro_reage_sem_janela_nem_calloop() {
        let config = ConfigEfetiva::carregar(None, |_| None);
        let comp = Arc::new(Compartilhado::novo(config.clone(), false));
        let onde = Onde {
            busca: vec![PathBuf::from("/nao/existe")],
            estado: PathBuf::from("/nao/existe"),
            debug: false,
            debug_personagem: false,
        };
        let mut nucleo = Nucleo::novo(Arc::clone(&comp), onde, None, &config, Instant::now());
        let (caixa, recebe) = Caixa::nova(16, Arc::new(SemDespertador));
        let (tocou, resposta) = mpsc::sync_channel(1);
        let olhando = Arc::clone(&comp);
        let mandar = thread::spawn(move || {
            caixa.tentar(evento("UserPromptSubmit")).unwrap();
            caixa.tentar(evento("Stop")).unwrap();
            caixa
                .tentar(Comando::Tocar {
                    reacao: "nod".into(),
                    resposta: tocou,
                })
                .unwrap();
            // A acomodação do Stop é de 0,8 s: a caixa só fecha (e o laço
            // termina) depois que a reação aparece no estado, ou em 10 s.
            // Nada de prazo fixo: com a máquina carregada o laço pode
            // acordar atrasado.
            let limite = Instant::now() + Duration::from_secs(10);
            while olhando.estado_json()["ultima_reacao"].is_null() && Instant::now() < limite {
                thread::sleep(Duration::from_millis(20));
            }
        });
        laco(&mut nucleo, &recebe);
        mandar.join().unwrap();
        assert_eq!(
            resposta.try_recv(),
            Ok(pet_core::motor::Tocou::SemPersonagem)
        );
        let estado = comp.estado_json();
        assert_eq!(estado["ultima_reacao"]["nome"], "nod", "{estado:#}");
        assert_eq!(estado["turnos"][0]["nivel"], "T0");
        assert!(comp.saudavel());
        assert_eq!(estado["visivel"], false, "sem janela");
    }
}
