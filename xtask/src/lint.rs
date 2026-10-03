//! `cargo xtask lint-skin <pasta>…`: confere uma skin antes de ela ir para
//! a tela (PLANO.md, M2).
//!
//! **Erros** (saída 1):
//! - a skin não carrega no core (JSON, PNG, folha × `sheet.json`, célula,
//!   quadros fora da folha, tags fora dos quadros, `idle` sem tag válida);
//! - duração de quadro fora de 16–5000 ms;
//! - skin com `redistribuivel: false` dentro de `skins/`, que vai para o git
//!   (decisão 0011).
//!
//! **Avisos:**
//! - os avisos do próprio carregamento (tag de estado que não existe…);
//! - mais de 32 cores;
//! - alfa parcial (o pet não tem anti-aliasing: halo sujo no papel de parede);
//! - a caixa do corpo pulando mais de 3 pixels de arte entre quadros
//!   seguidos de uma tag (corpo = a maior mancha opaca, para chapéu voando,
//!   bolhas e efeitos soltos não contarem);
//! - pés fora da linha do `pe` nos quadros das tags de `chao` (o corpo pode
//!   passar 1 pixel, o rabo às vezes passa);
//! - pixels opacos na borda da célula: um acessório ou contorno pode ter
//!   sido cortado.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use pet_core::animador::sequencia;
use pet_core::skin::Skin;

/// Limites de duração de um quadro.
pub const DURACAO_MIN: u64 = 16;
pub const DURACAO_MAX: u64 = 5000;
/// Mais cores que isto vira aviso.
pub const MAX_CORES: usize = 32;
/// Salto da caixa do corpo entre quadros seguidos (pixels de arte).
pub const MAX_SALTO: i32 = 3;

#[derive(Debug, Default)]
pub struct Achados {
    pub id: String,
    pub erros: Vec<String>,
    pub avisos: Vec<String>,
}

/// RGBA da célula inteira de um quadro (quadros recortados voltam ao lugar).
pub fn celula(skin: &Skin, q: usize) -> Vec<u8> {
    let (cw, ch) = skin.ancoras.celula;
    let quadro = skin.quadros[q];
    let mut rgba = vec![0u8; (cw * ch * 4) as usize];
    let (dx, dy) = quadro.deslocamento;
    for y in 0..quadro.origem.h {
        for x in 0..quadro.origem.w {
            let i =
                (((quadro.origem.y + y) * skin.folha.largura + quadro.origem.x + x) * 4) as usize;
            let o = (((dy + y) * cw + dx + x) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&skin.folha.rgba[i..i + 4]);
        }
    }
    rgba
}

/// Caixa (x0, y0, x1, y1) inclusiva da maior mancha opaca 8-conexa.
pub fn caixa_do_corpo(rgba: &[u8], celula: (i32, i32)) -> Option<(i32, i32, i32, i32)> {
    let (w, h) = celula;
    let opaco = |x: i32, y: i32| rgba[((y * w + x) * 4 + 3) as usize] != 0;
    let mut visto = vec![false; (w * h) as usize];
    let mut melhor: Option<(usize, (i32, i32, i32, i32))> = None;
    for y in 0..h {
        for x in 0..w {
            if visto[(y * w + x) as usize] || !opaco(x, y) {
                continue;
            }
            visto[(y * w + x) as usize] = true;
            let mut pilha = vec![(x, y)];
            let mut c = (x, y, x, y);
            let mut n = 0;
            while let Some((px, py)) = pilha.pop() {
                n += 1;
                c = (c.0.min(px), c.1.min(py), c.2.max(px), c.3.max(py));
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (px + dx, py + dy);
                        if (0..w).contains(&nx)
                            && (0..h).contains(&ny)
                            && !visto[(ny * w + nx) as usize]
                            && opaco(nx, ny)
                        {
                            visto[(ny * w + nx) as usize] = true;
                            pilha.push((nx, ny));
                        }
                    }
                }
            }
            if melhor.is_none_or(|(m, _)| n > m) {
                melhor = Some((n, c));
            }
        }
    }
    melhor.map(|(_, c)| c)
}

