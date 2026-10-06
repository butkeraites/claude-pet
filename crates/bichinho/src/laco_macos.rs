//! O laço do macOS (M8, T8.5): a thread principal roda o run loop do AppKit em
//! fatias e, entre elas, faz o que o `sem_janela` faz (esvaziar a caixa, vencer
//! os prazos, publicar, gravar a memória), mas com um [`Punho`] de verdade — o
//! `pet_macos::PunhoMac` (NSPanel + NSWorkspace). A entrada HTTP acorda a
//! thread principal pelo [`pet_macos::Despertador`] (CFRunLoopWakeUp).
//!
//! O backend e as decisões do spike estão em `crates/pet-macos` (decisão 0101).

use std::net::TcpListener;
use std::process::ExitCode;
use std::sync::{Arc, mpsc};
use std::time::Instant;

use pet_core::config::ConfigEfetiva;
use pet_core::plataforma::{Caixa, Punho};

use crate::ambiente::Ambiente;
use crate::comando::{self, Comando};
use crate::estado::Compartilhado;
use crate::nucleo::Nucleo;
use crate::{daemon, vigia};

/// Espera máxima entre duas voltas quando o pet está escondido (o batimento do
/// vigia e do `/saude`).
const BATIMENTO_MS: u64 = 5_000;
/// Com o pet na tela, a volta é curta: o click-through pelo plano B lê a
/// posição do ponteiro a cada volta (decisão 0101). O orçamento de commits não
/// muda: estas voltas não desenham nada por conta própria.
const POLL_VISIVEL_MS: u64 = 100;

pub fn rodar(
    ambiente: Ambiente,
    config: ConfigEfetiva,
    comp: Arc<Compartilhado>,
    ouvinte: TcpListener,
) -> ExitCode {
    pet_macos::preparar();

    let (caixa, recebe) = Caixa::nova(comando::CAPACIDADE, Arc::new(pet_macos::Despertador));
    // SIGTERM/SIGINT: a thread de sinais manda `Encerrar` pela caixa, que
    // acorda o run loop (o mesmo esquema do `sem_janela`).
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

    let mut punho = pet_macos::PunhoMac::novo();
    // Conecta a janela: o Motor cria o painel se o pet deve aparecer (a camada
    // anuncia `Pronta`, que a primeira volta consome e desenha).
    nucleo.conectou(&mut punho);

    info!(
        "bichinho {} escutando em {} (porta pública {}), painel nativo do macOS",
        pet_core::VERSAO,
        ambiente.escuta,
        ambiente.porta_publica
    );
    nucleo.publicar(Some(&punho));

    laco(&mut nucleo, &mut punho, &recebe);

    // Na saída, grava a memória das sessões (decisão 0093) e fecha a janela.
    nucleo.guardar_memoria(true);
    nucleo.encerrar(&mut punho);
    ExitCode::SUCCESS
}

fn laco(nucleo: &mut Nucleo, punho: &mut pet_macos::PunhoMac, recebe: &mpsc::Receiver<Comando>) {
    loop {
        comp_bater(nucleo);
        let agora = nucleo.agora_ms();
        punho.painel.marcar_tempo(agora);

        // Os comandos que chegaram (eventos dos hooks, reações, aprovações).
        if esvaziar_caixa(nucleo, punho, recebe) {
            return; // Encerrar ou caixa fechada
        }

        // O prazo que venceu (cérebro e animação), depois de esvaziar a caixa
        // (decisão 0032: um evento que chegou antes cancela uma acomodação).
        if nucleo
            .proximo_prazo(Some(punho))
            .is_some_and(|p| p <= nucleo.agora_ms())
        {
            nucleo.vencer(Some(punho));
        }

        // O app em foco (o terminal de cada sessão): anota as ativações antes
        // de drenar os eventos do desktop, para o anel casar com o `ts` do
        // hook (decisão 0055).
        punho.desktop.pollar();

        // Eventos da janela e do desktop (ponteiro, Pronta, Saiu, JanelaAtiva).
        nucleo.eventos_da_janela(Some(punho));

        // Segue o monitor ativo e atualiza o click-through (plano B).
        punho.painel.seguir_monitor();
        punho.painel.atualizar_click_through();

        nucleo.publicar(Some(punho));
        nucleo.guardar_memoria(false);

        // Quanto esperar: até o próximo prazo, mas curto se o pet está na tela
        // (para o click-through responder ao ponteiro).
        let agora = nucleo.agora_ms();
        let teto = if pet_na_tela(punho) {
            POLL_VISIVEL_MS
        } else {
            BATIMENTO_MS
        };
        let espera_ms = nucleo
            .proximo_prazo(Some(punho))
            .map(|p| p.saturating_sub(agora))
            .unwrap_or(BATIMENTO_MS)
            .min(teto);
        pet_macos::rodar_fatia(espera_ms as f64 / 1000.0);
    }
}

/// Esvazia a caixa: `true` se o laço deve parar (Encerrar ou caixa fechada).
fn esvaziar_caixa(
    nucleo: &mut Nucleo,
    punho: &mut pet_macos::PunhoMac,
    recebe: &mpsc::Receiver<Comando>,
) -> bool {
    loop {
        match recebe.try_recv() {
            Ok(Comando::Encerrar) => return true,
            Ok(comando) => nucleo.comando(comando, Some(punho)),
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => return true,
        }
    }
}

fn pet_na_tela(punho: &pet_macos::PunhoMac) -> bool {
    use pet_core::plataforma::Fase;
    matches!(punho.ver_janela().fase(), Fase::Viva { .. })
}

fn comp_bater(nucleo: &Nucleo) {
    nucleo.comp.bater();
}

/// SIGTERM/SIGINT numa thread, que manda `Encerrar` pela caixa (o mesmo do
/// `sem_janela`).
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
                let _ = caixa.mandar(Comando::Encerrar);
            }
        });
    if let Err(e) = feito {
        aviso!("não consegui iniciar a thread de sinais: {e}");
    }
}
