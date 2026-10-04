//! `cargo xtask globais [nome …]`: lista os globais que o compositor anuncia
//! no registro do Wayland (interface e versão), e diz se os pedidos estão
//! lá. Serve para conferir, nesta máquina, que o Hyprland oferece o
//! `zwlr_foreign_toplevel_manager_v1` e o `hyprland_toplevel_mapping_manager_v1`
//! (decisão 0056) antes de confiar no clique que foca o terminal.
//!
//! Só fala Wayland (o `WAYLAND_DISPLAY` do ambiente) e só lê o registro: não
//! liga nada, não cria superfície e não toca no IPC do Hyprland.

use smithay_client_toolkit::reexports::client::globals::{GlobalListContents, registry_queue_init};
use smithay_client_toolkit::reexports::client::protocol::wl_registry::{self, WlRegistry};
use smithay_client_toolkit::reexports::client::{Connection, Dispatch, QueueHandle};

/// Os protocolos que o clique do M4 pede, se nenhum nome for dado.
const PADRAO: [&str; 2] = [
    "zwlr_foreign_toplevel_manager_v1",
    "hyprland_toplevel_mapping_manager_v1",
];

struct Registro;

impl Dispatch<WlRegistry, GlobalListContents> for Registro {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// `true` se todos os pedidos estão no registro.
pub fn executar(args: &[String]) -> Result<bool, String> {
    let pedidos: Vec<String> = if args.is_empty() {
        PADRAO.iter().map(|s| s.to_string()).collect()
    } else {
        args.to_vec()
    };
    let conexao =
        Connection::connect_to_env().map_err(|e| format!("sem compositor Wayland: {e}"))?;
    let (globais, _fila) = registry_queue_init::<Registro>(&conexao)
        .map_err(|e| format!("registro do Wayland: {e}"))?;
    let mut lista: Vec<(String, u32)> = globais
        .contents()
        .with_list(|l| l.iter().map(|g| (g.interface.clone(), g.version)).collect());
    lista.sort();
    for (interface, versao) in &lista {
        println!("{interface} v{versao}");
    }
    println!();
    let mut todos = true;
    for pedido in &pedidos {
        match lista.iter().find(|(i, _)| i == pedido) {
            Some((_, versao)) => println!("✓ {pedido} v{versao}"),
            None => {
                println!("✗ {pedido} não está no registro");
                todos = false;
            }
        }
    }
    Ok(todos)
}
