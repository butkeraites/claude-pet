//! O [`Punho`] do Windows: a janela ([`Painel`]) e a ligação com o desktop
//! ([`DesktopWin`]) juntas, como o macOS. A janela é persistente (nasce uma vez
//! no laço, não recria a cada volta).

use pet_core::plataforma::{Desktop, Overlay, Punho};

use crate::desktop::DesktopWin;
use crate::painel::Painel;

pub struct PunhoWin {
    pub painel: Painel,
    pub desktop: DesktopWin,
}

impl PunhoWin {
    pub fn novo() -> PunhoWin {
        PunhoWin {
            painel: Painel::novo(),
            desktop: DesktopWin::novo(),
        }
    }
}

impl Default for PunhoWin {
    fn default() -> PunhoWin {
        PunhoWin::novo()
    }
}

impl Punho for PunhoWin {
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
