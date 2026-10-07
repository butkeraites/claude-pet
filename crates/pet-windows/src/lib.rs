//! Backend do Windows (M8): a janela pequena que anda (`WS_EX_LAYERED |
//! TOPMOST | TOOLWINDOW | NOACTIVATE`, DIB BGRA pré-multiplicado desenhado com
//! `UpdateLayeredWindow`) como [`pet_core::plataforma::Overlay`], o monitor e a
//! janela ativos como [`pet_core::plataforma::Desktop`], e o laço de mensagens
//! (`MsgWaitForMultipleObjectsEx` + `PeekMessageW`), com a
//! [`pet_core::plataforma::Caixa`] acordando o laço por `PostThreadMessageW`.
//!
//! O palco do Motor (pixels do dispositivo, origem no topo esquerda do monitor)
//! é o próprio sistema de coordenadas da tela no Windows, então as conversões
//! são triviais (`pixels::Tela`), sem os flips do macOS.
//!
//! O `unsafe` do Win32 fica só neste crate, com `// SAFETY:` em cada bloco.

#![cfg(windows)]

mod app;
mod desktop;
mod painel;
mod pixels;
mod punho;

pub use app::{Despertador, instalar_encerramento, pediram_encerrar, preparar, rodar_fatia};
pub use desktop::DesktopWin;
pub use painel::Painel;
pub use punho::PunhoWin;

use pet_core::plataforma::{CapDesktop, CapOverlay};

/// O que a janela do Windows sabe fazer (como o macOS: janela pequena que anda,
/// com área de toque e cursor próprios; o ritmo é do laço, não do compositor).
pub fn cap_overlay() -> CapOverlay {
    CapOverlay {
        tela_inteira: false,
        regiao_de_toque: true,
        escala_fracionaria: true,
        cursor: true,
        ritmo_do_compositor: false,
    }
}

/// O que a ligação com o desktop sabe fazer. A janela segue o monitor sozinha
/// (como o macOS), então `segue_foco` é falso; o rastreio da janela ativa e o
/// foco/clique entram num corte seguinte.
pub fn cap_desktop() -> CapDesktop {
    CapDesktop {
        segue_foco: false,
        janela_ativa: false,
        foca_janela: false,
        nao_perturbe: false,
    }
}

/// Fallback: enquanto o backend não está ligado no daemon, o laço sem janela
/// roda (o cérebro e o `/v1/estado`).
pub const SEM_JANELA: &str = "o pet ainda não aparece no Windows (M8)";
