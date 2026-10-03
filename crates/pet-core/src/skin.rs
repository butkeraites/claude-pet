//! Skin: o personagem como dado (decisão 0012).
//!
//! Uma pasta com três arquivos:
//! - `skin.json`: metadados, âncoras (célula, pés, área clicável), altura do
//!   corpo para o cálculo de D e o mapa estado semântico → tags;
//! - `sheet.json`: a folha no formato **json-array** do Aseprite (quadros com
//!   duração, `spriteSourceSize`/`sourceSize` para quadros recortados e
//!   `meta.frameTags` com direção);
//! - `sheet.png`: a folha, RGBA de 8 bits.
//!
//! Defeito estrutural (quadro fora da folha, tag apontando para quadro que
//! não existe, sem estado `idle`) é erro: a skin não carrega e o pet fica
//! escondido (nunca cai na skin de teste). Coisa menor vira aviso, que vai
//! para o `/v1/estado.skin.avisos`.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Cursor;
use std::path::Path;

use serde::Deserialize;

use crate::animador::DURACAO_MIN_MS;
use crate::geometria::{Ancoras, Ret};

/// Versão do formato do `skin.json`.
pub const FORMATO: u32 = 1;
/// Maior célula aceita (pixels de arte).
pub const CELULA_MAX: i32 = 256;
/// Duração usada quando o quadro vem com 0 ms.
const DURACAO_PADRAO: u32 = 100;

#[derive(Debug)]
pub enum ErroSkin {
    Arquivo(String),
    Json(String),
    Png(String),
    Invalida(String),
}

impl fmt::Display for ErroSkin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroSkin::Arquivo(m) => write!(f, "arquivo: {m}"),
            ErroSkin::Json(m) => write!(f, "json: {m}"),
            ErroSkin::Png(m) => write!(f, "png: {m}"),
            ErroSkin::Invalida(m) => write!(f, "skin inválida: {m}"),
        }
    }
}

impl std::error::Error for ErroSkin {}

/// Direção de uma tag do Aseprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direcao {
    Frente,
    Tras,
    PingPong,
    PingPongReverso,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub nome: String,
    pub de: usize,
    pub ate: usize,
    pub direcao: Direcao,
}

/// Um quadro da folha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quadro {
    /// Retângulo do quadro dentro da folha.
    pub origem: Ret,
    /// Onde o canto do quadro cai dentro da célula (`spriteSourceSize.x/y`;
    /// diferente de zero quando o Aseprite recortou o quadro).
    pub deslocamento: (i32, i32),
    pub duracao_ms: u32,
}

impl Quadro {
    /// Retângulo do quadro dentro da célula, em pixels de arte.
    pub fn na_celula(&self) -> Ret {
        Ret::novo(
            self.deslocamento.0,
            self.deslocamento.1,
            self.origem.w,
            self.origem.h,
        )
    }
}

/// Imagem RGBA de 8 bits, com a cópia pré-multiplicada em BGRA que o
/// `wl_shm` ARGB8888 espera em little-endian.
#[derive(Clone, PartialEq, Eq)]
pub struct Imagem {
    pub largura: i32,
    pub altura: i32,
    /// RGBA direto (alfa não multiplicado), como no PNG.
    pub rgba: Vec<u8>,
    /// BGRA pré-multiplicado, pronto para copiar no buffer.
    pub bgra: Vec<u8>,
}

impl fmt::Debug for Imagem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Imagem({}x{})", self.largura, self.altura)
    }
}

impl Imagem {
    pub fn de_rgba(largura: i32, altura: i32, rgba: Vec<u8>) -> Imagem {
        let bgra = rgba
            .chunks_exact(4)
            .flat_map(|p| {
                let a = p[3] as u32;
                let m = |c: u8| ((c as u32 * a + 127) / 255) as u8;
                [m(p[2]), m(p[1]), m(p[0]), p[3]]
            })
            .collect();
        Imagem {
            largura,
            altura,
            rgba,
            bgra,
        }
    }

    /// Pixel BGRA pré-multiplicado em (x, y).
    pub fn bgra_em(&self, x: i32, y: i32) -> [u8; 4] {
        let i = ((y * self.largura + x) * 4) as usize;
        [
            self.bgra[i],
            self.bgra[i + 1],
            self.bgra[i + 2],
            self.bgra[i + 3],
        ]
    }
}

