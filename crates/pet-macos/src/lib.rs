//! Backend do macOS (M8, T8.5; decisões 0038 e 0040).
//!
//! Vazio por enquanto, mas compilando: o `cargo check --target
//! aarch64-apple-darwin` prova que o núcleo (`pet-core`) e o daemon não
//! dependem de nada do Linux. O que vem aqui, pela pesquisa
//! (`docs/pesquisa/09-multiplataforma.md`):
//!
//! - o `NSPanel` não ativador em todos os Spaces, com o conteúdo num
//!   `CALayer` (BGRA pré-multiplicado, filtro nearest), como
//!   [`pet_core::plataforma::Overlay`];
//! - o monitor ativo (`NSScreen.main`) e ativar o app do terminal como
//!   [`pet_core::plataforma::Desktop`];
//! - o laço `NSApplication.run`, com a [`pet_core::plataforma::Caixa`]
//!   acordando o laço pela fila principal.
//!
//! O `unsafe` do AppKit fica só neste crate, com `// SAFETY:` em cada bloco
//! (o workspace o proíbe no resto). Hoje nem isso: o crate não tem `unsafe`.

#![cfg(target_os = "macos")]

use pet_core::plataforma::{CapDesktop, CapOverlay};

/// O que o painel do macOS vai saber fazer (nada, por enquanto).
pub fn capacidades() -> (CapOverlay, CapDesktop) {
    (CapOverlay::default(), CapDesktop::default())
}

/// Por que o pet ainda não aparece aqui: o daemon roda o laço sem janela
/// (o cérebro e o `/v1/estado` funcionam) até a janela do T8.5 chegar.
pub const SEM_JANELA: &str = "o pet ainda não aparece no macOS (M8, T8.5)";
