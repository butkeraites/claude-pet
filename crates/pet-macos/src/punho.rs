//! O [`Punho`] do macOS: a janela ([`Painel`]) e a ligação com o desktop
//! ([`DesktopMac`]) juntas, como o Wayland tem numa `Sessao`. No macOS a
//! janela é persistente (não recria a cada reconexão), então o punho nasce uma
//! vez no laço.

use objc2::MainThreadMarker;

use pet_core::plataforma::{Desktop, Overlay, Punho};

use crate::desktop::DesktopMac;
use crate::painel::Painel;

pub struct PunhoMac {
    pub painel: Painel,
    pub desktop: DesktopMac,
}

impl PunhoMac {
    /// Cria o punho na thread principal (o daemon chama o laço de lá). O
    /// `MainThreadMarker` fica dentro deste crate, longe do `bichinho`.
    pub fn novo() -> PunhoMac {
        let mtm = MainThreadMarker::new().expect("o laço do macOS roda na thread principal");
        PunhoMac {
            painel: Painel::novo(mtm),
            desktop: DesktopMac::novo(),
        }
    }
}

impl Punho for PunhoMac {
    fn janela(&mut self) -> &mut dyn Overlay {
        &mut self.painel
    }

    fn desktop(&mut self) -> &mut dyn Desktop {
        &mut self.desktop
    }

    fn ver_janela(&self) -> &dyn Overlay {
        &self.painel
    }

    fn ver_desktop(&self) -> &dyn Desktop {
        &self.desktop
    }
}