/// Decodifica um PNG qualquer (paleta, cinza, 16 bits) para RGBA de 8 bits.
pub fn decodificar_png(bytes: &[u8]) -> Result<Imagem, ErroSkin> {
    let erro = |e: png::DecodingError| ErroSkin::Png(e.to_string());
    let mut decodificador = png::Decoder::new(Cursor::new(bytes));
    decodificador.set_transformations(
        png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
    );
    let mut leitor = decodificador.read_info().map_err(erro)?;
    let tamanho = leitor
        .output_buffer_size()
        .ok_or_else(|| ErroSkin::Png("imagem grande demais".into()))?;
    let mut dados = vec![0; tamanho];
    let info = leitor.next_frame(&mut dados).map_err(erro)?;
    dados.truncate(info.buffer_size());
    let (largura, altura) = (info.width as i32, info.height as i32);
    let rgba = match info.color_type {
        png::ColorType::Rgba => dados,
        png::ColorType::Rgb => dados
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => dados
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => dados.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => {
            return Err(ErroSkin::Png("paleta não expandida".into()));
        }
    };
    Ok(Imagem::de_rgba(largura, altura, rgba))
}

/// Codifica RGBA de 8 bits em PNG (usado pelo xtask e pelo `/v1/debug/quadro`).
pub fn codificar_png(largura: u32, altura: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut saida = Vec::new();
    let mut codificador = png::Encoder::new(&mut saida, largura, altura);
    codificador.set_color(png::ColorType::Rgba);
    codificador.set_depth(png::BitDepth::Eight);
    let mut escritor = codificador.write_header().map_err(|e| e.to_string())?;
    escritor.write_image_data(rgba).map_err(|e| e.to_string())?;
    escritor.finish().map_err(|e| e.to_string())?;
    Ok(saida)
}

// --- formato em disco ------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkinJson {
    formato: u32,
    id: String,
    nome: String,
    autor: String,
    licenca: String,
    /// De onde veio a arte (página do pack), quando não é nossa.
    #[serde(default)]
    fonte: Option<String>,
    redistribuivel: bool,
    folha: String,
    dados: String,
    celula: [i32; 2],
    pe: [i32; 2],
    toque: [i32; 4],
    corpo_px: u32,
    #[serde(default)]
    escala_padrao: Option<u32>,
    estados: BTreeMap<String, Vec<String>>,
    /// Tags com os pés no chão: o lint confere que os pés ficam na linha do
    /// `pe` nos quadros delas.
    #[serde(default)]
    chao: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FolhaJson {
    frames: Vec<QuadroJson>,
    meta: MetaJson,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuadroJson {
    frame: RetJson,
    #[serde(default)]
    rotated: bool,
    sprite_source_size: RetJson,
    source_size: TamanhoJson,
    duration: u32,
}

#[derive(Debug, Deserialize)]
struct RetJson {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Debug, Deserialize)]
struct TamanhoJson {
    w: i32,
    h: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaJson {
    size: TamanhoJson,
    #[serde(default)]
    frame_tags: Vec<TagJson>,
}

#[derive(Debug, Deserialize)]
struct TagJson {
    name: String,
    from: usize,
    to: usize,
    #[serde(default)]
    direction: Option<String>,
}

// --- a skin carregada ------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Skin {
    pub id: String,
    pub nome: String,
    pub autor: String,
    pub licenca: String,
    pub fonte: Option<String>,
    pub redistribuivel: bool,
    pub ancoras: Ancoras,
    pub corpo_px: u32,
    pub escala_padrao: Option<u32>,
    /// Estado semântico (`idle`, `done_small`, …) → nomes de tag.
    pub estados: BTreeMap<String, Vec<String>>,
    /// Tags com os pés no chão (só as que existem).
    pub chao: Vec<String>,
    pub quadros: Vec<Quadro>,
    pub tags: Vec<Tag>,
    pub folha: Imagem,
    pub avisos: Vec<String>,
}

fn invalida(motivo: impl Into<String>) -> ErroSkin {
    ErroSkin::Invalida(motivo.into())
}

/// Nome de arquivo simples, sem caminho (a skin nunca lê fora da pasta).
fn nome_simples(nome: &str) -> bool {
    !nome.is_empty() && !nome.contains('/') && !nome.contains('\\') && nome != "." && nome != ".."
}

