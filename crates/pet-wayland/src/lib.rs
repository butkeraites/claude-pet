//! Backend Wayland do pet (decisões 0004, 0005 e 0040).
//!
//! Wayland em Rust puro (smithay-client-toolkit 0.21 sem xkbcommon,
//! wayland-client sem libwayland): o binário estático musl continua.
//!
//! - [`sessao`]: a conexão com o compositor e a camada OVERLAY do tamanho do
//!   monitor, que implementa o [`pet_core::plataforma::Overlay`];
//! - [`superficie`], [`shm`], [`saida`] e [`sincronia`]: a camada, os
//!   buffers, os monitores e a ida e volta com prazo;
//! - [`conexao`]: o que serve a qualquer compositor (socket por caminho
//!   longo, backoff de reconexão);
//! - [`toplevel`]: o foreign-toplevel genérico (as janelas que o compositor
//!   anuncia, para focar a de uma sessão; decisão 0056);
//! - [`hyprland`]: o adaptador do Hyprland (achar a instância pelo
//!   `hyprland.lock`, o monitor FALLBACK, o leitor do socket de eventos e o
//!   mapeamento dos toplevels para os endereços das janelas). O socket de
//!   comandos nunca é aberto (decisão 0006).
//!
//! Nada daqui sabe o que o pet faz: quem decide é o
//! [`pet_core::motor::Motor`], pelo laço do daemon.

#![cfg(target_os = "linux")]

#[macro_use]
extern crate pet_core;

pub mod conexao;
pub mod hyprland;
pub mod saida;
pub mod sessao;
pub mod shm;
pub mod sincronia;
pub mod superficie;
pub mod toplevel;

pub use sessao::{Conexao, Sessao, conectar};

/// Apoio dos testes: pastas temporárias apagadas no fim.
#[cfg(test)]
pub(crate) mod apoio {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct Temp(pub PathBuf);

    impl Temp {
        pub fn nova(nome: &str) -> Temp {
            static CONTADOR: AtomicUsize = AtomicUsize::new(0);
            let caminho = std::env::temp_dir().join(format!(
                "pet-wayland-{}-{}-{nome}",
                std::process::id(),
                CONTADOR.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&caminho);
            fs::create_dir_all(&caminho).unwrap();
            Temp(caminho)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
