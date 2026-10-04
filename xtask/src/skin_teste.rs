//! `cargo xtask skin-teste`: gera a skin xadrez de QA em `skins/_teste`.
//!
//! É arte nossa (MIT) e vai para o git. Serve só para testar o motor: um
//! "corpo" xadrez de 32 pixels de altura com o índice do quadro desenhado em
//! dígitos de pixel, uma sombra meio transparente (exercita o "over") e uma
//! tag por estado semântico, cada uma com cores próprias. Nunca é
//! personagem: só aparece com `PET_DEBUG=1` (decisão 0011).
//!
//! A saída é determinística: rodar de novo não muda nenhum byte.

use std::fs;
use std::path::Path;

use pet_core::skin::{Skin, codificar_png};
use serde_json::{Value, json};

/// Lado da célula, em pixels de arte.
pub const CELULA: i32 = 48;
/// Corpo: retângulo de cantos cortados.
const CORPO_X: (i32, i32) = (10, 37);
const CORPO_Y: (i32, i32) = (13, 44);

/// Uma tag por estado semântico: nome, durações (ms) e direção.
pub const TAGS: &[(&str, &[u32], &str)] = &[
    ("idle", &[300, 150, 150, 300], "forward"),
    ("blink", &[120, 160], "forward"),
    ("thinking", &[250, 250, 250], "forward"),
    ("working", &[150, 150, 150, 150], "forward"),
    ("waiting", &[400, 400], "forward"),
    ("ready", &[300, 300], "forward"),
    ("error", &[100, 200, 300], "forward"),
    ("sleep", &[400, 400], "forward"),
    ("yawn", &[200, 300, 400], "forward"),
    ("wave", &[150, 150, 150, 150], "pingpong"),
    ("done_small", &[200, 200], "forward"),
    ("done_medium", &[150, 150, 150], "forward"),
    ("done_big", &[100, 100, 100, 100], "forward"),
    ("alert", &[100, 100], "forward"),
    ("giggle", &[120, 120, 120], "forward"),
    ("dangle", &[150, 150, 150, 150], "pingpong"),
    ("land", &[100, 150, 200], "forward"),
    ("poof_in", &[100, 100, 100], "forward"),
    ("poof_out", &[100, 100, 100], "reverse"),
];

/// Dígitos 3x5.
const DIGITOS: [[&str; 5]; 10] = [
    ["111", "101", "101", "101", "111"],
    ["010", "110", "010", "010", "111"],
    ["111", "001", "111", "100", "111"],
    ["111", "001", "111", "001", "111"],
    ["101", "101", "111", "001", "001"],
    ["111", "100", "111", "001", "111"],
    ["111", "100", "111", "101", "111"],
    ["111", "001", "010", "010", "010"],
    ["111", "101", "111", "101", "111"],
    ["111", "101", "111", "001", "111"],
];

/// HSV (h em graus, s e v em 0..1) → RGB.
fn hsv(h: f64, s: f64, v: f64) -> [u8; 3] {
    let c = v * s;
    let hh = (h % 360.0) / 60.0;
    let x = c * (1.0 - (hh % 2.0 - 1.0).abs());
    let (r, g, b) = match hh as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let f = |u: f64| ((u + m) * 255.0).round() as u8;
    [f(r), f(g), f(b)]
}

struct Folha {
    largura: i32,
    rgba: Vec<u8>,
}

impl Folha {
    fn por(&mut self, x: i32, y: i32, cor: [u8; 4]) {
        let i = ((y * self.largura + x) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&cor);
    }
}

fn no_corpo(x: i32, y: i32) -> bool {
    let dentro = (CORPO_X.0..=CORPO_X.1).contains(&x) && (CORPO_Y.0..=CORPO_Y.1).contains(&y);
    let canto = (x == CORPO_X.0 || x == CORPO_X.1) && (y == CORPO_Y.0 || y == CORPO_Y.1);
    dentro && !canto
}