impl Skin {
    /// Carrega e valida a skin da pasta.
    pub fn carregar(pasta: &Path) -> Result<Skin, ErroSkin> {
        let ler = |nome: &str| {
            std::fs::read(pasta.join(nome))
                .map_err(|e| ErroSkin::Arquivo(format!("{}: {e}", pasta.join(nome).display())))
        };
        let texto_skin = String::from_utf8(ler("skin.json")?)
            .map_err(|_| ErroSkin::Json("skin.json não é UTF-8".into()))?;
        let skin_json: SkinJson = serde_json::from_str(&texto_skin)
            .map_err(|e| ErroSkin::Json(format!("skin.json: {e}")))?;
        if !nome_simples(&skin_json.folha) || !nome_simples(&skin_json.dados) {
            return Err(invalida(
                "folha e dados precisam ser nomes de arquivo da própria pasta",
            ));
        }
        let texto_folha = String::from_utf8(ler(&skin_json.dados)?)
            .map_err(|_| ErroSkin::Json(format!("{} não é UTF-8", skin_json.dados)))?;
        let png = ler(&skin_json.folha)?;
        let mut skin = Skin::montar(skin_json, &texto_folha, &png)?;
        let pasta_nome = pasta
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if pasta_nome != skin.id {
            skin.avisos.push(format!(
                "id «{}» diferente do nome da pasta «{pasta_nome}»",
                skin.id
            ));
        }
        Ok(skin)
    }

    /// Monta a skin a partir do conteúdo dos três arquivos.
    pub fn de_partes(skin_json: &str, folha_json: &str, png: &[u8]) -> Result<Skin, ErroSkin> {
        let skin_json: SkinJson = serde_json::from_str(skin_json)
            .map_err(|e| ErroSkin::Json(format!("skin.json: {e}")))?;
        Skin::montar(skin_json, folha_json, png)
    }

