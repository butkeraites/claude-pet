//! claude-pet: o Zeca, papagaio de pixel art que reage ao Claude Code.
//!
//! Subcomandos:
//! - `rodar` (padrão): o daemon.
//! - `saude`: healthcheck do Docker; sai 0 se o daemon está vivo.
//! - `versao`: imprime a versão.
//!
//! O que é igual em todo sistema mora aqui e no `pet-core` (o Motor); o laço
//! de cada sistema fica atrás de `cfg` (decisão 0040): no Linux, o calloop
//! com o Wayland ([`laco`], `pet-wayland`).

#[macro_use]
extern crate pet_core;

mod ambiente;
mod aprovacao;
mod comando;
mod daemon;
mod estado;
mod ingress;
#[cfg(target_os = "linux")]
mod laco;
mod nucleo;
mod personagem;
mod sem_janela;
mod vigia;

use std::process::ExitCode;

const AJUDA: &str = "\
uso: claude-pet [rodar | saude | versao]

  rodar   o daemon (padrão)
  saude   healthcheck: sai 0 se o daemon responde /saude
  versao  imprime a versão
";

fn main() -> ExitCode {
    pet_core::registro::iniciar(std::env::var("PET_LOG").ok().as_deref());
    let comando = std::env::args().nth(1);
    match comando.as_deref() {
        None | Some("rodar") => daemon::rodar(),
        Some("saude") => daemon::saude(),
        Some("versao" | "--version" | "-V") => {
            println!("claude-pet {}", pet_core::VERSAO);
            ExitCode::SUCCESS
        }
        Some("ajuda" | "--help" | "-h") => {
            print!("{AJUDA}");
            ExitCode::SUCCESS
        }
        Some(outro) => {
            eprint!("comando desconhecido: {outro}\n\n{AJUDA}");
            ExitCode::from(2)
        }
    }
}
