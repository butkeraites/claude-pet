//! Ferramentas de desenvolvimento: `cargo xtask <comando>`.
//!
//! Os comandos chegam junto com os marcos que precisam deles (PLANO.md):
//! `skin-teste` e `nitidez` no M1; `skin-importar`, `zeca`, `lint-skin`,
//! `cobertura` e `contato` no M2; `fonte` (a monogram dos balões) e
//! `globais` (os protocolos que o compositor oferece) no M4; `zeca-livre` (o
//! Zeca original, CC0, do gerador em `arte/zeca-livre/`) na skin livre (TS.1).

mod args;
// `carga` e `globais` são clientes Wayland do host (só Linux); fora do Linux o
// crate nem puxa o smithay (decisão 0040).
#[cfg(target_os = "linux")]
mod carga;
mod cobertura;
mod contato;
mod fantasma;
mod folha;
mod fonte;
mod fonte_mini;
#[cfg(target_os = "linux")]
mod globais;
mod importar;
mod lint;
mod nitidez;
mod skin_teste;
mod zeca;
mod zeca_livre;
mod zip;

use std::path::PathBuf;
use std::process::ExitCode;

const COMANDOS: &[(&str, &str)] = &[
    (
        "skin-teste",
        "gera a skin xadrez de QA em skins/_teste (M1)",
    ),
    ("nitidez", "confere blocos D×D numa captura do grim (M1)"),
    (
        "fantasma",
        "confere que o pet não deixa pixels velhos na tela (M1)",
    ),
    ("carga", "repintura invisível da tela para medir custo (M1)"),
    (
        "skin-importar",
        "importa um pack (.aseprite ou tiras PNG) (M2)",
    ),
    ("zeca", "recolore o pack e encaixa chapéu e gravata (M2)"),
    (
        "zeca-livre",
        "o Zeca original (CC0) do gerador arte/zeca-livre/zeca.py; --conferir só confere",
    ),
    ("lint-skin", "valida uma skin (M2)"),
    (
        "cobertura",
        "lista estados nativos, por receita e faltando (M2)",
    ),
    ("contato", "folha de contato e GIFs de prévia (M2)"),
    (
        "fonte",
        "assa a fonte monogram dos balões no pet-core (M4); --conferir só confere",
    ),
    (
        "globais",
        "lista os globais do compositor e confere os que o clique pede (M4)",
    ),
];

/// Raiz do repositório (o xtask mora em `<raiz>/xtask`).
fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// `cargo xtask carga`: só no Linux (cliente Wayland do host).
#[cfg(target_os = "linux")]
fn cmd_carga(args: &[String]) -> ExitCode {
    match carga::executar(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("carga: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn cmd_carga(_args: &[String]) -> ExitCode {
    eprintln!("carga: só no Linux (precisa de um compositor Wayland)");
    ExitCode::FAILURE
}

/// `cargo xtask globais`: só no Linux (lê o registro do compositor Wayland).
#[cfg(target_os = "linux")]
fn cmd_globais(args: &[String]) -> ExitCode {
    match globais::executar(args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("globais: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn cmd_globais(_args: &[String]) -> ExitCode {
    eprintln!("globais: só no Linux (precisa de um compositor Wayland)");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("ajuda" | "--help" | "-h") => {
            println!("uso: cargo xtask <comando>\n");
            for (nome, descricao) in COMANDOS {
                println!("  {nome:<14} {descricao}");
            }
            ExitCode::SUCCESS
        }
        Some("skin-teste") => {
            let pasta = raiz().join("skins/_teste");
            match skin_teste::executar(&pasta) {
                Ok(skin) => {
                    println!(
                        "skin «{}» gerada em {}: {} quadros, {} tags, {} estados",
                        skin.id,
                        pasta.display(),
                        skin.quadros.len(),
                        skin.tags.len(),
                        skin.estados.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("skin-teste: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("nitidez") => match nitidez::executar(&args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("nitidez: {e}");
                eprintln!(
                    "uso: cargo xtask nitidez --captura <png> --esperado <png> --x X --y Y --d D [--grade X,Y] [--tolerancia 2]"
                );
                ExitCode::from(2)
            }
        },
        Some("fantasma") => match fantasma::executar(&args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("fantasma: {e}");
                eprintln!(
                    "uso: cargo xtask fantasma --base <png> [--base <png>…] --depois <png> [--esperado <png>] [--tolerancia 2]"
                );
                ExitCode::from(2)
            }
        },
        Some("carga") => cmd_carga(&args[1..]),
        Some("zeca") => match zeca::executar(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("zeca: {e}");
                eprintln!("{}", zeca::USO);
                ExitCode::FAILURE
            }
        },
        Some("zeca-livre") => match zeca_livre::executar(&args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("zeca-livre: {e}");
                eprintln!("{}", zeca_livre::USO);
                ExitCode::FAILURE
            }
        },
        Some("lint-skin") => match lint::executar(&args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("lint-skin: {e}");
                eprintln!("uso: cargo xtask lint-skin [--so-erros] <pasta> [<pasta>…]");
                ExitCode::from(2)
            }
        },
        Some("contato") => match contato::executar(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("contato: {e}");
                eprintln!(
                    "uso: cargo xtask contato <pasta> [--saida <pasta>] [--escala 4] [--copia <pasta>]"
                );
                ExitCode::FAILURE
            }
        },
        Some("cobertura") => match cobertura::executar(&args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("cobertura: {e}");
                eprintln!("uso: cargo xtask cobertura <pasta> [--saida <arquivo.md>]");
                ExitCode::from(2)
            }
        },
        Some("globais") => cmd_globais(&args[1..]),
        Some("fonte") => match fonte::executar(&raiz(), &args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("fonte: {e}");
                ExitCode::FAILURE
            }
        },
        Some("skin-importar") => match importar::executar(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("skin-importar: {e}");
                eprintln!("{}", importar::USO);
                ExitCode::FAILURE
            }
        },
        Some(nome) if COMANDOS.iter().any(|(n, _)| *n == nome) => {
            eprintln!("cargo xtask {nome}: ainda não implementado (veja PLANO.md)");
            ExitCode::FAILURE
        }
        Some(outro) => {
            eprintln!("comando desconhecido: {outro} (cargo xtask ajuda)");
            ExitCode::from(2)
        }
    }
}