    fn montar(s: SkinJson, folha_json: &str, png: &[u8]) -> Result<Skin, ErroSkin> {
        let mut avisos = Vec::new();
        if s.formato != FORMATO {
            return Err(invalida(format!(
                "formato {} (este pet entende o {FORMATO})",
                s.formato
            )));
        }
        let id_ok = !s.id.is_empty()
            && s.id.len() <= 40
            && s.id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        if !id_ok {
            return Err(invalida(format!(
                "id «{}» fora de [a-z0-9_-]{{1,40}}",
                s.id
            )));
        }
        let celula = (s.celula[0], s.celula[1]);
        if !(1..=CELULA_MAX).contains(&celula.0) || !(1..=CELULA_MAX).contains(&celula.1) {
            return Err(invalida(format!(
                "célula {}x{} fora de 1..={CELULA_MAX}",
                celula.0, celula.1
            )));
        }
        let celula_ret = Ret::novo(0, 0, celula.0, celula.1);
        let toque = Ret::novo(s.toque[0], s.toque[1], s.toque[2], s.toque[3]);
        if toque.vazio() || celula_ret.intersecao(&toque) != Some(toque) {
            return Err(invalida("toque precisa caber na célula e ter tamanho"));
        }
        let pe = (s.pe[0], s.pe[1]);
        if !(0..=celula.0).contains(&pe.0) || !(0..=celula.1).contains(&pe.1) {
            return Err(invalida("pe fora da célula"));
        }
        if s.corpo_px == 0 || s.corpo_px as i32 > celula.1 {
            return Err(invalida(
                "corpo_px precisa estar entre 1 e a altura da célula",
            ));
        }

        let folha = decodificar_png(png)?;
        let dados: FolhaJson = serde_json::from_str(folha_json)
            .map_err(|e| ErroSkin::Json(format!("sheet.json: {e}")))?;
        if (dados.meta.size.w, dados.meta.size.h) != (folha.largura, folha.altura) {
            return Err(invalida(format!(
                "sheet.json diz {}x{}, o PNG tem {}x{}",
                dados.meta.size.w, dados.meta.size.h, folha.largura, folha.altura
            )));
        }
        if dados.frames.is_empty() {
            return Err(invalida("folha sem quadros"));
        }
        let limites_folha = Ret::novo(0, 0, folha.largura, folha.altura);
        let mut quadros = Vec::with_capacity(dados.frames.len());
        for (i, q) in dados.frames.iter().enumerate() {
            if q.rotated {
                return Err(invalida(format!("quadro {i} girado (exporte sem rotação)")));
            }
            let origem = Ret::novo(q.frame.x, q.frame.y, q.frame.w, q.frame.h);
            if origem.vazio() || limites_folha.intersecao(&origem) != Some(origem) {
                return Err(invalida(format!("quadro {i} fora da folha")));
            }
            if (q.source_size.w, q.source_size.h) != celula {
                return Err(invalida(format!(
                    "quadro {i} com sourceSize {}x{}, célula é {}x{}",
                    q.source_size.w, q.source_size.h, celula.0, celula.1
                )));
            }
            let na_celula = Ret::novo(
                q.sprite_source_size.x,
                q.sprite_source_size.y,
                q.sprite_source_size.w,
                q.sprite_source_size.h,
            );
            if (na_celula.w, na_celula.h) != (origem.w, origem.h)
                || celula_ret.intersecao(&na_celula) != Some(na_celula)
            {
                return Err(invalida(format!(
                    "quadro {i} com spriteSourceSize incoerente"
                )));
            }
            let duracao_ms = if q.duration == 0 {
                avisos.push(format!(
                    "quadro {i} com duração 0; usando {DURACAO_PADRAO} ms"
                ));
                DURACAO_PADRAO
            } else {
                if (q.duration as u64) < DURACAO_MIN_MS {
                    avisos.push(format!(
                        "quadro {i} com {} ms; o pet toca com {DURACAO_MIN_MS} ms (até 30 fps)",
                        q.duration
                    ));
                }
                q.duration
            };
            quadros.push(Quadro {
                origem,
                deslocamento: (na_celula.x, na_celula.y),
                duracao_ms,
            });
        }

        let mut tags = Vec::with_capacity(dados.meta.frame_tags.len());
        for t in &dados.meta.frame_tags {
            if t.from > t.to || t.to >= quadros.len() {
                return Err(invalida(format!(
                    "tag «{}» aponta para quadros {}..={} (há {})",
                    t.name,
                    t.from,
                    t.to,
                    quadros.len()
                )));
            }
            let direcao = match t.direction.as_deref() {
                None | Some("forward") => Direcao::Frente,
                Some("reverse") => Direcao::Tras,
                Some("pingpong") => Direcao::PingPong,
                Some("pingpong_reverse") => Direcao::PingPongReverso,
                Some(outra) => {
                    avisos.push(format!(
                        "tag «{}» com direção «{outra}»; usando forward",
                        t.name
                    ));
                    Direcao::Frente
                }
            };
            if tags.iter().any(|x: &Tag| x.nome == t.name) {
                avisos.push(format!("tag «{}» repetida; vale a primeira", t.name));
                continue;
            }
            tags.push(Tag {
                nome: t.name.clone(),
                de: t.from,
                ate: t.to,
                direcao,
            });
        }

        let mut estados = BTreeMap::new();
        for (estado, nomes) in s.estados {
            let validas: Vec<String> = nomes
                .into_iter()
                .filter(|nome| {
                    let existe = tags.iter().any(|t| &t.nome == nome);
                    if !existe {
                        avisos.push(format!("estado «{estado}»: tag «{nome}» não existe"));
                    }
                    existe
                })
                .collect();
            if validas.is_empty() {
                avisos.push(format!("estado «{estado}» sem nenhuma tag válida"));
            } else {
                estados.insert(estado, validas);
            }
        }
        if !estados.contains_key("idle") {
            return Err(invalida("falta o estado «idle» (a pose parada)"));
        }
        let mut chao = Vec::with_capacity(s.chao.len());
        for nome in s.chao {
            if tags.iter().any(|t| t.nome == nome) {
                chao.push(nome);
            } else {
                avisos.push(format!("chao: tag «{nome}» não existe"));
            }
        }

        Ok(Skin {
            id: s.id,
            nome: s.nome,
            autor: s.autor,
            licenca: s.licenca,
            fonte: s.fonte,
            redistribuivel: s.redistribuivel,
            ancoras: Ancoras { celula, pe, toque },
            corpo_px: s.corpo_px,
            escala_padrao: s.escala_padrao,
            estados,
            chao,
            quadros,
            tags,
            folha,
            avisos,
        })
    }

    pub fn tag(&self, nome: &str) -> Option<&Tag> {
        self.tags.iter().find(|t| t.nome == nome)
    }

