//! A folha de uma skin no formato **json-array** do Aseprite (o mesmo que o
//! `pet_core::skin` lê): `sheet.png` com as células e `sheet.json` com os
//! quadros (retângulo, duração) e as tags.
//!
//! Quadros com pixels idênticos ocupam uma célula só na imagem: vários
//! quadros do `sheet.json` podem apontar para o mesmo retângulo, com
//! durações diferentes. É assim que tags compostas (sequências feitas de
//! pedaços de outras) não custam pixel nenhum.
//!
//! A saída é determinística: mesma entrada, mesmos bytes.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use pet_core::skin::{Skin, codificar_png};
use serde_json::{Value, json};

/// Células por linha da imagem.
pub const COLUNAS: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuadroFolha {
    /// RGBA da célula inteira (`celula.0 * celula.1 * 4` bytes).
    pub rgba: Vec<u8>,
    pub duracao_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagFolha {
    pub nome: String,
    /// Nome no pack de origem, guardado no campo `data` da tag (o Aseprite
    /// usa esse campo para os dados de usuário da tag).
    pub original: Option<String>,
    pub de: usize,
    pub ate: usize,
    /// `forward`, `reverse` ou `pingpong`.
    pub direcao: String,
}

/// A folha montada, ainda em memória.
#[derive(Debug, Clone)]
pub struct Folha {
    pub png: Vec<u8>,
    pub json: String,
    pub largura: u32,
    pub altura: u32,
    /// Células únicas na imagem.
    pub celulas: usize,
}

/// Monta `sheet.png` e `sheet.json`. `app` vai para `meta.app`.
pub fn montar(
    celula: (u32, u32),
    quadros: &[QuadroFolha],
    tags: &[TagFolha],
    app: &str,
) -> Result<Folha, String> {
    let (cw, ch) = celula;
    let tamanho = (cw * ch * 4) as usize;
    if quadros.is_empty() {
        return Err("folha sem quadros".into());
    }
    let mut unicos: Vec<&[u8]> = Vec::new();
    let mut indice: HashMap<&[u8], usize> = HashMap::new();
    let mut posicao = Vec::with_capacity(quadros.len());
    for (i, q) in quadros.iter().enumerate() {
        if q.rgba.len() != tamanho {
            return Err(format!(
                "quadro {i} com {} bytes; a célula {cw}x{ch} pede {tamanho}",
                q.rgba.len()
            ));
        }
        let n = *indice.entry(q.rgba.as_slice()).or_insert_with(|| {
            unicos.push(q.rgba.as_slice());
            unicos.len() - 1
        });
        posicao.push(n);
    }
    for t in tags {
        if t.de > t.ate || t.ate >= quadros.len() {
            return Err(format!(
                "tag «{}» com quadros {}..={} (há {})",
                t.nome,
                t.de,
                t.ate,
                quadros.len()
            ));
        }
    }
    let colunas = COLUNAS.min(unicos.len() as u32);
    let linhas = (unicos.len() as u32).div_ceil(colunas);
    let (largura, altura) = (colunas * cw, linhas * ch);
    let mut rgba = vec![0u8; (largura * altura * 4) as usize];
    let canto = |n: usize| ((n as u32 % colunas) * cw, (n as u32 / colunas) * ch);
    for (n, celula_rgba) in unicos.iter().enumerate() {
        let (ox, oy) = canto(n);
        for y in 0..ch {
            let origem = (y * cw * 4) as usize;
            let destino = (((oy + y) * largura + ox) * 4) as usize;
            rgba[destino..destino + (cw * 4) as usize]
                .copy_from_slice(&celula_rgba[origem..origem + (cw * 4) as usize]);
        }
    }
    let quadros_json: Vec<Value> = quadros
        .iter()
        .zip(&posicao)
        .enumerate()
        .map(|(i, (q, &n))| {
            let (x, y) = canto(n);
            json!({
                "filename": format!("quadro {i}"),
                "frame": {"x": x, "y": y, "w": cw, "h": ch},
                "rotated": false,
                "trimmed": false,
                "spriteSourceSize": {"x": 0, "y": 0, "w": cw, "h": ch},
                "sourceSize": {"w": cw, "h": ch},
                "duration": q.duracao_ms,
            })
        })
        .collect();
    let tags_json: Vec<Value> = tags
        .iter()
        .map(|t| {
            let mut tag = json!({
                "name": t.nome,
                "from": t.de,
                "to": t.ate,
                "direction": t.direcao,
                "color": "#000000ff",
            });
            if let Some(original) = &t.original {
                tag["data"] = json!(original);
            }
            tag
        })
        .collect();
    let folha_json = json!({
        "frames": quadros_json,
        "meta": {
            "app": app,
            "version": pet_core::VERSAO,
            "image": "sheet.png",
            "format": "RGBA8888",
            "size": {"w": largura, "h": altura},
            "scale": "1",
            "frameTags": tags_json,
            "layers": [],
            "slices": [],
        },
    });
    Ok(Folha {
        png: codificar_png(largura, altura, &rgba)?,
        json: json_bonito(&folha_json)?,
        largura,
        altura,
        celulas: unicos.len(),
    })
}

/// JSON indentado com quebra de linha no fim (como os arquivos do git).
pub fn json_bonito(valor: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(valor)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

/// Grava `sheet.png`, `sheet.json` e `skin.json` em `pasta` (criada se
/// preciso) e confere carregando com o mesmo código do daemon.
pub fn gravar(pasta: &Path, folha: &Folha, skin_json: &str) -> Result<Skin, String> {
    fs::create_dir_all(pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    for (nome, dados) in [
        ("sheet.png", folha.png.as_slice()),
        ("sheet.json", folha.json.as_bytes()),
        ("skin.json", skin_json.as_bytes()),
    ] {
        fs::write(pasta.join(nome), dados)
            .map_err(|e| format!("{}: {e}", pasta.join(nome).display()))?;
    }
    Skin::carregar(pasta)
        .map_err(|e| format!("a skin gravada em {} não carrega: {e}", pasta.display()))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn quadro(cor: u8, duracao_ms: u32) -> QuadroFolha {
        QuadroFolha {
            rgba: [cor, 0, 0, 255].repeat(4),
            duracao_ms,
        }
    }

    fn tag(nome: &str, de: usize, ate: usize) -> TagFolha {
        TagFolha {
            nome: nome.into(),
            original: Some(format!("{nome} (pack)")),
            de,
            ate,
            direcao: "forward".into(),
        }
    }

    #[test]
    fn quadros_iguais_dividem_a_celula() {
        let quadros = [quadro(1, 100), quadro(2, 100), quadro(1, 300)];
        let f = montar((2, 2), &quadros, &[tag("a", 0, 2)], "teste").unwrap();
        assert_eq!(f.celulas, 2);
        assert_eq!((f.largura, f.altura), (4, 2));
        let v: Value = serde_json::from_str(&f.json).unwrap();
        assert_eq!(
            v["frames"][2]["frame"]["x"], 0,
            "o terceiro reusa a célula do primeiro"
        );
        assert_eq!(v["frames"][2]["duration"], 300);
        assert_eq!(v["meta"]["frameTags"][0]["data"], "a (pack)");
        let skin = r#"{"formato":1,"id":"t","nome":"T","autor":"a","licenca":"MIT",
            "redistribuivel":true,"folha":"sheet.png","dados":"sheet.json",
            "celula":[2,2],"pe":[1,2],"toque":[0,0,2,2],"corpo_px":2,
            "estados":{"idle":["a"]}}"#;
        let s = Skin::de_partes(skin, &f.json, &f.png).unwrap();
        assert_eq!(s.quadros.len(), 3);
        assert_eq!(s.quadros[2].origem, s.quadros[0].origem);
        assert!(s.avisos.is_empty(), "{:?}", s.avisos);
    }

    #[test]
    fn grade_quebra_em_colunas() {
        let quadros: Vec<QuadroFolha> = (0..23).map(|i| quadro(i, 100)).collect();
        let f = montar((2, 2), &quadros, &[tag("a", 0, 22)], "teste").unwrap();
        assert_eq!((f.largura, f.altura), (COLUNAS * 2, 3 * 2));
    }

    #[test]
    fn deterministica_e_com_erros_claros() {
        let quadros = [quadro(1, 100), quadro(2, 100)];
        let a = montar((2, 2), &quadros, &[tag("a", 0, 1)], "t").unwrap();
        let b = montar((2, 2), &quadros, &[tag("a", 0, 1)], "t").unwrap();
        assert_eq!((a.png, a.json), (b.png, b.json));
        assert!(montar((2, 2), &quadros, &[tag("a", 0, 2)], "t").is_err());
        assert!(
            montar((3, 3), &quadros, &[], "t").is_err(),
            "tamanho errado"
        );
        assert!(montar((2, 2), &[], &[], "t").is_err());
    }
}