fn normalizar(caminho: &Path) -> PathBuf {
    let absoluto = if caminho.is_absolute() {
        caminho.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(caminho)
    };
    let mut saida = PathBuf::new();
    for c in absoluto.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                saida.pop();
            }
            outro => saida.push(outro),
        }
    }
    saida
}

/// A pasta está dentro de `skins/`, a pasta de skins que vai para o git.
pub fn em_skins(pasta: &Path) -> bool {
    normalizar(pasta).starts_with(normalizar(&crate::raiz().join("skins")))
}

/// Confere a skin da pasta.
pub fn conferir(pasta: &Path) -> Achados {
    let mut a = Achados {
        id: pasta
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        ..Achados::default()
    };
    let skin = match Skin::carregar(pasta) {
        Ok(skin) => skin,
        Err(e) => {
            a.erros.push(format!("não carrega: {e}"));
            return a;
        }
    };
    a.id = skin.id.clone();
    a.avisos.extend(skin.avisos.iter().cloned());

    // Durações brutas (o core troca 0 por 100 ms e só avisa).
    let dados = std::fs::read_to_string(pasta.join("sheet.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
    let duracoes: Vec<u64> = dados
        .as_ref()
        .and_then(|v| v["frames"].as_array())
        .map(|q| {
            q.iter()
                .map(|f| f["duration"].as_u64().unwrap_or(0))
                .collect()
        })
        .unwrap_or_default();
    for (i, d) in duracoes.iter().enumerate() {
        if !(DURACAO_MIN..=DURACAO_MAX).contains(d) {
            a.erros.push(format!(
                "quadro {i}: {d} ms, fora de {DURACAO_MIN}–{DURACAO_MAX} ms"
            ));
        }
    }

    if !skin.redistribuivel && em_skins(pasta) {
        a.erros.push(
            "redistribuivel: false dentro de skins/, que vai para o git: o lugar é skins-locais/ (decisão 0011)".into(),
        );
    }

    let (cw, ch) = skin.ancoras.celula;
    let celulas: Vec<Vec<u8>> = (0..skin.quadros.len()).map(|q| celula(&skin, q)).collect();
    let mut cores = BTreeSet::new();
    let mut parciais = 0usize;
    let mut na_borda = BTreeSet::new();
    for (q, rgba) in celulas.iter().enumerate() {
        for (i, p) in rgba.chunks_exact(4).enumerate() {
            if p[3] == 0 {
                continue;
            }
            cores.insert([p[0], p[1], p[2]]);
            if p[3] != 255 {
                parciais += 1;
            }
            let (x, y) = (i as i32 % cw, i as i32 / cw);
            if x == 0 || y == 0 || x == cw - 1 || y == ch - 1 {
                na_borda.insert(q);
            }
        }
    }
    if cores.len() > MAX_CORES {
        a.avisos
            .push(format!("{} cores (mais de {MAX_CORES})", cores.len()));
    }
    if parciais > 0 {
        a.avisos.push(format!(
            "{parciais} pixels com alfa parcial (sem anti-aliasing, por favor)"
        ));
    }
    if !na_borda.is_empty() {
        let lista: Vec<String> = na_borda.iter().map(|q| q.to_string()).collect();
        a.avisos.push(format!(
            "pixels opacos na borda da célula (cortados?) nos quadros {}",
            lista.join(", ")
        ));
    }

    let caixas: Vec<Option<(i32, i32, i32, i32)>> = celulas
        .iter()
        .map(|rgba| caixa_do_corpo(rgba, (cw, ch)))
        .collect();
    for t in &skin.tags {
        let seq = sequencia(t);
        let mut saltos = Vec::new();
        for par in seq.windows(2) {
            let (Some(c0), Some(c1)) = (caixas[par[0]], caixas[par[1]]) else {
                continue;
            };
            let salto = [(c0.0 - c1.0), (c0.1 - c1.1), (c0.2 - c1.2), (c0.3 - c1.3)]
                .iter()
                .map(|d| d.abs())
                .max()
                .unwrap_or(0);
            if salto > MAX_SALTO {
                saltos.push(format!("{}→{} ({salto} px)", par[0] - t.de, par[1] - t.de));
            }
        }
        if !saltos.is_empty() {
            a.avisos.push(format!(
                "tag «{}»: a caixa do corpo pula mais de {MAX_SALTO} px em {}",
                t.nome,
                saltos.join(", ")
            ));
        }
        if skin.chao.contains(&t.nome) {
            let pe = skin.ancoras.pe.1;
            let fora: Vec<String> = (t.de..=t.ate)
                .filter_map(|q| {
                    let c = caixas[q]?;
                    let base = c.3 + 1;
                    (!(pe..=pe + 1).contains(&base)).then(|| format!("{} (base {base})", q - t.de))
                })
                .collect();
            if !fora.is_empty() {
                a.avisos.push(format!(
                    "tag «{}» está no chão, mas os pés saem da linha {pe} nos quadros {}",
                    t.nome,
                    fora.join(", ")
                ));
            }
        }
    }
    a
}

/// `--so-erros`: só o resumo de cada skin e os erros (o `bin/pet verificar`
/// usa assim).
pub fn executar(lista: &[String]) -> Result<bool, String> {
    let a = crate::args::Args::ler(lista, &[], &["--so-erros"])?;
    if a.posicionais().is_empty() {
        return Err("falta a pasta da skin".into());
    }
    let mut passou = true;
    for pasta in a.posicionais() {
        let achados = conferir(Path::new(pasta));
        println!(
            "lint-skin «{}» ({pasta}): {} erro(s), {} aviso(s)",
            achados.id,
            achados.erros.len(),
            achados.avisos.len()
        );
        for e in &achados.erros {
            println!("  ERRO {e}");
        }
        if !a.bandeira("--so-erros") {
            for v in &achados.avisos {
                println!("  aviso {v}");
            }
        }
        passou &= achados.erros.is_empty();
    }
    Ok(passou)
}

#[cfg(test)]
mod testes {
    use std::fs;

    use pet_core::skin::codificar_png;
    use serde_json::json;

    use super::*;

    const L: usize = 8;

    /// Grava uma skin de células 8x8 com os quadros dados (RGBA 8x8) numa
    /// pasta temporária e devolve a pasta.
    fn skin_em(nome: &str, quadros: &[(Vec<u8>, u64)], extra: serde_json::Value) -> PathBuf {
        let pasta = std::env::temp_dir()
            .join(format!("claude-pet-lint-{}-{nome}", std::process::id()))
            .join(nome);
        let _ = fs::remove_dir_all(&pasta);
        fs::create_dir_all(&pasta).unwrap();
        let n = quadros.len();
        let mut folha = vec![0u8; L * n * L * 4];
        for (q, (rgba, _)) in quadros.iter().enumerate() {
            for y in 0..L {
                for x in 0..L {
                    let o = (y * L + x) * 4;
                    let d = (y * L * n + q * L + x) * 4;
                    folha[d..d + 4].copy_from_slice(&rgba[o..o + 4]);
                }
            }
        }
        let png = codificar_png((L * n) as u32, L as u32, &folha).unwrap();
        fs::write(pasta.join("sheet.png"), png).unwrap();
        let frames: Vec<_> = quadros
            .iter()
            .enumerate()
            .map(|(q, (_, d))| {
                json!({"frame": {"x": q * L, "y": 0, "w": L, "h": L},
                       "spriteSourceSize": {"x": 0, "y": 0, "w": L, "h": L},
                       "sourceSize": {"w": L, "h": L}, "duration": d})
            })
            .collect();
        let sheet = json!({"frames": frames, "meta": {"size": {"w": L * n, "h": L},
            "frameTags": [{"name": "a", "from": 0, "to": n - 1, "direction": "forward"}]}});
        fs::write(pasta.join("sheet.json"), sheet.to_string()).unwrap();
        let mut skin = json!({"formato": 1, "id": nome, "nome": "T", "autor": "t", "licenca": "MIT",
            "redistribuivel": true, "folha": "sheet.png", "dados": "sheet.json",
            "celula": [L, L], "pe": [4, 7], "toque": [0, 0, L, L], "corpo_px": 4,
            "estados": {"idle": ["a"]}});
        for (k, v) in extra.as_object().unwrap() {
            skin[k] = v.clone();
        }
        fs::write(pasta.join("skin.json"), skin.to_string()).unwrap();
        pasta
    }

    /// Bloco opaco de (x0, y0) a (x1, y1), inclusive, com alfa `a`.
    fn bloco(x0: usize, y0: usize, x1: usize, y1: usize, a: u8) -> Vec<u8> {
        let mut rgba = vec![0u8; L * L * 4];
        for y in y0..=y1 {
            for x in x0..=x1 {
                rgba[(y * L + x) * 4..(y * L + x) * 4 + 4].copy_from_slice(&[10, 20, 30, a]);
            }
        }
        rgba
    }

    fn juntar(a: &[u8], b: &[u8]) -> Vec<u8> {
        a.chunks_exact(4)
            .zip(b.chunks_exact(4))
            .flat_map(|(p, q)| if q[3] != 0 { q.to_vec() } else { p.to_vec() })
            .collect()
    }

    #[test]
    fn skin_limpa_passa_sem_avisos() {
        let corpo = bloco(2, 3, 5, 6, 255);
        let p = skin_em(
            "limpa",
            &[(corpo.clone(), 100), (corpo, 200)],
            json!({"chao": ["a"]}),
        );
        let a = conferir(&p);
        assert!(
            a.erros.is_empty() && a.avisos.is_empty(),
            "{:?} {:?}",
            a.erros,
            a.avisos
        );
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn duracao_e_carregamento_sao_erro() {
        let p = skin_em(
            "lenta",
            &[(bloco(2, 3, 5, 6, 255), 9000), (bloco(2, 3, 5, 6, 255), 10)],
            json!({}),
        );
        assert_eq!(conferir(&p).erros.len(), 2);
        fs::write(p.join("skin.json"), "{").unwrap();
        assert!(conferir(&p).erros[0].contains("não carrega"));
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn avisos_de_alfa_borda_pes_e_salto() {
        let p = skin_em(
            "torta",
            &[(bloco(2, 3, 5, 6, 128), 100), (bloco(0, 0, 1, 1, 255), 100)],
            json!({"chao": ["a"]}),
        );
        let todos = conferir(&p).avisos.join(" | ");
        assert!(todos.contains("alfa parcial"), "{todos}");
        assert!(todos.contains("borda"), "{todos}");
        assert!(
            todos.contains("pés saem da linha 7 nos quadros 1"),
            "{todos}"
        );
        assert!(todos.contains("pula mais de 3 px em 0→1"), "{todos}");
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn chapeu_solto_nao_conta_como_salto_do_corpo() {
        let corpo = bloco(2, 3, 5, 6, 255);
        let voando = juntar(&corpo, &bloco(4, 1, 6, 1, 255));
        let p = skin_em(
            "chapeu",
            &[(corpo, 100), (voando, 100)],
            json!({"chao": ["a"]}),
        );
        let a = conferir(&p);
        assert!(a.avisos.is_empty(), "{:?}", a.avisos);
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn nao_redistribuivel_so_fora_de_skins() {
        let raiz = crate::raiz();
        assert!(em_skins(&raiz.join("skins/zeca")));
        assert!(em_skins(&raiz.join("skins-locais/../skins/zeca")));
        assert!(!em_skins(&raiz.join("skins-locais/zeca")));
        let p = skin_em(
            "pack",
            &[(bloco(2, 3, 5, 6, 255), 100)],
            json!({"redistribuivel": false}),
        );
        assert!(conferir(&p).erros.is_empty(), "fora de skins/ pode");
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn corpo_e_a_maior_mancha() {
        let rgba = juntar(&bloco(0, 0, 0, 0, 255), &bloco(2, 1, 4, 3, 255));
        assert_eq!(
            caixa_do_corpo(&rgba, (L as i32, L as i32)),
            Some((2, 1, 4, 3))
        );
    }

    #[test]
    fn skin_de_teste_passa() {
        let a = conferir(&crate::raiz().join("skins/_teste"));
        assert!(a.erros.is_empty(), "{:?}", a.erros);
    }
}
