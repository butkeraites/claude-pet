//! A camada OVERLAY do pet (decisões 0004 e 0005).
//!
//! Uma superfície só, ancorada nas quatro bordas (o compositor dá o tamanho
//! do monitor no `configure`), `exclusive_zone -1`, teclado NONE e criada
//! com output **NULL**: o Hyprland a põe no monitor focado. Nunca é
//! redimensionada nem ganha subsurfaces; o pet anda dentro do buffer.
//!
//! Com output NULL o Hyprland só manda escala e `enter` quando a camada é
//! mapeada. Por isso o primeiro mapeamento é um buffer 1x1 transparente
//! esticado pelo viewport até o tamanho lógico, com região de input vazia:
//! nada aparece e nenhum clique é roubado. Só depois de `enter` +
//! `preferred_scale` (ou dos prazos de reserva) a superfície fica pronta
//! para desenhar em pixels do monitor.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use smithay_client_toolkit::compositor::{CompositorState, Region};
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::client::QueueHandle;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::client::protocol::wl_shm;
use smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface;
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface,
};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};

use super::Sessao;
use super::saida;

/// Namespace da camada (decisão 0012); é por ele que o `hyprctl layers` e
/// uma eventual regra do Hyprland acham o pet.
pub const NAMESPACE: &str = "claude-pet";
/// Sem `preferred_scale` depois disto: escala pelo modo do monitor.
pub const PRAZO_ESCALA: Duration = Duration::from_millis(200);
/// Sem `enter` depois disto: primeiro monitor utilizável.
pub const PRAZO_ENTER: Duration = Duration::from_millis(500);

/// Cada superfície ganha um número; timers velhos comparam e se calam.
static GERACOES: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrigemEscala {
    /// `wp_fractional_scale_v1.preferred_scale` (o normal no Hyprland).
    Fracionaria,
    /// Modo do monitor ÷ tamanho lógico (reserva).
    Modo,
    /// Nada disponível: 1.0, com aviso.
    Padrao,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prazo {
    Escala,
    Enter,
}

/// Tudo o que é preciso para desenhar em pixels do monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct Pronta {
    /// Tamanho lógico da superfície (o do monitor).
    pub logico: (u32, u32),
    pub escala: f64,
    pub origem: OrigemEscala,
    pub monitor: Option<String>,
}

impl Pronta {
    /// Tamanho do buffer em pixels do monitor: `round(w*s) x round(h*s)`.
    pub fn buffer(&self) -> (i32, i32) {
        (
            (self.logico.0 as f64 * self.escala).round() as i32,
            (self.logico.1 as f64 * self.escala).round() as i32,
        )
    }
}

/// Os sinais que decidem se a camada já pode desenhar. Puro, para testar
/// os prazos de reserva sem compositor.
#[derive(Debug, Clone, Default)]
pub struct Prontidao {
    /// Tamanho lógico do último `configure`.
    pub logico: Option<(u32, u32)>,
    /// O buffer 1x1 já foi para a tela.
    pub mapeada: bool,
    /// `preferred_scale` × 120.
    pub escala_120: Option<u32>,
    pub prazo_escala: bool,
    pub prazo_enter: bool,
}

impl Prontidao {
    /// `do_enter`: o monitor do `enter`, se veio. `reserva`: o primeiro
    /// monitor utilizável, usado só depois do prazo do `enter`.
    pub fn decidir(
        &self,
        entrou: bool,
        do_enter: Option<&saida::Monitor>,
        reserva: Option<&saida::Monitor>,
    ) -> Option<Pronta> {
        let logico = self.logico?;
        if !self.mapeada {
            return None;
        }
        let monitor = match (entrou, self.prazo_enter) {
            (true, _) => do_enter,
            (false, true) => reserva,
            (false, false) => return None,
        };
        let (escala, origem) = match self.escala_120 {
            Some(v) if v > 0 => (v as f64 / 120.0, OrigemEscala::Fracionaria),
            _ if !(self.prazo_escala || self.prazo_enter) => return None,
            _ => match monitor.and_then(saida::Monitor::escala_pelo_modo) {
                Some(escala) => (escala, OrigemEscala::Modo),
                None if self.prazo_enter => (1.0, OrigemEscala::Padrao),
                None => return None,
            },
        };
        Some(Pronta {
            logico,
            escala,
            origem,
            monitor: monitor.map(|m| m.nome.clone()),
        })
    }
}

