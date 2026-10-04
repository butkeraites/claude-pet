//! O `hyprland_toplevel_mapping_manager_v1` (decisões 0039 e 0056): liga
//! cada toplevel do `zwlr_foreign_toplevel_manager_v1` ao endereço de janela
//! do Hyprland, o mesmo dos eventos do socket2 (`activewindowv2`). É a
//! extensão do Hyprland ao foreign-toplevel genérico: com ela, o clique sabe
//! qual handle ativar para a janela que o anel de ativações casou com a
//! sessão.
//!
//! O protocolo é do hyprland-protocols (BSD-3-Clause; o XML, com o aviso de
//! copyright, está em `crates/pet-wayland/protocolos/`). O código do cliente
//! sai do `wayland_scanner::generate_client_code!`; as tabelas das
//! interfaces ([`protocolo::__interfaces`]) são escritas à mão, porque o
//! `generate_interfaces!` gera também a ponte para a libwayland em C, com
//! `unsafe` — e o pet usa só o backend em Rust puro (`c_ptr: None`), num
//! crate em que `unsafe` é proibido. Um teste confere as tabelas contra o
//! XML.

/// O código do cliente do protocolo.
#[allow(
    dead_code,
    non_camel_case_types,
    unused_variables,
    non_upper_case_globals,
    non_snake_case,
    unused_imports,
    missing_docs,
    clippy::all
)]
pub mod protocolo {
    use smithay_client_toolkit::reexports::protocols_wlr::foreign_toplevel::v1::client::*;
    use wayland_client;
    use wayland_client::protocol::*;
    use wayland_protocols::ext::foreign_toplevel_list::v1::client::*;

    /// As tabelas das duas interfaces, como o `generate_interfaces!` as faria,
    /// sem a ponte para a libwayland em C.
    pub mod __interfaces {
        use smithay_client_toolkit::reexports::protocols_wlr::foreign_toplevel::v1::client::__interfaces::ZWLR_FOREIGN_TOPLEVEL_HANDLE_V1_INTERFACE;
        use wayland_client::backend::protocol::{AllowNull, ArgumentType, Interface, MessageDesc};
        use wayland_protocols::ext::foreign_toplevel_list::v1::client::__interfaces::EXT_FOREIGN_TOPLEVEL_HANDLE_V1_INTERFACE;

        pub static HYPRLAND_TOPLEVEL_MAPPING_MANAGER_V1_INTERFACE: Interface = Interface {
            name: "hyprland_toplevel_mapping_manager_v1",
            version: 1,
            requests: &[
                MessageDesc {
                    name: "get_window_for_toplevel",
                    signature: &[ArgumentType::NewId, ArgumentType::Object(AllowNull::No)],
                    since: 1,
                    is_destructor: false,
                    child_interface: Some(&HYPRLAND_TOPLEVEL_WINDOW_MAPPING_HANDLE_V1_INTERFACE),
                    arg_interfaces: &[&EXT_FOREIGN_TOPLEVEL_HANDLE_V1_INTERFACE],
                },
                MessageDesc {
                    name: "get_window_for_toplevel_wlr",
                    signature: &[ArgumentType::NewId, ArgumentType::Object(AllowNull::No)],
                    since: 1,
                    is_destructor: false,
                    child_interface: Some(&HYPRLAND_TOPLEVEL_WINDOW_MAPPING_HANDLE_V1_INTERFACE),
                    arg_interfaces: &[&ZWLR_FOREIGN_TOPLEVEL_HANDLE_V1_INTERFACE],
                },
                MessageDesc {
                    name: "destroy",
                    signature: &[],
                    since: 1,
                    is_destructor: true,
                    child_interface: None,
                    arg_interfaces: &[],
                },
            ],
            events: &[],
            c_ptr: None,
        };

        pub static HYPRLAND_TOPLEVEL_WINDOW_MAPPING_HANDLE_V1_INTERFACE: Interface = Interface {
            name: "hyprland_toplevel_window_mapping_handle_v1",
            version: 1,
            requests: &[MessageDesc {
                name: "destroy",
                signature: &[],
                since: 1,
                is_destructor: true,
                child_interface: None,
                arg_interfaces: &[],
            }],
            events: &[
                MessageDesc {
                    name: "window_address",
                    signature: &[ArgumentType::Uint, ArgumentType::Uint],
                    since: 1,
                    is_destructor: false,
                    child_interface: None,
                    arg_interfaces: &[],
                },
                MessageDesc {
                    name: "failed",
                    signature: &[],
                    since: 1,
                    is_destructor: false,
                    child_interface: None,
                    arg_interfaces: &[],
                },
            ],
            c_ptr: None,
        };
    }

    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("protocolos/hyprland-toplevel-mapping-v1.xml");
}

