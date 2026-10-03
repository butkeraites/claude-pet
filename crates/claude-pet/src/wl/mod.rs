//! Sessão Wayland: uma conexão com o compositor, do handshake ao EOF.
//!
//! Wayland em Rust puro (smithay-client-toolkit 0.21 sem xkbcommon,
//! wayland-client sem libwayland), decisão 0004. A sessão só existe
//! enquanto o compositor responde; qualquer erro de protocolo ou EOF derruba
//! tudo e o laço volta a esperar (decisão 0007). Por isso nada aqui tenta se
//! recuperar da conexão sozinho: quem decide reconectar é o [`crate::laco`].
//! O que a sessão recupera sozinha é a própria camada (`closed` → recria).

pub mod saida;
pub mod superficie;

use std::os::unix::net::UnixStream;
use std::time::Duration;

use smithay_client_toolkit as sctk;
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::LoopHandle;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::client::globals::{GlobalList, registry_queue_init};
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_surface};
use smithay_client_toolkit::reexports::client::{
    Connection, Dispatch, EventQueue, QueueHandle, delegate_noop,
};
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::{
    self, WpFractionalScaleV1,
};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_registry, registry_handlers};

use crate::laco::Laco;
use superficie::{OrigemEscala, Prazo, Superficie};

/// Globais sem os quais não há camada para desenhar.
const OBRIGATORIOS: &[&str] = &[
    "wl_compositor",
    "wl_shm",
    "zwlr_layer_shell_v1",
    "wp_viewporter",
];
/// Globais que melhoram o resultado; a falta deles vira aviso.
const OPCIONAIS: &[&str] = &[
    "wp_fractional_scale_manager_v1",
    "wl_seat",
    "wp_cursor_shape_manager_v1",
];
/// Depois de um `closed`, espera o monitor assentar antes de recriar.
const RECRIAR_APOS_FECHAR: Duration = Duration::from_millis(250);

/// Estado da fila de eventos Wayland.
pub struct Sessao {
    registro: RegistryState,
    saidas: OutputState,
    compositor: CompositorState,
    shm: Shm,
    camadas: LayerShell,
    fracional: Option<WpFractionalScaleManagerV1>,
    viewporter: WpViewporter,
    qh: QueueHandle<Sessao>,
    handle: LoopHandle<'static, Laco>,
    superficie: Option<Superficie>,
    /// O pet deve aparecer (há personagem e ninguém mandou esconder).
    visivel: bool,
}

/// Resultado do handshake: a conexão, a fila e o estado que ela despacha.
pub struct Conexao {
    pub conexao: Connection,
    pub fila: EventQueue<Sessao>,
    pub sessao: Sessao,
}

/// Faz o handshake numa conexão já aberta (a da prova da descoberta).
pub fn conectar(fluxo: UnixStream, handle: LoopHandle<'static, Laco>) -> Result<Conexao, String> {
    let conexao = Connection::from_socket(fluxo).map_err(|e| format!("conexão Wayland: {e}"))?;
    let (globais, fila) =
        registry_queue_init::<Sessao>(&conexao).map_err(|e| format!("registro Wayland: {e}"))?;
    let faltando: Vec<&str> = OBRIGATORIOS
        .iter()
        .copied()
        .filter(|nome| !tem_global(&globais, nome))
        .collect();
    if !faltando.is_empty() {
        return Err(format!("o compositor não oferece {}", faltando.join(", ")));
    }
    for nome in OPCIONAIS {
        if !tem_global(&globais, nome) {
            aviso!("o compositor não oferece {nome}; seguindo sem");
        }
    }
    let qh = fila.handle();
    let falhou = |nome: &str, e: &dyn std::fmt::Display| format!("{nome}: {e}");
    let compositor =
        CompositorState::bind(&globais, &qh).map_err(|e| falhou("wl_compositor", &e))?;
    let shm = Shm::bind(&globais, &qh).map_err(|e| falhou("wl_shm", &e))?;
    let camadas = LayerShell::bind(&globais, &qh).map_err(|e| falhou("layer shell", &e))?;
    let viewporter = globais
        .bind::<WpViewporter, _, _>(&qh, 1..=1, ())
        .map_err(|e| falhou("wp_viewporter", &e))?;
    let fracional = globais
        .bind::<WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ())
        .ok();
    let sessao = Sessao {
        registro: RegistryState::new(&globais),
        saidas: OutputState::new(&globais, &qh),
        compositor,
        shm,
        camadas,
        fracional,
        viewporter,
        qh,
        handle,
        superficie: None,
        visivel: false,
    };
    Ok(Conexao {
        conexao,
        fila,
        sessao,
    })
}

fn tem_global(globais: &GlobalList, interface: &str) -> bool {
    globais
        .contents()
        .with_list(|lista| lista.iter().any(|g| g.interface == interface))
}

/// Resumo de uma saída para log: nome, tamanho lógico e modo atual.
fn descrever_saida(saidas: &OutputState, saida: &wl_output::WlOutput) -> String {
    match saida::monitor(saidas, saida) {
        Some(m) => {
            let modo = m
                .modo
                .map(|(w, h)| format!("{w}x{h}"))
                .unwrap_or_else(|| "?".into());
            format!(
                "{} (lógico {}x{}, modo {modo})",
                m.nome, m.logico.0, m.logico.1
            )
        }
        None => "saída sem informação".into(),
    }
}

impl Sessao {
    /// Liga ou desliga o pet na tela. Ligar cria a camada (no monitor
    /// focado); desligar a destrói.
    pub fn definir_visivel(&mut self, visivel: bool) {
        self.visivel = visivel;
        if visivel && self.superficie.is_none() {
            self.criar_superficie();
        } else if !visivel {
            self.superficie = None;
        }
    }

