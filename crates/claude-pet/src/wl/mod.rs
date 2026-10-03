//! Sessão Wayland: uma conexão com o compositor, do handshake ao EOF.
//!
//! Wayland em Rust puro (smithay-client-toolkit 0.21 sem xkbcommon,
//! wayland-client sem libwayland), decisão 0004. A sessão só existe
//! enquanto o compositor responde; qualquer erro de protocolo ou EOF derruba
//! tudo e o laço volta a esperar (decisão 0007). Por isso nada aqui tenta se
//! recuperar sozinho: quem decide reconectar é o [`crate::laco`].

use std::os::unix::net::UnixStream;

use smithay_client_toolkit as sctk;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::globals::{GlobalList, registry_queue_init};
use smithay_client_toolkit::reexports::client::protocol::wl_output;
use smithay_client_toolkit::reexports::client::{Connection, EventQueue, QueueHandle};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::{delegate_registry, registry_handlers};

/// Globais sem os quais não há camada para desenhar.
const OBRIGATORIOS: &[&str] = &["wl_compositor", "wl_shm", "zwlr_layer_shell_v1"];
/// Globais que melhoram o resultado; a falta deles vira aviso.
const OPCIONAIS: &[&str] = &[
    "wp_fractional_scale_manager_v1",
    "wp_viewporter",
    "wl_seat",
    "wp_cursor_shape_manager_v1",
];

/// Estado da fila de eventos Wayland.
pub struct Sessao {
    registro: RegistryState,
    saidas: OutputState,
}

/// Resultado do handshake: a conexão, a fila e o estado que ela despacha.
pub struct Conexao {
    pub conexao: Connection,
    pub fila: EventQueue<Sessao>,
    pub sessao: Sessao,
}

/// Faz o handshake numa conexão já aberta (a da prova da descoberta).
pub fn conectar(fluxo: UnixStream) -> Result<Conexao, String> {
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
    let sessao = Sessao {
        registro: RegistryState::new(&globais),
        saidas: OutputState::new(&globais, &qh),
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
    let Some(info) = saidas.info(saida) else {
        return "saída sem informação".into();
    };
    let nome = info.name.as_deref().unwrap_or("?");
    let logico = info
        .logical_size
        .map(|(w, h)| format!("{w}x{h}"))
        .unwrap_or_else(|| "?".into());
    let modo = info
        .modes
        .iter()
        .find(|m| m.current)
        .map(|m| format!("{}x{}", m.dimensions.0, m.dimensions.1))
        .unwrap_or_else(|| "?".into());
    format!("{nome} (lógico {logico}, modo {modo})")
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

impl ProvidesRegistryState for Sessao {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registro
    }
    registry_handlers![OutputState];
}

delegate_registry!(Sessao);
sctk::delegate_dispatch2!(Sessao);