/// O endereço de janela do Hyprland (o dos eventos do socket2: hexadecimal
/// sem `0x`) a partir das duas metades do `window_address`.
pub fn endereco(alto: u32, baixo: u32) -> String {
    format!("{:x}", (u64::from(alto) << 32) | u64::from(baixo))
}

#[cfg(test)]
mod testes {
    use wayland_client::backend::protocol::{ArgumentType, Interface};

    use super::protocolo::__interfaces::*;
    use super::*;

    #[test]
    fn endereco_igual_ao_do_socket2() {
        assert_eq!(endereco(0x5bbf, 0x4e6128f0), "5bbf4e6128f0");
        assert_eq!(endereco(0, 0xabc), "abc");
        assert_eq!(endereco(0x1, 0), "100000000");
    }

    /// Os pedidos e eventos de uma interface no XML, em ordem: (nome, tipos
    /// dos argumentos, destrutor).
    fn do_xml(xml: &str, interface: &str, tipo: &str) -> Vec<(String, Vec<String>, bool)> {
        let inicio = xml
            .find(&format!("<interface name=\"{interface}\""))
            .expect("a interface no XML");
        let fim = xml[inicio..].find("</interface>").unwrap() + inicio;
        let trecho = &xml[inicio..fim];
        let mut saida = Vec::new();
        let marca = format!("<{tipo} name=\"");
        let mut resto = trecho;
        while let Some(i) = resto.find(&marca) {
            resto = &resto[i + marca.len()..];
            let nome = resto[..resto.find('"').unwrap()].to_owned();
            let cabeca = &resto[..resto.find('>').unwrap()];
            let destrutor = cabeca.contains("type=\"destructor\"");
            let corpo = &resto[..resto.find(&format!("</{tipo}>")).unwrap_or(resto.len())];
            let mut args = Vec::new();
            let mut a = corpo;
            while let Some(j) = a.find("type=\"") {
                a = &a[j + 6..];
                args.push(a[..a.find('"').unwrap()].to_owned());
            }
            // O type="destructor" do próprio pedido não é argumento.
            if destrutor {
                args.retain(|t| t != "destructor");
            }
            saida.push((nome, args, destrutor));
        }
        saida
    }

    fn tipo(t: &ArgumentType) -> &'static str {
        match t {
            ArgumentType::NewId => "new_id",
            ArgumentType::Object(_) => "object",
            ArgumentType::Uint => "uint",
            ArgumentType::Int => "int",
            ArgumentType::Str(_) => "string",
            ArgumentType::Fixed => "fixed",
            ArgumentType::Array => "array",
            ArgumentType::Fd => "fd",
        }
    }

    fn conferir(xml: &str, interface: &Interface) {
        for (tipo_xml, mensagens) in [("request", interface.requests), ("event", interface.events)]
        {
            let esperado = do_xml(xml, interface.name, tipo_xml);
            let tabela: Vec<(String, Vec<String>, bool)> = mensagens
                .iter()
                .map(|m| {
                    (
                        m.name.to_owned(),
                        m.signature.iter().map(|t| tipo(t).to_owned()).collect(),
                        m.is_destructor,
                    )
                })
                .collect();
            assert_eq!(tabela, esperado, "{} ({tipo_xml}s)", interface.name);
        }
    }

    #[test]
    fn as_tabelas_escritas_a_mao_sao_as_do_xml() {
        let xml = include_str!("../../protocolos/hyprland-toplevel-mapping-v1.xml");
        conferir(xml, &HYPRLAND_TOPLEVEL_MAPPING_MANAGER_V1_INTERFACE);
        conferir(xml, &HYPRLAND_TOPLEVEL_WINDOW_MAPPING_HANDLE_V1_INTERFACE);
        assert!(
            xml.contains("Copyright © 2025 WhySoBad"),
            "o aviso de copyright"
        );
    }
}
