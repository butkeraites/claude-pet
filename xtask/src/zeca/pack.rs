//! Acha o Parrot 2 (o papagaio verde) dentro do pack *Cute Parrots!*, seja o
//! `.zip` como veio do itch.io, seja a pasta descompactada.
//!
//! O que o pack tem (conferido no zip comprado): uma pasta raiz `Cute
//! Parrots! -Pixel Art Asset Pack/` com `Parrot 1/`, `Parrot 2/` e
//! `Parrot 3/`, cada uma com `Parrot.aseprite` e `Parrot.png` (tiras de
//! 48x48), mais o lixo do macOS (`__MACOSX/`, `._*`, `.DS_Store`), que é
//! ignorado.

use std::fs;
use std::path::{Path, PathBuf};

use crate::zip::Zip;

/// Pasta e arquivos do papagaio verde.
pub const PASTA: &str = "Parrot 2";
pub const ASEPRITE: &str = "Parrot.aseprite";
pub const TIRAS: &str = "Parrot.png";

/// Os arquivos do Parrot 2.
#[derive(Debug)]
pub struct Parrot2 {
    pub aseprite: Vec<u8>,
    pub tiras: Option<Vec<u8>>,
    /// Onde estava (para a mensagem).
    pub onde: String,
}

fn lixo_do_mac(caminho: &str) -> bool {
    caminho
        .split('/')
        .any(|parte| parte == "__MACOSX" || parte.starts_with("._"))
}

/// `…/Parrot 2/<arquivo>`, fora do lixo do macOS.
fn eh(caminho: &str, arquivo: &str) -> bool {
    let partes: Vec<&str> = caminho.split('/').filter(|p| !p.is_empty()).collect();
    !lixo_do_mac(caminho)
        && partes.len() >= 2
        && partes[partes.len() - 2] == PASTA
        && partes[partes.len() - 1] == arquivo
}

const ERRO_LAYOUT: &str = "esperava o pack Cute Parrots! da exclusiveOlive (pastas «Parrot 1», «Parrot 2» e «Parrot 3», cada uma com Parrot.aseprite e Parrot.png); baixe de novo em https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack";

fn unico<'a>(achados: Vec<&'a str>, o_que: &str, pack: &Path) -> Result<Option<&'a str>, String> {
    match achados.as_slice() {
        [] => Ok(None),
        [um] => Ok(Some(um)),
        varios => Err(format!(
            "{}: {} cópias de {PASTA}/{o_que} ({}); deixe só uma",
            pack.display(),
            varios.len(),
            varios.join(", ")
        )),
    }
}

/// Lê o Parrot 2 de um `.zip` ou de uma pasta.
pub fn parrot2(pack: &Path) -> Result<Parrot2, String> {
    if pack.is_dir() {
        return da_pasta(pack);
    }
    let bytes = fs::read(pack).map_err(|e| format!("{}: {e}", pack.display()))?;
    let zip = Zip::ler(&bytes).map_err(|e| format!("{}: {e}; {ERRO_LAYOUT}", pack.display()))?;
    let nomes: Vec<&str> = zip.entradas.iter().map(|e| e.nome.as_str()).collect();
    let ase = unico(
        nomes.iter().copied().filter(|n| eh(n, ASEPRITE)).collect(),
        ASEPRITE,
        pack,
    )?
    .ok_or_else(|| {
        format!(
            "{}: não achei {PASTA}/{ASEPRITE}; {ERRO_LAYOUT}",
            pack.display()
        )
    })?;
    let png = unico(
        nomes.iter().copied().filter(|n| eh(n, TIRAS)).collect(),
        TIRAS,
        pack,
    )?;
    let conteudo = |nome: &str| {
        let entrada = zip
            .entradas
            .iter()
            .find(|e| e.nome == nome)
            .expect("nome veio da lista");
        zip.conteudo(entrada)
    };
    Ok(Parrot2 {
        aseprite: conteudo(ase)?,
        tiras: png.map(conteudo).transpose()?,
        onde: format!("{} → {ase}", pack.display()),
    })
}

