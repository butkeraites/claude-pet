//! O [`Desktop`] do macOS. Por enquanto mínimo (T8.5): o monitor ativo é
//! seguido pelo próprio painel (`Painel::seguir_monitor`, por `NSScreen.main`).
//! Focar o terminal e a janela ativa de cada sessão vêm na parte do clique
//! (T8.7), com o `NSWorkspace` e o `NSRunningApplication`.

use pet_core::plataforma::{Alca, CapDesktop, Desktop, ErroFoco, InfoDesktop};

#[derive(Default)]
pub struct DesktopMac {}

impl DesktopMac {
    pub fn novo() -> DesktopMac {
        DesktopMac::default()
    }
}

impl Desktop for DesktopMac {
    fn capacidades(&self) -> CapDesktop {
        // v1: o painel segue o monitor sozinho; focar e janela ativa (T8.7)
        // ainda não.
        CapDesktop::default()
    }

    fn focar(&mut self, _alvo: &Alca) -> Result<(), ErroFoco> {
        Err(ErroFoco::NaoSuportado)
    }

    fn info(&self) -> InfoDesktop {
        InfoDesktop::default()
    }
}
