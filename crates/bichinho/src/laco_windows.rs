//! O laço do Windows (M8): a thread principal roda o laço de mensagens do Win32
//! em fatias (`MsgWaitForMultipleObjectsEx` + `PeekMessageW`) e, entre elas, faz
//! o que o `sem_janela` faz (esvaziar a caixa, vencer os prazos, publicar,
//! gravar a memória), mas com um [`Punho`] de verdade — o
//! `pet_windows::PunhoWin` (janela layered + `UpdateLayeredWindow`). A entrada
//! HTTP acorda a thread principal pelo [`pet_windows::Despertador`]
//! (`PostThreadMessageW`), e um Ctrl+C / fechar o console encerra com calma.
//!
//! O backend fica em `crates/pet-windows`. O foco/clique e o arraste entram num
//! corte seguinte (como o rastreio da janela ativa no `DesktopWin`).

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
/// Com o pet na tela, a volta é curta (o mesmo que o macOS): o orçamento de
/// commits não muda, estas voltas não desenham nada por conta própria.
const POLL_VISIVEL_MS: u64 = 100;

pub fn rodar(
    ambiente: Ambiente,
    config: ConfigEfetiva,
    comp: Arc<Compartilhado>,
    ouvinte: TcpListener,
) -> ExitCode {
    pet_windows::preparar();
    pet_windows::instalar_encerramento();

    let (caixa, recebe) = Caixa::nova(comando::CAPACIDADE, Arc::new(pet_windows::Despertador));
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

    let mut punho = pet_windows::PunhoWin::novo();
    nucleo.conectou(&mut punho);

    info!(
        "bichinho {} escutando em {} (porta pública {}), janela nativa do Windows",
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

fn laco(nucleo: &mut Nucleo, punho: &mut pet_windows::PunhoWin, recebe: &mpsc::Receiver<Comando>) {
    loop {
        nucleo.comp.bater();
        let agora = nucleo.agora_ms();
        punho.painel.marcar_tempo(agora);

        // Os comandos que chegaram (eventos dos hooks, reações, aprovações).
        if esvaziar_caixa(nucleo, punho, recebe) {
            return; // caixa fechada
        }
        // Ctrl+C / fechar o console: sai com calma.
        if pet_windows::pediram_encerrar() {
            info!("encerrando: Ctrl+C / console fechado");
            return;
        }

        // O prazo que venceu (cérebro e animação), depois de esvaziar a caixa
        // (decisão 0032: um evento que chegou antes cancela uma acomodação).
        if nucleo
            .proximo_prazo(Some(punho))
            .is_some_and(|p| p <= nucleo.agora_ms())
        {
            nucleo.vencer(Some(punho));
        }

        // Simetria com o macOS (o rastreio da janela ativa entra depois).
        punho.desktop.pollar();

        // Eventos da janela (Pronta, Saiu) e do desktop (Ligado).
        nucleo.eventos_da_janela(Some(punho));

        // Segue o monitor ativo (a janela faz sozinha, como o macOS).
        punho.painel.seguir_monitor();
        punho.painel.atualizar_click_through();

        nucleo.publicar(Some(punho));
        nucleo.guardar_memoria(false);

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
        pet_windows::rodar_fatia(espera_ms as f64 / 1000.0);
    }
}

/// Esvazia a caixa: `true` se o laço deve parar (Encerrar ou caixa fechada).
fn esvaziar_caixa(
    nucleo: &mut Nucleo,
    punho: &mut pet_windows::PunhoWin,
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

fn pet_na_tela(punho: &pet_windows::PunhoWin) -> bool {
    use pet_core::plataforma::Fase;
    matches!(punho.ver_janela().fase(), Fase::Viva { .. })
}
