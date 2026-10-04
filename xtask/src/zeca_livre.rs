//! `cargo xtask zeca-livre [--conferir]`: o Zeca original, arte livre (CC0 1.0) feita com o
//! Claude para o projeto bichinho (decisão 0065).
//!
//! A fonte da arte é o gerador `arte/zeca-livre/zeca.py`: paleta, grades das peças, rig e
//! animações, só com a biblioteca padrão do Python. Os PNG saem dele, nunca de edição à mão.
//! Este comando roda o gerador numa pasta temporária (`zeca.py --quadros`: os quadros dos dois
//! visuais e o `anims.json`, com a verificação da própria arte e sem o ImageMagick) e grava no
//! repositório o que vai para o git:
//! - `arte/zeca-livre/anims.json`, o manifesto: quadros e durações de cada animação, os
//!   trechos do repouso, as transições de cada laço e os gatilhos sugeridos.
//!
//! `--conferir` não grava nada: roda o gerador duas vezes e compara os bytes de tudo o que ele
//! escreveu (o gerador tem de ser determinístico) e confere que o manifesto do git é o que sai
//! agora (gerador mudado sem rodar `cargo xtask zeca-livre`, ou arquivo mexido à mão, reprova).
//! O `bin/pet verificar` roda o `--conferir` quando há `python3`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::args::Args;

/// Onde mora a arte, relativo à raiz do repositório.
pub const ARTE: &str = "arte/zeca-livre";

pub const USO: &str = "uso: cargo xtask zeca-livre [--conferir]";

/// O Python que roda o gerador (`PET_PYTHON` troca).
fn python() -> String {
    std::env::var("PET_PYTHON").unwrap_or_else(|_| "python3".into())
}

/// Pasta temporária que se apaga sozinha.
pub struct Temporaria(PathBuf);