/// Desenha um quadro com a célula em (ox, oy).
fn desenhar_quadro(folha: &mut Folha, ox: i32, oy: i32, tag: usize, k: usize, indice: usize) {
    let matiz = tag as f64 * 360.0 / TAGS.len() as f64;
    let [ar, ag, ab] = hsv(matiz, 0.45, 0.95);
    let [br, bg, bb] = hsv(matiz, 0.70, 0.72);
    let [cr, cg, cb] = hsv(matiz, 0.80, 0.28);
    let deslocamento = k as i32 * 2;
    for y in 0..CELULA {
        for x in 0..CELULA {
            if !no_corpo(x, y) {
                continue;
            }
            let borda = !no_corpo(x - 1, y)
                || !no_corpo(x + 1, y)
                || !no_corpo(x, y - 1)
                || !no_corpo(x, y + 1);
            let cor = if borda {
                [cr, cg, cb, 255]
            } else if ((x + deslocamento) / 4 + y / 4) % 2 == 0 {
                [ar, ag, ab, 255]
            } else {
                [br, bg, bb, 255]
            };
            folha.por(ox + x, oy + y, cor);
        }
    }
    // Plaquinha com o índice do quadro na folha (dois dígitos).
    for y in 15..=21 {
        for x in 13..=22 {
            folha.por(ox + x, oy + y, [245, 245, 240, 255]);
        }
    }
    let texto = format!("{indice:02}");
    for (n, c) in texto.bytes().enumerate() {
        let glifo = &DIGITOS[(c - b'0') as usize];
        for (dy, linha) in glifo.iter().enumerate() {
            for (dx, bit) in linha.bytes().enumerate() {
                if bit == b'1' {
                    let x = ox + 14 + n as i32 * 4 + dx as i32;
                    folha.por(x, oy + 16 + dy as i32, [25, 25, 35, 255]);
                }
            }
        }
    }
    // Sombra meio transparente sob os pés (a linha do chão é y = 45).
    for (y, x0, x1) in [(45, 15, 32), (46, 17, 30)] {
        for x in x0..=x1 {
            folha.por(ox + x, oy + y, [20, 20, 30, 96]);
        }
    }
}

/// Monta os três arquivos em memória: (sheet.png, sheet.json, skin.json).
pub fn gerar() -> Result<(Vec<u8>, String, String), String> {
    let colunas = TAGS.iter().map(|t| t.1.len()).max().unwrap_or(1) as i32;
    let largura = colunas * CELULA;
    let altura = TAGS.len() as i32 * CELULA;
    let mut folha = Folha {
        largura,
        rgba: vec![0; (largura * altura * 4) as usize],
    };
    let mut quadros = Vec::new();
    let mut tags = Vec::new();
    let mut estados = serde_json::Map::new();
    for (t, (nome, duracoes, direcao)) in TAGS.iter().enumerate() {
        let de = quadros.len();
        for (k, &duracao) in duracoes.iter().enumerate() {
            let (ox, oy) = (k as i32 * CELULA, t as i32 * CELULA);
            desenhar_quadro(&mut folha, ox, oy, t, k, quadros.len());
            quadros.push(json!({
                "filename": format!("_teste {}.aseprite", quadros.len()),
                "frame": {"x": ox, "y": oy, "w": CELULA, "h": CELULA},
                "rotated": false,
                "trimmed": false,
                "spriteSourceSize": {"x": 0, "y": 0, "w": CELULA, "h": CELULA},
                "sourceSize": {"w": CELULA, "h": CELULA},
                "duration": duracao,
            }));
        }
        tags.push(json!({
            "name": nome,
            "from": de,
            "to": quadros.len() - 1,
            "direction": direcao,
            "color": "#000000ff",
        }));
        let lista: Vec<Value> = if *nome == "idle" {
            vec![json!("idle"), json!("blink")]
        } else if *nome == "blink" {
            continue;
        } else {
            vec![json!(nome)]
        };
        estados.insert((*nome).to_owned(), Value::Array(lista));
    }
    let folha_json = json!({
        "frames": quadros,
        "meta": {
            "app": "claude-pet cargo xtask skin-teste",
            "version": pet_core::VERSAO,
            "image": "sheet.png",
            "format": "RGBA8888",
            "size": {"w": largura, "h": altura},
            "scale": "1",
            "frameTags": tags,
            "layers": [],
            "slices": [],
        },
    });
    let skin_json = json!({
        "formato": pet_core::skin::FORMATO,
        "id": "_teste",
        "nome": "Xadrez de teste (QA, nunca é personagem)",
        "autor": "claude-pet",
        "licenca": "MIT",
        "redistribuivel": true,
        "folha": "sheet.png",
        "dados": "sheet.json",
        "celula": [CELULA, CELULA],
        "pe": [24, 45],
        "toque": [CORPO_X.0, CORPO_Y.0, CORPO_X.1 - CORPO_X.0 + 1, CORPO_Y.1 - CORPO_Y.0 + 1],
        "corpo_px": CORPO_Y.1 - CORPO_Y.0 + 1,
        "escala_padrao": 4,
        "estados": estados,
    });
    let png = codificar_png(largura as u32, altura as u32, &folha.rgba)?;
    let texto = |v: &Value| serde_json::to_string_pretty(v).map(|s| s + "\n");
    Ok((
        png,
        texto(&folha_json).map_err(|e| e.to_string())?,
        texto(&skin_json).map_err(|e| e.to_string())?,
    ))
}