pub struct Superficie {
    pub camada: LayerSurface,
    viewport: WpViewport,
    fracional: Option<WpFractionalScaleV1>,
    pub geracao: u64,
    prontidao: Prontidao,
    /// Monitor do primeiro `enter`.
    saida: Option<WlOutput>,
    /// Pool e buffer do mapeamento 1x1 (vivos até o primeiro quadro de
    /// verdade, porque o compositor pode ainda ler o buffer).
    inicial: Option<(SlotPool, Buffer)>,
    pronta: Option<Pronta>,
}

impl Superficie {
    pub fn criar(
        compositor: &CompositorState,
        camadas: &LayerShell,
        fracional: Option<&WpFractionalScaleManagerV1>,
        viewporter: &WpViewporter,
        qh: &QueueHandle<Sessao>,
    ) -> Superficie {
        let wl_surface = compositor.create_surface(qh);
        let fracional = fracional.map(|f| f.get_fractional_scale(&wl_surface, qh, ()));
        let viewport = viewporter.get_viewport(&wl_surface, qh, ());
        let camada =
            camadas.create_layer_surface(qh, wl_surface, Layer::Overlay, Some(NAMESPACE), None);
        camada.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        camada.set_size(0, 0);
        camada.set_exclusive_zone(-1);
        camada.set_keyboard_interactivity(KeyboardInteractivity::None);
        // Primeiro commit sem buffer: pede o configure.
        camada.commit();
        Superficie {
            camada,
            viewport,
            fracional,
            geracao: GERACOES.fetch_add(1, Ordering::Relaxed),
            prontidao: Prontidao::default(),
            saida: None,
            inicial: None,
            pronta: None,
        }
    }

    pub fn wl_surface(&self) -> &WlSurface {
        self.camada.wl_surface()
    }

    pub fn eh(&self, surface: &WlSurface) -> bool {
        self.wl_surface() == surface
    }

    pub fn eh_fracional(&self, objeto: &WpFractionalScaleV1) -> bool {
        self.fracional.as_ref() == Some(objeto)
    }

    pub fn pronta(&self) -> Option<&Pronta> {
        self.pronta.as_ref()
    }

    /// Aplica um `configure`. No primeiro, mapeia com o buffer 1x1
    /// transparente e a região de input vazia. Devolve `true` no primeiro
    /// mapeamento (hora de armar os prazos de reserva).
    pub fn configurar(
        &mut self,
        largura: u32,
        altura: u32,
        compositor: &CompositorState,
        shm: &Shm,
    ) -> Result<bool, String> {
        if largura == 0 || altura == 0 {
            return Err(format!("configure sem tamanho ({largura}x{altura})"));
        }
        self.prontidao.logico = Some((largura, altura));
        self.viewport.set_destination(largura as i32, altura as i32);
        if self.prontidao.mapeada {
            return Ok(false);
        }
        let mut pool = SlotPool::new(64, shm).map_err(|e| format!("pool do 1x1: {e}"))?;
        let (buffer, tela) = pool
            .create_buffer(1, 1, 4, wl_shm::Format::Argb8888)
            .map_err(|e| format!("buffer 1x1: {e}"))?;
        tela.fill(0);
        let superficie = self.camada.wl_surface();
        buffer
            .attach_to(superficie)
            .map_err(|e| format!("anexar o 1x1: {e}"))?;
        superficie.damage_buffer(0, 0, 1, 1);
        let vazia = Region::new(compositor).map_err(|e| format!("região de input: {e}"))?;
        superficie.set_input_region(Some(vazia.wl_region()));
        self.camada.commit();
        self.inicial = Some((pool, buffer));
        self.prontidao.mapeada = true;
        Ok(true)
    }

    pub fn escala_preferida(&mut self, escala_120: u32) {
        self.prontidao.escala_120 = Some(escala_120);
    }

    /// Guarda o monitor do primeiro `enter`.
    pub fn entrou(&mut self, saida: &WlOutput) {
        if self.saida.is_none() {
            self.saida = Some(saida.clone());
        }
    }

    pub fn venceu(&mut self, prazo: Prazo) {
        match prazo {
            Prazo::Escala => self.prontidao.prazo_escala = true,
            Prazo::Enter => self.prontidao.prazo_enter = true,
        }
    }