fn da_pasta(pasta: &Path) -> Result<Parrot2, String> {
    let mut arquivos: Vec<PathBuf> = Vec::new();
    let mut pendentes = vec![(pasta.to_path_buf(), 0)];
    while let Some((dir, nivel)) = pendentes.pop() {
        let entradas = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entrada in entradas.filter_map(Result::ok) {
            let caminho = entrada.path();
            let tipo = entrada.file_type().map_err(|e| e.to_string())?;
            if tipo.is_dir() && nivel < 4 {
                pendentes.push((caminho, nivel + 1));
            } else if tipo.is_file() {
                arquivos.push(caminho);
            }
        }
    }
    arquivos.sort();
    let relativos: Vec<String> = arquivos
        .iter()
        .map(|c| {
            c.strip_prefix(pasta)
                .unwrap_or(c)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    let achar = |o_que: &str| {
        let achados: Vec<&str> = relativos
            .iter()
            .map(String::as_str)
            .filter(|r| eh(r, o_que))
            .collect();
        unico(achados, o_que, pasta).map(|a| a.map(|r| pasta.join(r)))
    };
    let ase = achar(ASEPRITE)?.ok_or_else(|| {
        format!(
            "{}: não achei {PASTA}/{ASEPRITE}; {ERRO_LAYOUT}",
            pasta.display()
        )
    })?;
    let png = achar(TIRAS)?;
    let ler = |c: &Path| fs::read(c).map_err(|e| format!("{}: {e}", c.display()));
    Ok(Parrot2 {
        aseprite: ler(&ase)?,
        tiras: png.as_deref().map(ler).transpose()?,
        onde: ase.display().to_string(),
    })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::zip::testes::zip_sintetico;

    #[test]
    fn caminhos_do_pack() {
        assert!(eh(
            "Cute Parrots! -Pixel Art Asset Pack/Parrot 2/Parrot.aseprite",
            ASEPRITE
        ));
        assert!(eh("Parrot 2/Parrot.png", TIRAS));
        assert!(!eh(
            "__MACOSX/Cute Parrots! -Pixel Art Asset Pack/Parrot 2/._Parrot.aseprite",
            ASEPRITE
        ));
        assert!(!eh("Parrot 1/Parrot.aseprite", ASEPRITE));
        assert!(!eh("Parrot.aseprite", ASEPRITE));
    }

    #[test]
    fn acha_no_zip_e_ignora_o_lixo_do_mac() {
        let z = zip_sintetico(
            &[
                ("Pack/Parrot 1/Parrot.aseprite", b"um"),
                ("Pack/Parrot 2/Parrot.aseprite", b"dois"),
                ("Pack/Parrot 2/Parrot.png", b"png"),
                ("__MACOSX/Pack/Parrot 2/._Parrot.aseprite", b"lixo"),
            ],
            true,
        );
        let pasta = std::env::temp_dir().join(format!("claude-pet-pack-{}", std::process::id()));
        fs::create_dir_all(&pasta).unwrap();
        let arquivo = pasta.join("pack.zip");
        fs::write(&arquivo, &z).unwrap();
        let p = parrot2(&arquivo).unwrap();
        assert_eq!(p.aseprite, b"dois");
        assert_eq!(p.tiras.as_deref(), Some(&b"png"[..]));
        // Zip sem o Parrot 2: erro que explica o que esperava.
        fs::write(
            &arquivo,
            zip_sintetico(&[("x/Parrot 1/Parrot.aseprite", b"1")], false),
        )
        .unwrap();
        let erro = parrot2(&arquivo).unwrap_err();
        assert!(erro.contains("Cute Parrots"), "{erro}");
        // Pasta descompactada.
        let raiz = pasta.join("descompactado/Cute Parrots!/Parrot 2");
        fs::create_dir_all(&raiz).unwrap();
        fs::write(raiz.join(ASEPRITE), b"da pasta").unwrap();
        let p = parrot2(&pasta.join("descompactado")).unwrap();
        assert_eq!(p.aseprite, b"da pasta");
        assert!(p.tiras.is_none());
        let _ = fs::remove_dir_all(&pasta);
    }
}
