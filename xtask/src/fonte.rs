//! `cargo xtask fonte [--conferir]`: assa a fonte dos balões (monogram, de
//! datagoblin, CC0 1.0; decisão 0052) numa tabela Rust dentro do
//! `pet-core`.
//!
//! Lê `assets/fonte/monogram/monogram-bitmap.json` (cada glifo em 12 linhas
//! de bits, o bit 0 na coluna da esquerda: 5 colunas, e uns poucos acentos
//! que passam para a sexta e a sétima) e escreve
//! `crates/pet-core/src/fonte/glifos.rs`, em ordem de código, já no formato
//! do `rustfmt`. Com `--conferir`, só confere que o arquivo gerado é o de
//! agora (sai 1 se não for). O pet nunca lê a fonte do disco: a tabela vai
//! dentro do binário.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::args::Args;

/// Linhas de cada glifo.
pub const LINHAS: usize = 12;

pub fn origem(raiz: &Path) -> PathBuf {
    raiz.join("assets/fonte/monogram/monogram-bitmap.json")
}

pub fn destino(raiz: &Path) -> PathBuf {
    raiz.join("crates/pet-core/src/fonte/glifos.rs")
}

/// Os glifos do JSON, em ordem de código.
pub fn ler(json: &str) -> Result<Vec<(char, [u8; LINHAS])>, String> {
    let valor: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("o JSON da fonte não se lê: {e}"))?;
    let mapa = valor.as_object().ok_or("o JSON da fonte não é um objeto")?;
    let mut glifos = Vec::with_capacity(mapa.len());
    for (chave, linhas) in mapa {
        let mut letras = chave.chars();
        let (Some(c), None) = (letras.next(), letras.next()) else {
            return Err(format!("a chave {chave:?} não é um caractere só"));
        };
        let linhas = linhas
            .as_array()
            .filter(|l| l.len() == LINHAS)
            .ok_or_else(|| format!("o glifo {c:?} não tem {LINHAS} linhas"))?;
        let mut bits = [0u8; LINHAS];
        for (i, linha) in linhas.iter().enumerate() {
            bits[i] = linha
                .as_u64()
                .filter(|&n| n < 256)
                .ok_or_else(|| format!("o glifo {c:?} tem uma linha que não cabe em 8 bits"))?
                as u8;
        }
        glifos.push((c, bits));
    }
    glifos.sort_by_key(|(c, _)| *c);
    Ok(glifos)
}

/// O literal do caractere: ASCII imprimível como ele é, o resto em `\u{…}`.
fn literal(c: char) -> String {
    match c {
        '\'' => "'\\''".into(),
        '\\' => "'\\\\'".into(),
        ' '..='~' => format!("'{c}'"),
        _ => format!("'\\u{{{:x}}}'", c as u32),
    }
}

/// O arquivo Rust da tabela.
pub fn gerar(glifos: &[(char, [u8; LINHAS])]) -> String {
    let mut s = String::new();
    s.push_str(
        "//! Gerado por `cargo xtask fonte` a partir de\n\
         //! `assets/fonte/monogram/monogram-bitmap.json` (monogram, de datagoblin,\n\
         //! CC0 1.0; decisão 0052). Não edite à mão.\n\
         \n\
         /// (caractere, 12 linhas de bits: o bit 0 é a coluna da esquerda), em\n\
         /// ordem de código.\n\
         pub const GLIFOS: &[(char, [u8; 12])] = &[\n",
    );
    for (c, bits) in glifos {
        let linhas: Vec<String> = bits.iter().map(u8::to_string).collect();
        let _ = writeln!(s, "    ({}, [{}]),", literal(*c), linhas.join(", "));
    }
    s.push_str("];\n");
    s
}

pub fn executar(raiz: &Path, args: &[String]) -> Result<(), String> {
    let args = Args::ler(args, &[], &["--conferir"])?;
    if !args.posicionais().is_empty() {
        return Err("fonte não leva argumentos posicionais".into());
    }
    let json = std::fs::read_to_string(origem(raiz))
        .map_err(|e| format!("não li {}: {e}", origem(raiz).display()))?;
    let glifos = ler(&json)?;
    let texto = gerar(&glifos);
    let destino = destino(raiz);
    if args.bandeira("--conferir") {
        let atual = std::fs::read_to_string(&destino).unwrap_or_default();
        if atual != texto {
            return Err(format!(
                "{} não é o gerado do JSON: rode cargo xtask fonte",
                destino.display()
            ));
        }
        println!("fonte: {} glifos, tabela em dia", glifos.len());
        return Ok(());
    }
    if let Some(pasta) = destino.parent() {
        std::fs::create_dir_all(pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    }
    std::fs::write(&destino, texto).map_err(|e| format!("{}: {e}", destino.display()))?;
    println!("fonte: {} glifos em {}", glifos.len(), destino.display());
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_ordena_e_gera_no_formato_do_rustfmt() {
        let json = r#"{"b":[0,0,0,1,1,1,1,1,1,1,0,0],"a":[0,0,0,0,0,30,17,17,17,30,0,0],
            "✳":[0,0,0,0,0,0,0,0,0,0,0,0],"'":[0,0,0,4,4,4,0,0,0,0,0,0]}"#;
        let glifos = ler(json).unwrap();
        let ordem: Vec<char> = glifos.iter().map(|(c, _)| *c).collect();
        assert_eq!(ordem, vec!['\'', 'a', 'b', '\u{2733}']);
        let texto = gerar(&glifos);
        assert!(texto.contains("    ('a', [0, 0, 0, 0, 0, 30, 17, 17, 17, 30, 0, 0]),\n"));
        assert!(texto.contains("    ('\\u{2733}', ["));
        assert!(texto.contains("    ('\\'', ["));
        assert!(ler(r#"{"ab":[0,0,0,0,0,0,0,0,0,0,0,0]}"#).is_err());
        assert!(ler(r#"{"a":[0,0,0]}"#).is_err());
        assert!(ler(r#"{"a":[0,0,0,0,0,0,0,0,0,0,0,256]}"#).is_err());
    }

    #[test]
    fn a_tabela_do_core_e_a_do_json() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let json = std::fs::read_to_string(origem(&raiz)).unwrap();
        let gerado = gerar(&ler(&json).unwrap());
        let atual = std::fs::read_to_string(destino(&raiz)).unwrap();
        assert_eq!(atual, gerado, "rode cargo xtask fonte");
    }
}
