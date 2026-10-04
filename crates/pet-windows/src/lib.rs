//! Backend do Windows (M8, T8.4; decisões 0038 e 0040).
//!
//! Vazio por enquanto, mas compilando: o `cargo check --target
//! x86_64-pc-windows-msvc` prova que o núcleo (`pet-core`) e o daemon não
//! dependem de nada do Linux. O que vem aqui, pela pesquisa
//! (`docs/pesquisa/09-multiplataforma.md`):
//!
//! - a janela pequena que anda (`WS_EX_LAYERED | TOOLWINDOW | NOACTIVATE |
//!   TOPMOST`, DIB de 32 bits pré-multiplicado, clique pelo alfa) como
//!   [`pet_core::plataforma::Overlay`];
//! - o monitor ativo e o foco da janela da sessão como
//!   [`pet_core::plataforma::Desktop`];
//! - o laço `MsgWaitForMultipleObjectsEx`, com a
//!   [`pet_core::plataforma::Caixa`] acordando o laço por `PostMessageW`.
//!
//! O hook dos plugins (`bichinho avisar`) já compila para este sistema: só
//! usa a `std` (decisão 0041); rodar de verdade pede uma máquina ou o CI
//! (T8.2). Até a janela chegar, o daemon roda o laço sem janela (o cérebro e
//! o `/v1/estado`).
//!
//! O `unsafe` do Win32 fica só neste crate, com `// SAFETY:` em cada bloco
//! (o workspace o proíbe no resto). Hoje nem isso: o crate não tem `unsafe`.

#![cfg(windows)]

use pet_core::plataforma::{CapDesktop, CapOverlay};

/// O que a janela do Windows vai saber fazer (nada, por enquanto).
pub fn capacidades() -> (CapOverlay, CapDesktop) {
    (CapOverlay::default(), CapDesktop::default())
}

/// Por que o pet ainda não aparece aqui: o daemon roda o laço sem janela
/// (o cérebro e o `/v1/estado` funcionam) até a janela do T8.4 chegar.
pub const SEM_JANELA: &str = "o pet ainda não aparece no Windows (M8, T8.4)";
