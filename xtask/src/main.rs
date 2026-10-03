//! Ferramentas de desenvolvimento: `cargo xtask <comando>`.
//!
//! Os comandos chegam junto com os marcos que precisam deles (PLANO.md):
//! `skin-teste` e `nitidez` no M1; `skin-importar`, `zeca`, `lint-skin`,
//! `cobertura`, `contato` e `fonte` no M2.

mod nitidez;
mod skin_teste;

use std::path::PathBuf;
use std::process::ExitCode;

const COMANDOS: &[(&str, &str)] = &[
    (
        "skin-teste",
        "gera a skin xadrez de QA em skins/_teste (M1)",
    ),
    ("nitidez", "confere blocos D×D numa captura do grim (M1)"),
    (
        "skin-importar",
        "importa um pack (.aseprite ou tiras PNG) (M2)",
    ),
    ("zeca", "recolore o pack e encaixa chapéu e gravata (M2)"),
    ("lint-skin", "valida uma skin (M2)"),
    (
        "cobertura",
        "lista estados nativos, por receita e faltando (M2)",
    ),
    ("contato", "folha de contato e GIFs de prévia (M2)"),
    ("fonte", "monta o atlas da fonte monogram (M2)"),
];

/// Raiz do repositório (o xtask mora em `<raiz>/xtask`).
fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
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
