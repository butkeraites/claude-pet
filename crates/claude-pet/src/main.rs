//! claude-pet: o Zeca, papagaio de pixel art que reage ao Claude Code.
//!
//! Subcomandos:
//! - `rodar` (padrão): o daemon.
//! - `saude`: healthcheck do Docker; sai 0 se o daemon está vivo.
//! - `versao`: imprime a versão.

#[macro_use]
mod registro;

mod ambiente;
mod aprovacao;
mod comando;
mod daemon;
mod descoberta;
mod estado;
mod ingress;
mod laco;
mod personagem;
mod pet;
mod vigia;
mod wl;

use std::process::ExitCode;

const AJUDA: &str = "\
uso: claude-pet [rodar | saude | versao]

  rodar   o daemon (padrão)
  saude   healthcheck: sai 0 se o daemon responde /saude
  versao  imprime a versão
";

fn main() -> ExitCode {
    registro::iniciar(std::env::var("PET_LOG").ok().as_deref());
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
