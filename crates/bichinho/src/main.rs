//! bichinho: o Zeca, papagaio de pixel art que reage ao Claude Code.
//!
//! Subcomandos:
//! - `rodar` (padrão): o daemon.
//! - `avisar <Evento>`: o hook dos plugins (decisão 0041): lê o JSON do hook,
//!   manda só metadados ao pet do 127.0.0.1, nunca imprime e sempre sai 0.
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
mod avisar;
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
uso: bichinho [rodar | avisar <Evento> | saude | versao]

  rodar            o daemon (padrão)
  avisar <Evento>  o hook do plugin: lê o JSON do hook na entrada padrão,
                   manda só metadados ao pet do 127.0.0.1, nunca imprime e
                   sempre sai 0
  saude            healthcheck: sai 0 se o daemon responde /saude
  versao           imprime a versão
";

fn main() -> ExitCode {
    pet_core::registro::iniciar(std::env::var("PET_LOG").ok().as_deref());
    let comando = std::env::args().nth(1);
    match comando.as_deref() {
        // O hook primeiro: não lê nada além do que precisa e nunca imprime.
        Some("avisar") => avisar::rodar(std::env::args().skip(2)),
        None | Some("rodar") => daemon::rodar(),
        Some("saude") => daemon::saude(),
        Some("versao" | "--version" | "-V") => {
            println!("bichinho {}", pet_core::VERSAO);
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