    /// Junta tamanho, escala e monitor. Devolve `Some` quando o resultado
    /// mudou (inclusive na primeira vez que fica pronta).
    pub fn resolver(&mut self, saidas: &OutputState) -> Option<Pronta> {
        let do_enter = self.saida.as_ref().and_then(|s| saida::monitor(saidas, s));
        let reserva = saida::primeiro_utilizavel(saidas).map(|(_, m)| m);
        let nova =
            self.prontidao
                .decidir(self.saida.is_some(), do_enter.as_ref(), reserva.as_ref())?;
        if self.pronta.as_ref() == Some(&nova) {
            return None;
        }
        self.pronta = Some(nova.clone());
        Some(nova)
    }
}

impl Drop for Superficie {
    fn drop(&mut self) {
        // O viewport e a escala fracionária saem antes da superfície; o
        // `LayerSurface` destrói o papel e depois o `wl_surface`.
        if let Some(fracional) = self.fracional.take() {
            fracional.destroy();
        }
        self.viewport.destroy();
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::wl::saida::Monitor;

    fn edp() -> Monitor {
        Monitor {
            nome: "eDP-1".into(),
            logico: (1280, 800),
            modo: Some((1920, 1200)),
        }
    }

    fn mapeada() -> Prontidao {
        Prontidao {
            logico: Some((1280, 800)),
            mapeada: true,
            ..Prontidao::default()
        }
    }

    #[test]
    fn caminho_normal_enter_e_preferred_scale() {
        let mut p = mapeada();
        assert_eq!(p.decidir(true, Some(&edp()), None), None, "falta a escala");
        p.escala_120 = Some(180);
        let pronta = p.decidir(true, Some(&edp()), None).unwrap();
        assert_eq!(pronta.escala, 1.5);
        assert_eq!(pronta.origem, OrigemEscala::Fracionaria);
        assert_eq!(pronta.monitor.as_deref(), Some("eDP-1"));
        assert_eq!(pronta.buffer(), (1920, 1200));
    }

    #[test]
    fn sem_configure_ou_sem_mapear_nao_fica_pronta() {
        let mut p = Prontidao {
            escala_120: Some(180),
            ..Prontidao::default()
        };
        assert_eq!(p.decidir(true, Some(&edp()), None), None);
        p.logico = Some((1280, 800));
        assert_eq!(
            p.decidir(true, Some(&edp()), None),
            None,
            "1x1 ainda não foi"
        );
    }

    #[test]
    fn sem_preferred_scale_usa_o_modo_depois_de_200_ms() {
        let mut p = mapeada();
        assert_eq!(p.decidir(true, Some(&edp()), None), None);
        p.prazo_escala = true;
        let pronta = p.decidir(true, Some(&edp()), None).unwrap();
        assert_eq!(pronta.escala, 1.5);
        assert_eq!(pronta.origem, OrigemEscala::Modo);
    }

    #[test]
    fn sem_enter_usa_o_primeiro_monitor_depois_de_500_ms() {
        let mut p = mapeada();
        p.escala_120 = Some(180);
        assert_eq!(p.decidir(false, None, Some(&edp())), None, "espera o enter");
        p.prazo_enter = true;
        let pronta = p.decidir(false, None, Some(&edp())).unwrap();
        assert_eq!(pronta.monitor.as_deref(), Some("eDP-1"));
        assert_eq!(pronta.origem, OrigemEscala::Fracionaria);
    }

    #[test]
    fn sem_nada_cai_em_1_com_aviso() {
        let mut p = mapeada();
        p.prazo_escala = true;
        p.prazo_enter = true;
        let pronta = p.decidir(false, None, None).unwrap();
        assert_eq!(pronta.escala, 1.0);
        assert_eq!(pronta.origem, OrigemEscala::Padrao);
        assert_eq!(pronta.monitor, None);
    }

    #[test]
    fn buffer_arredonda_no_4k_e_em_escala_quebrada() {
        let p = Pronta {
            logico: (2560, 1440),
            escala: 1.5,
            origem: OrigemEscala::Fracionaria,
            monitor: None,
        };
        assert_eq!(p.buffer(), (3840, 2160));
        let q = Pronta {
            logico: (1093, 615),
            escala: 1.75,
            ..p
        };
        assert_eq!(q.buffer(), (1913, 1076));
    }
}
