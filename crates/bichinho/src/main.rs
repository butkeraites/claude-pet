//! bichinho: o Zeca, papagaio de pixel art que reage ao Claude Code.
//!
//! Subcomandos:
//! - `rodar`: o daemon. Só com o nome: sem subcomando, nada roda (decisão
//!   0045). Este binário fica no PATH de todo Claude Code por causa do hook,
//!   e um `bichinho` solto (um Claude Code que ignorasse os `args` do exec
//!   form, ou alguém curioso no terminal) nunca sobe um daemon.
//! - `avisar <Evento>`: o hook dos plugins (decisão 0041): lê o JSON do hook,
//!   manda só metadados ao pet do 127.0.0.1, nunca imprime, não tem log e
//!   sempre sai 0.
//! - `saude`: healthcheck do Docker; sai 0 se o daemon está vivo.
//! - `versao`: imprime a versão e o commit de onde o binário saiu.
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
mod privacidade;
mod sem_janela;
mod vigia;

use std::io::IsTerminal;
use std::process::ExitCode;

/// O commit de onde o binário saiu (`git rev-parse HEAD`, com `-sujo` se a
/// árvore tinha mudanças), posto pelo `bin/pet` no build da imagem
/// (`BICHINHO_FONTE`); "desconhecida" num `cargo build` solto. O `bin/pet
/// instalar-host` só põe no PATH o binário da worktree estável (decisão
/// 0045).
pub const FONTE: &str = match option_env!("BICHINHO_FONTE") {
    Some(fonte) => fonte,
    None => "desconhecida",
};

const AJUDA: &str = "\
uso: bichinho <rodar | avisar <Evento> | saude | versao>

  rodar            o daemon
  avisar <Evento>  o hook do plugin: lê o JSON do hook na entrada padrão,
                   manda só metadados ao pet do 127.0.0.1, nunca imprime e
                   sempre sai 0
  saude            healthcheck: sai 0 se o daemon responde /saude
  versao           imprime a versão e o commit de onde o binário saiu

Sem comando, nada roda (no terminal, esta ajuda).
";

fn main() -> ExitCode {
    let mut argumentos = std::env::args().skip(1);
    let comando = argumentos.next();
    // O hook primeiro, antes do log: não lê nada além do que precisa, nunca
    // imprime e não registra nada, nem com PET_LOG=debug no ambiente.
    if comando.as_deref() == Some("avisar") {
        pet_core::registro::desligar();
        return avisar::rodar(argumentos);
    }
    pet_core::registro::iniciar(std::env::var("PET_LOG").ok().as_deref());
    match comando.as_deref() {
        None => {
            if std::io::stdin().is_terminal() {
                print!("{AJUDA}");
            }
            ExitCode::SUCCESS
        }
        Some("rodar") => daemon::rodar(),
        Some("saude") => daemon::saude(),
        Some("versao" | "--version" | "-V") => {
            println!("bichinho {} (fonte {FONTE})", pet_core::VERSAO);
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