impl Temporaria {
    pub fn nova(nome: &str) -> Result<Temporaria, String> {
        static N: AtomicU32 = AtomicU32::new(0);
        let pasta = std::env::temp_dir().join(format!(
            "bichinho-{nome}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&pasta);
        fs::create_dir_all(&pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
        Ok(Temporaria(pasta))
    }

    pub fn caminho(&self) -> &Path {
        &self.0
    }
}

impl Drop for Temporaria {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Roda `zeca.py --quadros <destino>` (quadros dos dois visuais e `anims.json`). Sem
/// `__pycache__` e com a semente de hash fixa: nada do ambiente entra na saída.
pub fn gerar(arte: &Path, destino: &Path) -> Result<(), String> {
    let gerador = arte.join("zeca.py");
    let saida = Command::new(python())
        .arg("-B")
        .arg(&gerador)
        .arg("--quadros")
        .arg(destino)
        .env("PYTHONHASHSEED", "0")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .map_err(|e| {
            format!(
                "{} {}: {e} (a arte livre sai de um gerador em Python; PET_PYTHON troca o interpretador)",
                python(),
                gerador.display()
            )
        })?;
    if !saida.status.success() {
        return Err(format!(
            "o gerador {} reprovou ({}):\n{}{}",
            gerador.display(),
            saida.status,
            String::from_utf8_lossy(&saida.stdout),
            String::from_utf8_lossy(&saida.stderr)
        ));
    }
    Ok(())
}

/// Todos os arquivos de uma pasta, pelo caminho relativo (com `/`), em ordem.
pub fn arquivos(pasta: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    fn andar(
        base: &Path,
        pasta: &Path,
        saida: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<(), String> {
        let mut entradas: Vec<PathBuf> = fs::read_dir(pasta)
            .map_err(|e| format!("{}: {e}", pasta.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        entradas.sort();
        for caminho in entradas {
            if caminho.is_dir() {
                andar(base, &caminho, saida)?;
            } else {
                let relativo = caminho
                    .strip_prefix(base)
                    .map_err(|e| e.to_string())?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let dados =
                    fs::read(&caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
                saida.insert(relativo, dados);
            }
        }
        Ok(())
    }
    let mut saida = BTreeMap::new();
    andar(pasta, pasta, &mut saida)?;
    Ok(saida)
}

/// O que difere entre duas saídas do gerador: arquivos que só uma tem ou com bytes diferentes.
pub fn diferencas(a: &BTreeMap<String, Vec<u8>>, b: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut lista = Vec::new();
    for (nome, dados) in a {
        match b.get(nome) {
            None => lista.push(format!("{nome}: só na primeira")),
            Some(outro) if outro != dados => lista.push(format!("{nome}: bytes diferentes")),
            Some(_) => {}
        }
    }
    for nome in b.keys().filter(|n| !a.contains_key(*n)) {
        lista.push(format!("{nome}: só na segunda"));
    }
    lista
}

/// Arquivos do repositório que este comando escreve, com o conteúdo que o gerador dá agora:
/// (caminho relativo à raiz, bytes).
fn do_repositorio(gerado: &BTreeMap<String, Vec<u8>>) -> Result<Vec<(String, Vec<u8>)>, String> {
    let manifesto = gerado
        .get("anims.json")
        .ok_or("o gerador não escreveu o anims.json")?;
    Ok(vec![(format!("{ARTE}/anims.json"), manifesto.clone())])
}

/// Confere o determinismo e o que está no git, sem gravar nada. `Ok(problemas)`.
fn conferir(raiz: &Path) -> Result<Vec<String>, String> {
    let arte = raiz.join(ARTE);
    let (a, b) = (
        Temporaria::nova("zeca-livre")?,
        Temporaria::nova("zeca-livre")?,
    );
    gerar(&arte, a.caminho())?;
    gerar(&arte, b.caminho())?;
    let (primeira, segunda) = (arquivos(a.caminho())?, arquivos(b.caminho())?);
    let mut problemas: Vec<String> = diferencas(&primeira, &segunda)
        .into_iter()
        .map(|d| format!("o gerador não é determinístico: {d}"))
        .collect();
    for (relativo, dados) in do_repositorio(&primeira)? {
        match fs::read(raiz.join(&relativo)) {
            Ok(no_git) if no_git == dados => {}
            Ok(_) => problemas.push(format!(
                "{relativo} desatualizado: rode `cargo xtask zeca-livre`"
            )),
            Err(e) => problemas.push(format!("{relativo}: {e}")),
        }
    }
    println!(
        "zeca-livre: o gerador escreveu {} arquivos, duas vezes",
        primeira.len()
    );
    Ok(problemas)
}

pub fn executar(lista: &[String]) -> Result<bool, String> {
    let a = Args::ler(lista, &[], &["--conferir"])?;
    if !a.posicionais().is_empty() {
        return Err("argumento a mais".into());
    }
    let raiz = crate::raiz();
    if a.bandeira("--conferir") {
        let problemas = conferir(&raiz)?;
        for p in &problemas {
            println!("  ✗ {p}");
        }
        if problemas.is_empty() {
            println!("zeca-livre: determinístico e igual ao que está no git");
        }
        return Ok(problemas.is_empty());
    }
    let pasta = Temporaria::nova("zeca-livre")?;
    gerar(&raiz.join(ARTE), pasta.caminho())?;
    let gerado = arquivos(pasta.caminho())?;
    for (relativo, dados) in do_repositorio(&gerado)? {
        let destino = raiz.join(&relativo);
        fs::write(&destino, dados).map_err(|e| format!("{}: {e}", destino.display()))?;
        println!("  {relativo}");
    }
    let quadros = gerado.keys().filter(|n| n.ends_with(".png")).count();
    println!("zeca-livre: {quadros} quadros gerados (os dois visuais)");
    Ok(true)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn diferencas_acham_bytes_e_nomes() {
        let a: BTreeMap<String, Vec<u8>> = [("x".into(), vec![1]), ("y".into(), vec![2])]
            .into_iter()
            .collect();
        let mut b = a.clone();
        assert!(diferencas(&a, &b).is_empty());
        b.insert("y".into(), vec![3]);
        b.insert("z".into(), vec![4]);
        b.remove("x");
        assert_eq!(
            diferencas(&a, &b),
            vec![
                "x: só na primeira".to_owned(),
                "y: bytes diferentes".to_owned(),
                "z: só na segunda".to_owned()
            ]
        );
    }

    #[test]
    fn arquivos_anda_nas_subpastas_em_ordem() {
        let t = Temporaria::nova("zeca-livre-teste").unwrap();
        fs::create_dir_all(t.caminho().join("frames/escuro")).unwrap();
        fs::write(t.caminho().join("frames/escuro/b.png"), [2]).unwrap();
        fs::write(t.caminho().join("frames/a.png"), [1]).unwrap();
        fs::write(t.caminho().join("anims.json"), [0]).unwrap();
        let lista = arquivos(t.caminho()).unwrap();
        assert_eq!(
            lista.keys().cloned().collect::<Vec<_>>(),
            vec!["anims.json", "frames/a.png", "frames/escuro/b.png"]
        );
        let caminho = t.caminho().to_path_buf();
        drop(t);
        assert!(!caminho.exists(), "a pasta temporária se apaga");
    }

    #[test]
    fn manifesto_do_git_tem_os_trechos_e_as_transicoes() {
        // O anims.json do repositório (sem rodar o gerador): o trecho «respira» é 0-3 (a
        // crítica: os quadros 4-6 repetiam os 0-2) e cada laço tem entrada e saída.
        let texto = fs::read_to_string(crate::raiz().join(ARTE).join("anims.json")).unwrap();
        let m: serde_json::Value = serde_json::from_str(&texto).unwrap();
        let trechos = &m["animacoes"]["idle"]["trechos"];
        assert_eq!(trechos["respira"]["quadros"], serde_json::json!([0, 3]));
        assert_eq!(trechos["ginga"]["quadros"], serde_json::json!([7, 13]));
        let lacos = &m["uso"]["transicoes"]["lacos"];
        for laco in ["work", "sleep", "attention", "fly"] {
            for lado in ["entrar", "sair"] {
                let anim = lacos[laco][lado].as_str().unwrap();
                assert!(
                    m["animacoes"][anim]["quadros"].as_array().is_some(),
                    "{laco}.{lado} → {anim} sem quadros"
                );
            }
        }
        assert!(m["licenca"].as_str().unwrap().starts_with("CC0-1.0"));
    }
}