    fn criar_superficie(&mut self) {
        let superficie = Superficie::criar(
            &self.compositor,
            &self.camadas,
            self.fracional.as_ref(),
            &self.viewporter,
            &self.qh,
        );
        depurar!("camada criada (geração {})", superficie.geracao);
        self.superficie = Some(superficie);
    }

    /// Arma um timer que chama `acao` nesta sessão se a superfície ainda
    /// for a mesma geração.
    fn depois(&self, daqui_a: Duration, geracao: u64, acao: fn(&mut Sessao)) {
        let timer = Timer::from_duration(daqui_a);
        let inserido = self.handle.insert_source(timer, move |_, _, laco| {
            if let Some(sessao) = laco.sessao_mut() {
                let mesma = sessao.superficie.as_ref().map(|s| s.geracao) == Some(geracao);
                if mesma {
                    acao(sessao);
                }
            }
            TimeoutAction::Drop
        });
        if let Err(e) = inserido {
            erro!("não consegui armar um timer da camada: {e}");
        }
    }

    fn prazo_escala(&mut self) {
        self.vencer(Prazo::Escala);
    }

    fn prazo_enter(&mut self) {
        self.vencer(Prazo::Enter);
    }

    fn vencer(&mut self, prazo: Prazo) {
        if let Some(superficie) = self.superficie.as_mut() {
            if superficie.pronta().is_none() {
                depurar!("prazo de reserva vencido: {prazo:?}");
            }
            superficie.venceu(prazo);
        }
        self.tentar_aprontar();
    }

    /// Recalcula tamanho, escala e monitor da camada; avisa quando muda.
    fn tentar_aprontar(&mut self) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        let Some(pronta) = superficie.resolver(&self.saidas) else {
            return;
        };
        let (largura, altura) = pronta.buffer();
        let origem = match pronta.origem {
            OrigemEscala::Fracionaria => "preferred_scale",
            OrigemEscala::Modo => "modo do monitor",
            OrigemEscala::Padrao => "padrão 1.0, sem informação do compositor",
        };
        info!(
            "camada pronta em {}: {}x{} lógicos, escala {} ({origem}), buffer {largura}x{altura}",
            pronta.monitor.as_deref().unwrap_or("monitor desconhecido"),
            pronta.logico.0,
            pronta.logico.1,
            pronta.escala
        );
    }

    fn escala_preferida(&mut self, objeto: &WpFractionalScaleV1, escala_120: u32) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        if superficie.eh_fracional(objeto) {
            depurar!("preferred_scale {escala_120}/120");
            superficie.escala_preferida(escala_120);
            self.tentar_aprontar();
        }
    }
}

impl CompositorHandler for Sessao {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        fator: i32,
    ) {
        // A escala inteira (ceil(1.5) = 2) não serve para pixel art; quem
        // manda é a fracionária.
        depurar!("escala inteira sugerida: {fator} (ignorada)");
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        saida: &wl_output::WlOutput,
    ) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        if superficie.eh(surface) {
            depurar!("enter em {}", descrever_saida(&self.saidas, saida));
            superficie.entrou(saida);
            self.tentar_aprontar();
        }
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for Sessao {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, camada: &LayerSurface) {
        let nossa = self
            .superficie
            .as_ref()
            .is_some_and(|s| &s.camada == camada);
        if !nossa {
            return;
        }
        info!(
            "a camada foi fechada pelo compositor; recriando em {} ms",
            RECRIAR_APOS_FECHAR.as_millis()
        );
        self.superficie = None;
        let timer = Timer::from_duration(RECRIAR_APOS_FECHAR);
        let inserido = self.handle.insert_source(timer, |_, _, laco| {
            if let Some(sessao) = laco.sessao_mut()
                && sessao.visivel
                && sessao.superficie.is_none()
            {
                sessao.criar_superficie();
            }
            TimeoutAction::Drop
        });
        if let Err(e) = inserido {
            erro!("não consegui agendar a recriação da camada: {e}");
        }
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        camada: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        if &superficie.camada != camada {
            return;
        }
        let (largura, altura) = configure.new_size;
        depurar!("configure {largura}x{altura}");
        match superficie.configurar(largura, altura, &self.compositor, &self.shm) {
            Ok(true) => {
                let geracao = superficie.geracao;
                self.depois(superficie::PRAZO_ESCALA, geracao, Sessao::prazo_escala);
                self.depois(superficie::PRAZO_ENTER, geracao, Sessao::prazo_enter);
            }
            Ok(false) => self.tentar_aprontar(),
            Err(motivo) => aviso!("configure da camada: {motivo}"),
        }
    }
}

impl ShmHandler for Sessao {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl OutputHandler for Sessao {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.saidas
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, saida: wl_output::WlOutput) {
        info!("monitor: {}", descrever_saida(&self.saidas, &saida));
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, saida: wl_output::WlOutput) {
        depurar!("monitor mudou: {}", descrever_saida(&self.saidas, &saida));
    }

    fn output_destroyed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        saida: wl_output::WlOutput,
    ) {
        info!("monitor saiu: {}", descrever_saida(&self.saidas, &saida));
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for Sessao {
    fn event(
        sessao: &mut Self,
        objeto: &WpFractionalScaleV1,
        evento: wp_fractional_scale_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = evento {
            sessao.escala_preferida(objeto, scale);
        }
    }
}

// Sem eventos: o gerente da escala fracionária, o viewporter e o viewport.
delegate_noop!(Sessao: WpFractionalScaleManagerV1);
delegate_noop!(Sessao: WpViewporter);
delegate_noop!(Sessao: WpViewport);

impl ProvidesRegistryState for Sessao {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registro
    }
    registry_handlers![OutputState];
}

delegate_registry!(Sessao);
sctk::delegate_dispatch2!(Sessao);