    /// Índices das tags de um estado semântico, na ordem do `skin.json`.
    pub fn tags_do_estado(&self, estado: &str) -> Vec<usize> {
        self.estados
            .get(estado)
            .map(|nomes| {
                nomes
                    .iter()
                    .filter_map(|nome| self.tags.iter().position(|t| &t.nome == nome))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
pub(crate) mod testes {
    use super::*;

    /// Folha mínima 4x2 com dois quadros 2x2 e uma célula 4x4 (o segundo
    /// quadro é recortado: fica no canto inferior direito da célula).
    pub(crate) fn skin_minima() -> Skin {
        let mut rgba = Vec::new();
        for y in 0..2 {
            for x in 0..4 {
                let a = if (x, y) == (3, 1) { 128 } else { 255 };
                rgba.extend_from_slice(&[(x * 60) as u8, (y * 100) as u8, 7, a]);
            }
        }
        let png = codificar_png(4, 2, &rgba).unwrap();
        let folha = r#"{"frames":[
            {"frame":{"x":0,"y":0,"w":2,"h":2},"rotated":false,"trimmed":false,
             "spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":4,"h":4},"duration":100},
            {"frame":{"x":2,"y":0,"w":2,"h":2},"rotated":false,"trimmed":true,
             "spriteSourceSize":{"x":2,"y":2,"w":2,"h":2},"sourceSize":{"w":4,"h":4},"duration":300}],
            "meta":{"size":{"w":4,"h":2},"frameTags":[
              {"name":"idle","from":0,"to":1,"direction":"forward"},
              {"name":"pula","from":0,"to":1,"direction":"pingpong"}]}}"#;
        let skin = r#"{"formato":1,"id":"mini","nome":"Mini","autor":"testes","licenca":"MIT",
            "redistribuivel":true,"folha":"sheet.png","dados":"sheet.json",
            "celula":[4,4],"pe":[2,4],"toque":[0,0,4,4],"corpo_px":4,
            "estados":{"idle":["idle"],"festa":["pula","sumida"]}}"#;
        Skin::de_partes(skin, folha, &png).unwrap()
    }

