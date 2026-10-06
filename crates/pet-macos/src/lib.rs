//! Backend do macOS (M8, T8.5; decisões 0040, 0100 e 0101).
//!
//! Um `NSPanel` não ativador, pequeno, que anda (um por conexão; a janela é
//! persistente, ao contrário do Wayland que recria a camada): o conteúdo é um
//! `CALayer` com um `CGImage` BGRA pré-multiplicado e filtro nearest, como o
//! [`pet_core::plataforma::Overlay`]. A ligação com o desktop
//! ([`pet_core::plataforma::Desktop`]) vem do `NSWorkspace` (monitor em foco,
//! app ativo) e ativa o app do terminal para focar.
//!
//! O que o spike (decisão 0101) decidiu e vale aqui:
//! - painel não ativador, borderless, nível alto, `CanJoinAllSpaces |
//!   FullScreenAuxiliary | Stationary | IgnoresCycle`;
//! - **click-through pelo plano B**: o alfa não atravessa com `CALayer`, então
//!   o painel alterna `ignoresMouseEvents` pela posição do ponteiro (dentro da
//!   caixa de toque do pet, pega; fora, atravessa);
//! - monitor ativo por `NSScreen.main`;
//! - App Nap não atrapalhou, mas o laço segura `beginActivity` por garantia.
//!
//! O `unsafe` do AppKit fica só neste crate, com `// SAFETY:` em cada bloco.

#![cfg(target_os = "macos")]
#![allow(unsafe_op_in_unsafe_fn)]

#[macro_use]
extern crate pet_core;

use pet_core::plataforma::{CapDesktop, CapOverlay};

mod app;
mod ax;
mod desktop;
mod painel;
mod permissoes;
mod pixels;
mod punho;

pub use app::{Despertador, preparar, rodar_fatia};
pub use desktop::DesktopMac;
pub use painel::Painel;
pub use permissoes::permissoes;
pub use punho::PunhoMac;

/// As capacidades da janela pequena do macOS (decisão 0101).
pub fn cap_overlay() -> CapOverlay {
    CapOverlay {
        // Janela pequena que anda: não é o monitor inteiro (o confete do
        // estresse espera o palco transitório do M8).
        tela_inteira: false,
        // Só a caixa de toque pega clique; o resto atravessa (plano B).
        regiao_de_toque: true,
        // O `backingScaleFactor` do macOS pode ser fracionário; desenhamos em
        // pixels do dispositivo (D×D).
        escala_fracionaria: true,
        // Cursor próprio (mão aberta/fechada) por cima do pet.
        cursor: true,
        // Sem frame callback: a animação vem dos prazos do Motor, e o
        // `desenhar` nunca adia (não há um quadro em voo de cada vez).
        ritmo_do_compositor: false,
    }
}

/// As capacidades da ligação com o desktop no macOS.
pub fn cap_desktop() -> CapDesktop {
    CapDesktop {
        segue_foco: true,
        janela_ativa: true,
        foca_janela: true,
        // O "não perturbe"/Focus do macOS fica para depois.
        nao_perturbe: false,
    }
}

/// Mantida para o `daemon::rodar_no_sistema` até o laço do macóS entrar; o
/// backend de verdade é o [`PunhoMac`] com o `laco_macos`.
pub const SEM_JANELA: &str = "o pet no macOS roda pelo laço nativo (M8, T8.5)";
