//! O laço sem janela (decisão 0040): o daemon nos sistemas que ainda não têm
//! backend (Windows e macOS, até o M8).
//!
//! Só a `std`: a caixa acorda o `recv_timeout`, o prazo mais próximo do
//! Motor vira o tempo de espera, e o cérebro e o `/v1/estado` funcionam como
//! no Linux sem compositor (as reações ficam só no estado). No Linux este
//! laço só roda nos testes, que provam o [`Nucleo`] sem Wayland nem calloop.

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
const BATIMENTO: Duration = Duration::from_secs(5);

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
    ExitCode::SUCCESS
}

/// Roda até a caixa fechar: os comandos da caixa primeiro, depois o prazo
/// que venceu (decisão 0032), e o painel a cada volta.
pub fn laco(nucleo: &mut Nucleo, recebe: &mpsc::Receiver<Comando>) {
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
                nucleo.comando(comando, None);
                while let Ok(comando) = recebe.try_recv() {
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
