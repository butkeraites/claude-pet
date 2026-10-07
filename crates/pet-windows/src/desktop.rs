//! O [`Desktop`] do Windows. Primeiro corte: anuncia que está ligado (o Motor
//! acende o pet) e nada mais — o monitor ativo é seguido pela própria janela
//! ([`crate::painel::Painel::seguir_monitor`], como no macOS), e o foco/clique
//! na janela do terminal (`GetForegroundWindow`/`SetForegroundWindow`, o anel
//! de ativações) entra num corte seguinte. Nenhum título de janela passa por
//! aqui.

use pet_core::plataforma::{Alca, CapDesktop, Desktop, ErroFoco, EventoDesktop, InfoDesktop};

#[derive(Default)]
pub struct DesktopWin {
    eventos: Vec<EventoDesktop>,
    iniciado: bool,
}

impl DesktopWin {
    pub fn novo() -> DesktopWin {
        DesktopWin::default()
    }

    /// O laço chama a cada volta (simetria com o macOS). A primeira volta liga
    /// a fonte; o resto (janela ativa) entra depois.
    pub fn pollar(&mut self) {
        if !self.iniciado {
            self.iniciado = true;
            self.eventos.push(EventoDesktop::Ligado(true));
        }
    }
}

impl Desktop for DesktopWin {
    fn capacidades(&self) -> CapDesktop {
        CapDesktop {
            // A janela segue o monitor sozinha (como o macOS).
            segue_foco: false,
            // O rastreio da janela ativa e o foco entram num corte seguinte.
            janela_ativa: false,
            foca_janela: false,
            nao_perturbe: false,
        }
    }

    fn focar(&mut self, _alvo: &Alca) -> Result<(), ErroFoco> {
        Err(ErroFoco::NaoSuportado)
    }

    fn eventos(&mut self) -> Vec<EventoDesktop> {
        std::mem::take(&mut self.eventos)
    }

    fn janela_ativa(&self) -> Option<Alca> {
        None
    }

    fn info(&self) -> InfoDesktop {
        InfoDesktop {
            protocolos: vec!["Win32".to_owned()],
            janelas: 0,
        }
    }
}