/// Gera, grava em `pasta` e valida carregando com o mesmo código do daemon.
pub fn executar(pasta: &Path) -> Result<Skin, String> {
    let (png, folha_json, skin_json) = gerar()?;
    fs::create_dir_all(pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    for (nome, dados) in [
        ("sheet.png", png.as_slice()),
        ("sheet.json", folha_json.as_bytes()),
        ("skin.json", skin_json.as_bytes()),
    ] {
        fs::write(pasta.join(nome), dados).map_err(|e| format!("{nome}: {e}"))?;
    }
    Skin::carregar(pasta).map_err(|e| format!("a skin gerada não carrega: {e}"))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_skin_valida_e_deterministica() {
        let (png, folha, skin) = gerar().unwrap();
        let carregada = Skin::de_partes(&skin, &folha, &png).unwrap();
        assert!(carregada.avisos.is_empty(), "{:?}", carregada.avisos);
        assert_eq!(carregada.tags.len(), TAGS.len());
        assert_eq!(carregada.tags_do_estado("idle").len(), 2);
        assert_eq!(carregada.corpo_px, 32);
        for t in &carregada.tags {
            let n = t.ate - t.de + 1;
            assert!((2..=4).contains(&n), "{} com {n} quadros", t.nome);
        }
        assert!(
            carregada
                .quadros
                .iter()
                .all(|q| (100..=400).contains(&q.duracao_ms))
        );
        assert_eq!(gerar().unwrap().0, png, "PNG muda entre execuções");
    }

    /// O que está no git é exatamente o que o gerador produz: mudou o
    /// gerador sem rodar `cargo xtask skin-teste` (ou mexeu nos arquivos à
    /// mão), este teste avisa.
    #[test]
    fn arquivos_commitados_sao_os_do_gerador() {
        let pasta = crate::raiz().join("skins/_teste");
        let (png, folha, skin) = gerar().unwrap();
        let ler = |nome: &str| std::fs::read(pasta.join(nome)).unwrap();
        let aviso = "skins/_teste desatualizada: rode `cargo xtask skin-teste`";
        assert!(ler("sheet.png") == png, "{aviso} (sheet.png)");
        assert!(
            ler("sheet.json") == folha.as_bytes(),
            "{aviso} (sheet.json)"
        );
        assert!(ler("skin.json") == skin.as_bytes(), "{aviso} (skin.json)");
    }

    #[test]
    fn quadros_diferentes_tem_pixels_diferentes() {
        let (png, folha, skin) = gerar().unwrap();
        let s = Skin::de_partes(&skin, &folha, &png).unwrap();
        let pixels = |q: usize| {
            let o = s.quadros[q].origem;
            (0..o.h)
                .flat_map(|y| (0..o.w).map(move |x| (x, y)))
                .map(|(x, y)| s.folha.bgra_em(o.x + x, o.y + y))
                .collect::<Vec<_>>()
        };
        assert_ne!(pixels(0), pixels(1));
        assert_ne!(pixels(0), pixels(4));
    }
}