    /// Skin de células 1x1 cujo `idle` é uma tag só com um quadro por
    /// duração de `duracoes` (cada quadro com uma cor diferente).
    pub(crate) fn skin_com_idle(duracoes: &[u32]) -> Skin {
        let n = duracoes.len();
        let rgba: Vec<u8> = (0..n)
            .flat_map(|i| [i as u8, (i * 7) as u8, 200, 255])
            .collect();
        let png = codificar_png(n as u32, 1, &rgba).unwrap();
        let quadros: Vec<String> = duracoes
            .iter()
            .enumerate()
            .map(|(i, d)| {
                format!(
                    r#"{{"frame":{{"x":{i},"y":0,"w":1,"h":1}},"spriteSourceSize":{{"x":0,"y":0,"w":1,"h":1}},"sourceSize":{{"w":1,"h":1}},"duration":{d}}}"#
                )
            })
            .collect();
        let folha = format!(
            r#"{{"frames":[{}],"meta":{{"size":{{"w":{n},"h":1}},"frameTags":[{{"name":"idle","from":0,"to":{}}}]}}}}"#,
            quadros.join(","),
            n - 1
        );
        let skin = r#"{"formato":1,"id":"densa","nome":"Densa","autor":"testes","licenca":"MIT",
            "redistribuivel":true,"folha":"sheet.png","dados":"sheet.json",
            "celula":[1,1],"pe":[0,1],"toque":[0,0,1,1],"corpo_px":1,
            "estados":{"idle":["idle"]}}"#;
        Skin::de_partes(skin, &folha, &png).unwrap()
    }

    #[test]
    fn carrega_skin_minima_com_aviso_de_tag_sumida() {
        let s = skin_minima();
        assert_eq!(s.quadros.len(), 2);
        assert_eq!(s.quadros[1].deslocamento, (2, 2));
        assert_eq!(s.quadros[1].na_celula(), Ret::novo(2, 2, 2, 2));
        assert_eq!(s.tag("pula").unwrap().direcao, Direcao::PingPong);
        assert_eq!(s.tags_do_estado("festa"), vec![1]);
        assert_eq!(s.tags_do_estado("nada"), Vec::<usize>::new());
        assert_eq!(s.avisos.len(), 1, "{:?}", s.avisos);
        assert!(s.avisos[0].contains("sumida"));
    }

    #[test]
    fn premultiplica_em_bgra() {
        let s = skin_minima();
        // (3,1): R=180 G=100 B=7 A=128 → pré-multiplicado.
        assert_eq!(s.folha.bgra_em(3, 1), [4, 50, 90, 128]);
        // Opaco: igual, só reordenado.
        assert_eq!(s.folha.bgra_em(1, 0), [7, 0, 60, 255]);
    }

    #[test]
    fn png_ida_e_volta() {
        let rgba: Vec<u8> = (0..16u8).collect();
        let png = codificar_png(2, 2, &rgba).unwrap();
        let img = decodificar_png(&png).unwrap();
        assert_eq!((img.largura, img.altura), (2, 2));
        assert_eq!(img.rgba, rgba);
    }

    fn com_folha(folha: &str) -> Result<Skin, ErroSkin> {
        let png = codificar_png(4, 2, &[255u8; 32]).unwrap();
        let skin = r#"{"formato":1,"id":"mini","nome":"Mini","autor":"t","licenca":"MIT",
            "redistribuivel":true,"folha":"sheet.png","dados":"sheet.json",
            "celula":[4,4],"pe":[2,4],"toque":[0,0,4,4],"corpo_px":4,
            "estados":{"idle":["idle"]}}"#;
        Skin::de_partes(skin, folha, &png)
    }

    #[test]
    fn defeitos_estruturais_sao_erro() {
        let fora = r#"{"frames":[{"frame":{"x":3,"y":0,"w":2,"h":2},
            "spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":4,"h":4},"duration":100}],
            "meta":{"size":{"w":4,"h":2},"frameTags":[{"name":"idle","from":0,"to":0}]}}"#;
        assert!(matches!(com_folha(fora), Err(ErroSkin::Invalida(_))));
        let tag_ruim = r#"{"frames":[{"frame":{"x":0,"y":0,"w":2,"h":2},
            "spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":4,"h":4},"duration":100}],
            "meta":{"size":{"w":4,"h":2},"frameTags":[{"name":"idle","from":0,"to":3}]}}"#;
        assert!(matches!(com_folha(tag_ruim), Err(ErroSkin::Invalida(_))));
        let sem_idle = r#"{"frames":[{"frame":{"x":0,"y":0,"w":2,"h":2},
            "spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":4,"h":4},"duration":100}],
            "meta":{"size":{"w":4,"h":2},"frameTags":[{"name":"outra","from":0,"to":0}]}}"#;
        assert!(matches!(com_folha(sem_idle), Err(ErroSkin::Invalida(_))));
        let tamanho_errado = r#"{"frames":[{"frame":{"x":0,"y":0,"w":2,"h":2},
            "spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":4,"h":4},"duration":100}],
            "meta":{"size":{"w":8,"h":2},"frameTags":[{"name":"idle","from":0,"to":0}]}}"#;
        assert!(matches!(
            com_folha(tamanho_errado),
            Err(ErroSkin::Invalida(_))
        ));
    }

    #[test]
    fn duracao_zero_e_direcao_estranha_viram_aviso() {
        let folha = r#"{"frames":[{"frame":{"x":0,"y":0,"w":2,"h":2},
            "spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":4,"h":4},"duration":0}],
            "meta":{"size":{"w":4,"h":2},"frameTags":[{"name":"idle","from":0,"to":0,"direction":"sideways"}]}}"#;
        let s = com_folha(folha).unwrap();
        assert_eq!(s.quadros[0].duracao_ms, 100);
        assert_eq!(s.avisos.len(), 2, "{:?}", s.avisos);
    }

    #[test]
    fn fonte_e_chao_sao_opcionais() {
        let png = codificar_png(4, 2, &[255u8; 32]).unwrap();
        let folha = r#"{"frames":[{"frame":{"x":0,"y":0,"w":2,"h":2},
            "spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":4,"h":4},"duration":100}],
            "meta":{"size":{"w":4,"h":2},"frameTags":[{"name":"idle","from":0,"to":0}]}}"#;
        let skin = r#"{"formato":1,"id":"mini","nome":"Mini","autor":"t","licenca":"MIT",
            "fonte":"https://exemplo","redistribuivel":false,"folha":"sheet.png","dados":"sheet.json",
            "celula":[4,4],"pe":[2,4],"toque":[0,0,4,4],"corpo_px":4,
            "estados":{"idle":["idle"]},"chao":["idle","voo"]}"#;
        let s = Skin::de_partes(skin, folha, &png).unwrap();
        assert_eq!(s.fonte.as_deref(), Some("https://exemplo"));
        assert_eq!(s.chao, vec!["idle".to_owned()]);
        assert_eq!(s.avisos, vec!["chao: tag «voo» não existe".to_owned()]);
        assert!(com_folha(folha).unwrap().chao.is_empty());
    }

    #[test]
    fn nomes_de_arquivo_nao_saem_da_pasta() {
        assert!(nome_simples("sheet.png"));
        assert!(!nome_simples("../segredo.png"));
        assert!(!nome_simples("/etc/passwd"));
        assert!(!nome_simples(".."));
    }
}
