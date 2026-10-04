//! Monitores vistos pelo Wayland (`wl_output` v4 + `xdg_output`, via SCTK).
//!
//! O nome (`eDP-1`, `HDMI-A-1`) é o mesmo do Hyprland e vai para o
//! `/v1/estado.monitor`. A escala pelo modo (pixels do modo ÷ tamanho
//! lógico) é a reserva quando o `preferred_scale` fracionário não chega.

use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::client::protocol::wl_output::{Transform, WlOutput};

use crate::hyprland;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Monitor {
    pub nome: String,
    /// Descrição (fabricante e modelo, `wl_output` v4), se o compositor disser.
    pub descricao: Option<String>,
    /// Tamanho lógico (`xdg_output`), em pixels lógicos.
    pub logico: (i32, i32),
    /// Canto superior esquerdo no desktop (`xdg_output`), em pixels lógicos.
    pub posicao: Option<(i32, i32)>,
    /// Modo atual, em pixels do monitor, já com a rotação aplicada.
    pub modo: Option<(i32, i32)>,
}

impl Monitor {
    /// Serve de casa para o pet: não é o FALLBACK do Hyprland e tem tamanho.
    pub fn utilizavel(&self) -> bool {
        !hyprland::eh_reserva(&self.nome) && self.logico.0 > 0 && self.logico.1 > 0
    }

    /// Escala pelo modo: largura do modo ÷ largura lógica.
    pub fn escala_pelo_modo(&self) -> Option<f64> {
        let (largura_modo, _) = self.modo?;
        (self.logico.0 > 0 && largura_modo > 0).then(|| largura_modo as f64 / self.logico.0 as f64)
    }
}

/// Rotações de 90/270 graus trocam largura e altura do modo.
fn gira(transform: Transform) -> bool {
    matches!(
        transform,
        Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270
    )
}

pub fn monitor(saidas: &OutputState, saida: &WlOutput) -> Option<Monitor> {
    let info = saidas.info(saida)?;
    let modo = info.modes.iter().find(|m| m.current).map(|m| {
        if gira(info.transform) {
            (m.dimensions.1, m.dimensions.0)
        } else {
            m.dimensions
        }
    });
    Some(Monitor {
        nome: info
            .name
            .clone()
            .unwrap_or_else(|| format!("saida-{}", info.id)),
        descricao: info.description.clone(),
        logico: info.logical_size.unwrap_or((0, 0)),
        posicao: info.logical_position,
        modo,
    })
}

/// Primeiro monitor utilizável (reserva quando o `enter` não chega).
pub fn primeiro_utilizavel(saidas: &OutputState) -> Option<(WlOutput, Monitor)> {
    saidas
        .outputs()
        .filter_map(|saida| monitor(saidas, &saida).map(|m| (saida, m)))
        .find(|(_, m)| m.utilizavel())
}

#[cfg(test)]
mod testes {
    use super::*;

    fn m(nome: &str, logico: (i32, i32), modo: Option<(i32, i32)>) -> Monitor {
        Monitor {
            nome: nome.into(),
            logico,
            modo,
            ..Monitor::default()
        }
    }

    #[test]
    fn escala_pelo_modo_do_notebook_e_do_4k() {
        let edp = m("eDP-1", (1280, 800), Some((1920, 1200)));
        assert_eq!(edp.escala_pelo_modo(), Some(1.5));
        let hdmi = m("HDMI-A-1", (2560, 1440), Some((3840, 2160)));
        assert_eq!(hdmi.escala_pelo_modo(), Some(1.5));
        assert_eq!(m("X", (0, 0), Some((10, 10))).escala_pelo_modo(), None);
        assert_eq!(m("X", (10, 10), None).escala_pelo_modo(), None);
    }

    #[test]
    fn fallback_e_tamanho_zero_nao_servem() {
        assert!(m("eDP-1", (1280, 800), None).utilizavel());
        assert!(!m("FALLBACK", (1920, 1080), None).utilizavel());
        assert!(!m("HDMI-A-1", (0, 0), None).utilizavel());
    }

    #[test]
    fn rotacao_troca_o_modo() {
        assert!(gira(Transform::_90));
        assert!(gira(Transform::Flipped270));
        assert!(!gira(Transform::Normal));
        assert!(!gira(Transform::_180));
    }
}
