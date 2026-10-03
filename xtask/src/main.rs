//! Ferramentas de desenvolvimento: `cargo xtask <comando>`.
//!
//! Os comandos chegam junto com os marcos que precisam deles (PLANO.md):
//! `skin-teste` e `nitidez` no M1; `skin-importar`, `zeca`, `lint-skin`,
//! `cobertura`, `contato` e `fonte` no M2.

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

fn main() -> ExitCode {
    let comando = std::env::args().nth(1);
    match comando.as_deref() {
        None | Some("ajuda" | "--help" | "-h") => {
            println!("uso: cargo xtask <comando>\n");
            for (nome, descricao) in COMANDOS {
                println!("  {nome:<14} {descricao}");
            }
            ExitCode::SUCCESS
        }
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
