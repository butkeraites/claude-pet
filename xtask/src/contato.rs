//! `cargo xtask contato <pasta> [--saida <pasta>] [--escala N] [--copia <pasta>]`:
//! a folha de contato e as prévias animadas de uma skin, para o Renan
//! aprovar o personagem (PLANO.md, "Zeca: arte e skin", item 5).
//!
//! Escreve em `--saida` (padrão `tmp/contato/<id>/`, fora do git, porque a
//! skin pode ser derivada de um pack que não pode ser redistribuído; uma
//! skin não redistribuível só sai para `tmp/` ou `skins-locais/`):
//! - `contato.png`: todas as tags, cada quadro com o índice e a duração,
//!   sobre o fundo escuro `#0B0C16` (o tema escuro do Omarchy) e, logo
//!   abaixo, sobre o claro `#EFF1F5`, em escala ×4; o título traz a
//!   impressão digital da skin (decisão 0026);
//! - `contato.sha256`: essa impressão, para o `bin/pet skin-aprovar` conferir
//!   que a skin aprovada é a da folha que o Renan viu;
//! - `previa/<tag>.gif`: um GIF por tag, com as durações de verdade, com o
//!   fundo escuro e o claro lado a lado;
//! - `previa/repouso.gif`: o estado parado como o daemon toca (pose fixa e
//!   rajadas, o mesmo `animador::Repouso`).
//!
//! `--copia <pasta>` põe numa pasta só a folha (`contato-<id>.png`, com o
//! `contato-<id>.sha256`) e os GIFs dos estados que mais importam (parado,
//! piando, comendo, voando, mergulho, susto, pousando), com nomes legíveis;
//! as cópias velhas desses nomes são apagadas antes.
//!
//! Todos os quadros são recortados no mesmo retângulo (a união de tudo que
//! é opaco na skin, com folga), para o movimento entre eles aparecer.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use pet_core::animador::{Repouso, sequencia};
use pet_core::skin::{Skin, codificar_png};

use crate::args::Args;
use crate::fonte_mini;
use crate::importar;
use crate::lint::celula;

pub const ESCURO: [u8; 4] = [0x0B, 0x0C, 0x16, 255];
pub const CLARO: [u8; 4] = [0xEF, 0xF1, 0xF5, 255];
/// Fundo da folha, diferente dos dois fundos de teste.
const PAPEL: [u8; 4] = [0x2A, 0x2B, 0x36, 255];
const TINTA: [u8; 4] = [0xE6, 0xE6, 0xEA, 255];
const TINTA_FRACA: [u8; 4] = [0x9A, 0x9C, 0xAA, 255];
const MARGEM: i32 = 16;
const ESPACO: i32 = 6;
const COLUNAS: usize = 12;
/// Escala dos rótulos (fonte 3x5).
const LETRA: i32 = 2;
/// Quanto tempo de repouso o `repouso.gif` mostra: um ciclo inteiro do
/// Zeca (três respiradas e uma levantada, ~18 s).
const REPOUSO_MS: u64 = 20_000;

/// Retângulo da célula que aparece na folha e nos GIFs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recorte {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// União de tudo que é opaco em todos os quadros, com 2 pixels de folga.
pub fn recorte(celulas: &[Vec<u8>], celula: (i32, i32)) -> Recorte {
    let (w, h) = celula;
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, -1, -1);
    for rgba in celulas {
        for (i, p) in rgba.chunks_exact(4).enumerate() {
            if p[3] != 0 {
                let (x, y) = (i as i32 % w, i as i32 / w);
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < 0 {
        return Recorte { x: 0, y: 0, w, h };
    }
    let (x0, y0) = ((x0 - 2).max(0), (y0 - 2).max(0));
    let (x1, y1) = ((x1 + 2).min(w - 1), (y1 + 2).min(h - 1));
    Recorte {
        x: x0,
        y: y0,
        w: x1 - x0 + 1,
        h: y1 - y0 + 1,
    }
}

/// Uma imagem RGBA para desenhar.
pub struct Tela {
    pub w: i32,
    pub h: i32,
    pub rgba: Vec<u8>,
}

impl Tela {
    pub fn nova(w: i32, h: i32, fundo: [u8; 4]) -> Tela {
        Tela {
            w,
            h,
            rgba: fundo.repeat((w * h) as usize),
        }
    }

    fn por(&mut self, x: i32, y: i32, cor: [u8; 4]) {
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            let i = ((y * self.w + x) * 4) as usize;
            self.rgba[i..i + 4].copy_from_slice(&cor);
        }
    }

    pub fn retangulo(&mut self, x: i32, y: i32, w: i32, h: i32, cor: [u8; 4]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.por(xx, yy, cor);
            }
        }
    }

    pub fn texto(&mut self, x: i32, y: i32, texto: &str, cor: [u8; 4]) {
        for (px, py) in fonte_mini::pixels(texto, x, y, LETRA) {
            self.por(px, py, cor);
        }
    }

    /// Cola o recorte de uma célula ampliada `escala` vezes, com "over"
    /// sobre o que já está lá (o pet não tem alfa parcial, mas não custa).
    pub fn colar(&mut self, x: i32, y: i32, rgba: &[u8], cw: i32, r: Recorte, escala: i32) {
        for sy in 0..r.h {
            for sx in 0..r.w {
                let i = (((r.y + sy) * cw + r.x + sx) * 4) as usize;
                let p = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
                if p[3] == 0 {
                    continue;
                }
                for dy in 0..escala {
                    for dx in 0..escala {
                        let (tx, ty) = (x + sx * escala + dx, y + sy * escala + dy);
                        if !(0..self.w).contains(&tx) || !(0..self.h).contains(&ty) {
                            continue;
                        }
                        let j = ((ty * self.w + tx) * 4) as usize;
                        let a = p[3] as u32;
                        for (destino, &fonte) in self.rgba[j..j + 3].iter_mut().zip(&p[..3]) {
                            let base = *destino as u32;
                            *destino = ((fonte as u32 * a + base * (255 - a) + 127) / 255) as u8;
                        }
                        self.rgba[j + 3] = 255;
                    }
                }
            }
        }
    }

    pub fn png(&self) -> Result<Vec<u8>, String> {
        codificar_png(self.w as u32, self.h as u32, &self.rgba)
    }
}

/// Estados da skin que tocam a tag (para o cabeçalho).
fn estados_da_tag(skin: &Skin, tag: &str) -> Vec<String> {
    skin.estados
        .iter()
        .filter(|(_, tags)| tags.iter().any(|t| t == tag))
        .map(|(e, _)| e.clone())
        .collect()
}

/// Nome original da tag no pack (o campo `data` do arquivo de dados).
fn originais(pasta: &Path) -> HashMap<String, String> {
    let v = crate::lint::dados_crus(pasta).unwrap_or_default();
    v["meta"]["frameTags"]
        .as_array()
        .map(|tags| {
            tags.iter()
                .filter_map(|t| {
                    Some((
                        t["name"].as_str()?.to_owned(),
                        t["data"].as_str()?.to_owned(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A folha de contato. `sha256`: a impressão digital da skin (aparece no
/// título, para conferir com o que o `bin/pet skin-aprovar` mostra).
pub fn folha(
    skin: &Skin,
    celulas: &[Vec<u8>],
    r: Recorte,
    escala: i32,
    originais: &HashMap<String, String>,
    sha256: &str,
) -> Tela {
    let (cw, _) = skin.ancoras.celula;
    let caixa_w = r.w * escala;
    let caixa_h = r.h * escala;
    let linha_texto = fonte_mini::altura(LETRA) + 6;
    let bloco_quadro_h = caixa_h * 2 + linha_texto;
    let titulo = format!(
        "{} ({}) · {} tags · {} quadros · celula {}x{} · x{escala} · sha {}",
        skin.nome,
        skin.id,
        skin.tags.len(),
        skin.quadros.len(),
        skin.ancoras.celula.0,
        skin.ancoras.celula.1,
        &sha256[..sha256.len().min(12)]
    );
    let legenda = format!(
        "fundo escuro 0B0C16 em cima e claro EFF1F5 embaixo · rotulo: quadro · ms · recorte {}x{} em ({}, {})",
        r.w, r.h, r.x, r.y
    );
    let cabecalhos: Vec<String> = skin
        .tags
        .iter()
        .map(|t| {
            let total_ms: u64 = (t.de..=t.ate)
                .map(|q| skin.quadros[q].duracao_ms as u64)
                .sum();
            let original = originais
                .get(&t.nome)
                .map(|o| format!(" ({o})"))
                .unwrap_or_else(|| " (composta)".into());
            let estados = estados_da_tag(skin, &t.nome);
            let estados = if estados.is_empty() {
                String::new()
            } else {
                format!(" · estados: {}", estados.join(", "))
            };
            format!(
                "{}{original} · {} quadros · {total_ms} ms{estados}",
                t.nome,
                t.ate - t.de + 1
            )
        })
        .collect();
    let texto_max = std::iter::once(&titulo)
        .chain([&legenda])
        .chain(&cabecalhos)
        .map(|t| fonte_mini::largura(t, LETRA))
        .max()
        .unwrap_or(0);
    let grade = COLUNAS as i32 * (caixa_w + ESPACO) - ESPACO;
    let largura = MARGEM * 2 + grade.max(texto_max);
    let fileiras: usize = skin
        .tags
        .iter()
        .map(|t| (t.ate - t.de + 1).div_ceil(COLUNAS))
        .sum();
    let altura = MARGEM * 2
        + linha_texto * 3
        + skin.tags.len() as i32 * (linha_texto + ESPACO * 2)
        + fileiras as i32 * (bloco_quadro_h + ESPACO);
    let mut tela = Tela::nova(largura, altura, PAPEL);
    let mut y = MARGEM;
    tela.texto(MARGEM, y, &titulo, TINTA);
    y += linha_texto;
    tela.texto(MARGEM, y, &legenda, TINTA_FRACA);
    y += linha_texto * 2;
    for (t, cabecalho) in skin.tags.iter().zip(&cabecalhos) {
        y += ESPACO;
        tela.texto(MARGEM, y, cabecalho, TINTA);
        y += linha_texto + ESPACO;
        let quadros: Vec<usize> = (t.de..=t.ate).collect();
        for fileira in quadros.chunks(COLUNAS) {
            for (k, &q) in fileira.iter().enumerate() {
                let x = MARGEM + k as i32 * (caixa_w + ESPACO);
                tela.retangulo(x, y, caixa_w, caixa_h, ESCURO);
                tela.retangulo(x, y + caixa_h, caixa_w, caixa_h, CLARO);
                tela.colar(x, y, &celulas[q], cw, r, escala);
                tela.colar(x, y + caixa_h, &celulas[q], cw, r, escala);
                tela.texto(
                    x,
                    y + caixa_h * 2 + 3,
                    &format!("{} · {}", q - t.de, skin.quadros[q].duracao_ms),
                    TINTA_FRACA,
                );
            }
            y += bloco_quadro_h + ESPACO;
        }
    }
    tela
}

/// GIF com paleta exata (o pet tem poucas cores; nada de quantização).
/// `quadros`: (RGBA opaco, centésimos de segundo).
pub fn gif(quadros: &[(Vec<u8>, u16)], w: u16, h: u16) -> Result<Vec<u8>, String> {
    let mut paleta: Vec<u8> = Vec::new();
    let mut indice: HashMap<[u8; 3], u8> = HashMap::new();
    let mut indexados = Vec::with_capacity(quadros.len());
    for (rgba, _) in quadros {
        let mut px = Vec::with_capacity(rgba.len() / 4);
        for p in rgba.chunks_exact(4) {
            let c = [p[0], p[1], p[2]];
            let n = match indice.get(&c) {
                Some(&n) => n,
                None => {
                    if indice.len() == 256 {
                        return Err("mais de 256 cores num GIF".into());
                    }
                    let n = indice.len() as u8;
                    indice.insert(c, n);
                    paleta.extend_from_slice(&c);
                    n
                }
            };
            px.push(n);
        }
        indexados.push(px);
    }
    // A paleta global do GIF tem tamanho de potência de 2 (mínimo 2 cores).
    let mut tamanho = 2;
    while tamanho < indice.len() {
        tamanho *= 2;
    }
    paleta.resize(tamanho * 3, 0);
    let mut saida = Vec::new();
    {
        let erro = |e: gif::EncodingError| e.to_string();
        let mut enc = gif::Encoder::new(&mut saida, w, h, &paleta).map_err(erro)?;
        enc.set_repeat(gif::Repeat::Infinite).map_err(erro)?;
        for (px, (_, cs)) in indexados.into_iter().zip(quadros) {
            let quadro = gif::Frame {
                width: w,
                height: h,
                buffer: Cow::Owned(px),
                delay: *cs,
                ..gif::Frame::default()
            };
            enc.write_frame(&quadro).map_err(erro)?;
        }
    }
    Ok(saida)
}

/// Vão entre o lado escuro e o claro dos GIFs, em pixels.
const VAO_GIF: i32 = 4;

/// Tamanho de um quadro de GIF: o recorte ampliado duas vezes, lado a lado.
fn tamanho_gif(r: Recorte, escala: i32) -> (u16, u16) {
    ((r.w * escala * 2 + VAO_GIF) as u16, (r.h * escala) as u16)
}

/// Um quadro do GIF: o recorte da célula ampliado sobre o fundo escuro (à
/// esquerda) e sobre o claro (à direita), para escolher o contorno vendo os
/// dois temas em movimento.
fn quadro_gif(rgba: &[u8], cw: i32, r: Recorte, escala: i32) -> Vec<u8> {
    let (w, h) = (r.w * escala, r.h * escala);
    let mut t = Tela::nova(w * 2 + VAO_GIF, h, PAPEL);
    t.retangulo(0, 0, w, h, ESCURO);
    t.retangulo(w + VAO_GIF, 0, w, h, CLARO);
    t.colar(0, 0, rgba, cw, r, escala);
    t.colar(w + VAO_GIF, 0, rgba, cw, r, escala);
    t.rgba
}

/// Centésimos de segundo para o GIF (no mínimo 2: menos que isso os
/// navegadores trocam por 10).
fn cs(ms: u64) -> u16 {
    (ms.div_ceil(10)).clamp(2, u16::MAX as u64) as u16
}

/// GIF de uma tag, na ordem em que ela toca.
pub fn gif_da_tag(
    skin: &Skin,
    celulas: &[Vec<u8>],
    tag: usize,
    r: Recorte,
    escala: i32,
) -> Result<Vec<u8>, String> {
    let cw = skin.ancoras.celula.0;
    let quadros: Vec<(Vec<u8>, u16)> = sequencia(&skin.tags[tag])
        .into_iter()
        .map(|q| {
            (
                quadro_gif(&celulas[q], cw, r, escala),
                cs(skin.quadros[q].duracao_ms as u64),
            )
        })
        .collect();
    let (w, h) = tamanho_gif(r, escala);
    gif(&quadros, w, h)
}

/// GIF do repouso como o daemon toca: pose fixa e rajadas.
pub fn gif_do_repouso(
    skin: &Skin,
    celulas: &[Vec<u8>],
    r: Recorte,
    escala: i32,
) -> Result<Vec<u8>, String> {
    let cw = skin.ancoras.celula.0;
    let repouso = Repouso::novo(skin, 0);
    let mut quadros = Vec::new();
    let mut t = 0;
    while t < REPOUSO_MS {
        let (q, proxima) = repouso.em(t);
        let ate = proxima.min(REPOUSO_MS).max(t + 1);
        quadros.push((quadro_gif(&celulas[q], cw, r, escala), cs(ate - t)));
        t = ate;
    }
    let (w, h) = tamanho_gif(r, escala);
    gif(&quadros, w, h)
}

/// Prévias com nome legível: (nome do arquivo, estado semântico). O
/// `parado` é o repouso.
const PREVIAS: &[(&str, &str)] = &[
    ("piando", "waiting"),
    ("comendo", "working"),
    ("decolando-voando", "done_medium"),
    ("mergulho-chapeu-voando", "done_big"),
    ("susto-chapeu-voando", "error"),
    ("pousando", "land"),
];

fn gravar(caminho: &Path, dados: &[u8]) -> Result<(), String> {
    fs::write(caminho, dados).map_err(|e| format!("{}: {e}", caminho.display()))
}

pub fn executar(lista: &[String]) -> Result<(), String> {
    let a = Args::ler(lista, &["--saida", "--escala", "--copia"], &[])?;
    let [pasta] = a.posicionais() else {
        return Err("dê uma pasta de skin".into());
    };
    let pasta = Path::new(pasta);
    let skin = Skin::carregar(pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    let sha256 = pet_core::aprovacao::impressao_da_pasta(pasta)?;
    let escala: i32 = match a.valor("--escala") {
        Some(e) => e
            .parse()
            .ok()
            .filter(|e| (1..=12).contains(e))
            .ok_or_else(|| format!("--escala inválida: «{e}» (1 a 12)"))?,
        None => 4,
    };
    let saida = a
        .valor("--saida")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::raiz().join("tmp/contato").join(&skin.id));
    let copia = a.valor("--copia").map(PathBuf::from);
    // Folha e GIFs de uma skin de pack têm os pixels do pack: só para fora
    // do git (decisão 0011).
    for destino in std::iter::once(&saida).chain(copia.as_ref()) {
        importar::conferir_destino(destino, skin.redistribuivel)?;
    }
    let previa = saida.join("previa");
    fs::create_dir_all(&previa).map_err(|e| format!("{}: {e}", previa.display()))?;
    // GIFs de tags que não existem mais não ficam para trás.
    for velho in fs::read_dir(&previa)
        .map_err(|e| format!("{}: {e}", previa.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "gif"))
    {
        fs::remove_file(&velho).map_err(|e| format!("{}: {e}", velho.display()))?;
    }
    let celulas: Vec<Vec<u8>> = (0..skin.quadros.len()).map(|q| celula(&skin, q)).collect();
    let r = recorte(&celulas, skin.ancoras.celula);
    let folha_png = folha(&skin, &celulas, r, escala, &originais(pasta), &sha256).png()?;
    let caminho_folha = saida.join("contato.png");
    gravar(&caminho_folha, &folha_png)?;
    let linha_sha = format!("{sha256}\n");
    gravar(&saida.join("contato.sha256"), linha_sha.as_bytes())?;
    for (i, t) in skin.tags.iter().enumerate() {
        let arquivo = previa.join(format!("{}.gif", t.nome));
        gravar(&arquivo, &gif_da_tag(&skin, &celulas, i, r, escala)?)?;
    }
    let repouso = previa.join("repouso.gif");
    gravar(&repouso, &gif_do_repouso(&skin, &celulas, r, escala)?)?;
    println!(
        "contato «{}» (sha {}): {} e {} GIFs em {}",
        skin.id,
        &sha256[..12],
        caminho_folha.display(),
        skin.tags.len() + 1,
        previa.display()
    );
    if let Some(copia) = copia {
        fs::create_dir_all(&copia).map_err(|e| format!("{}: {e}", copia.display()))?;
        // Só os nomes desta skin (zeca-*.gif também casaria com os do
        // zeca-contorno): apaga as cópias velhas, inclusive de um estado que
        // a skin não tem mais.
        let id = &skin.id;
        let nomes_gif: Vec<String> = std::iter::once("parado")
            .chain(PREVIAS.iter().map(|(nome, _)| *nome))
            .map(|nome| format!("{id}-{nome}.gif"))
            .collect();
        for velho in nomes_gif
            .iter()
            .chain([
                &format!("contato-{id}.png"),
                &format!("contato-{id}.sha256"),
            ])
            .map(|n| copia.join(n))
            .filter(|p| p.exists())
        {
            fs::remove_file(&velho).map_err(|e| format!("{}: {e}", velho.display()))?;
        }
        let mut copiados = vec![copia.join(format!("contato-{id}.png"))];
        gravar(&copiados[0], &folha_png)?;
        gravar(
            &copia.join(format!("contato-{id}.sha256")),
            linha_sha.as_bytes(),
        )?;
        let parado = copia.join(format!("{id}-parado.gif"));
        fs::copy(&repouso, &parado).map_err(|e| format!("{}: {e}", parado.display()))?;
        copiados.push(parado);
        for (nome, estado) in PREVIAS {
            if let Some(&tag) = skin.tags_do_estado(estado).first() {
                let destino = copia.join(format!("{id}-{nome}.gif"));
                let origem = previa.join(format!("{}.gif", skin.tags[tag].nome));
                fs::copy(&origem, &destino).map_err(|e| format!("{}: {e}", destino.display()))?;
                copiados.push(destino);
            }
        }
        for c in copiados {
            println!("  {}", c.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn recorte_e_a_uniao_com_folga() {
        let mut a = vec![0u8; 10 * 10 * 4];
        let mut b = a.clone();
        a[(3 * 10 + 4) * 4 + 3] = 255;
        b[(6 * 10 + 7) * 4 + 3] = 255;
        assert_eq!(
            recorte(&[a, b], (10, 10)),
            Recorte {
                x: 2,
                y: 1,
                w: 8,
                h: 8
            }
        );
        assert_eq!(
            recorte(&[vec![0u8; 400]], (10, 10)),
            Recorte {
                x: 0,
                y: 0,
                w: 10,
                h: 10
            }
        );
    }

    #[test]
    fn gif_tem_paleta_exata_e_duracoes() {
        let vermelho = [255, 0, 0, 255].repeat(4);
        let azul = [0, 0, 255, 255].repeat(4);
        let g = gif(&[(vermelho, 10), (azul, 25)], 2, 2).unwrap();
        let mut leitor = gif::DecodeOptions::new().read_info(g.as_slice()).unwrap();
        let q0 = leitor.read_next_frame().unwrap().unwrap().clone();
        let q1 = leitor.read_next_frame().unwrap().unwrap().clone();
        assert_eq!((q0.delay, q1.delay), (10, 25));
        let paleta = leitor.global_palette().unwrap();
        assert_eq!(&paleta[..6], &[255, 0, 0, 0, 0, 255]);
        assert_eq!(cs(100), 10);
        assert_eq!(cs(5), 2);
    }

    #[test]
    fn folha_e_gifs_da_skin_de_teste() {
        let pasta = crate::raiz().join("skins/_teste");
        let skin = Skin::carregar(&pasta).unwrap();
        let celulas: Vec<Vec<u8>> = (0..skin.quadros.len()).map(|q| celula(&skin, q)).collect();
        let r = recorte(&celulas, skin.ancoras.celula);
        let t = folha(&skin, &celulas, r, 2, &HashMap::new(), &"ab".repeat(32));
        assert!(t.w > 0 && t.h > 0);
        assert!(t.rgba.chunks_exact(4).any(|p| p == ESCURO));
        assert!(t.rgba.chunks_exact(4).any(|p| p == CLARO));
        let g = gif_do_repouso(&skin, &celulas, r, 1).unwrap();
        let mut leitor = gif::DecodeOptions::new().read_info(g.as_slice()).unwrap();
        let mut total = 0u32;
        while let Some(q) = leitor.read_next_frame().unwrap() {
            total += q.delay as u32;
        }
        assert_eq!(
            total * 10,
            REPOUSO_MS as u32,
            "o repouso.gif cobre o tempo pedido"
        );
        let g = gif_da_tag(&skin, &celulas, 0, r, 1).unwrap();
        let mut leitor = gif::DecodeOptions::new().read_info(g.as_slice()).unwrap();
        let (w, _) = tamanho_gif(r, 1);
        assert_eq!(leitor.width(), w, "escuro e claro lado a lado");
        let q = leitor.read_next_frame().unwrap().unwrap();
        assert_eq!(q.width, w);
        let paleta = leitor.global_palette().unwrap().to_vec();
        let tem = |c: [u8; 4]| paleta.chunks_exact(3).any(|p| p == &c[..3]);
        assert!(tem(ESCURO) && tem(CLARO), "os dois fundos no GIF");
    }

    #[test]
    fn previa_de_pack_nao_sai_para_pasta_do_git() {
        let pasta = std::env::temp_dir().join(format!("claude-pet-contato-{}", std::process::id()));
        let skin = pasta.join("pack");
        fs::create_dir_all(&skin).unwrap();
        let teste = crate::raiz().join("skins/_teste");
        for nome in ["sheet.png", "sheet.json"] {
            fs::copy(teste.join(nome), skin.join(nome)).unwrap();
        }
        let json = fs::read_to_string(teste.join("skin.json"))
            .unwrap()
            .replace("\"redistribuivel\": true", "\"redistribuivel\": false")
            .replace("\"_teste\"", "\"pack\"");
        fs::write(skin.join("skin.json"), json).unwrap();
        let docs = crate::raiz().join("docs/fotos/m2-proibido");
        let args = |extra: &[&str]| -> Vec<String> {
            std::iter::once(skin.to_string_lossy().into_owned())
                .chain(extra.iter().map(|s| (*s).to_owned()))
                .collect()
        };
        let erro = executar(&args(&["--saida", &docs.to_string_lossy()])).unwrap_err();
        assert!(erro.contains("skins-locais/ ou tmp/"), "{erro}");
        assert!(!docs.exists(), "nada foi escrito em docs/");
        // Fora do repositório pode, e a folha sai com a impressão digital.
        let fora = pasta.join("saida");
        executar(&args(&[
            "--saida",
            &fora.to_string_lossy(),
            "--escala",
            "1",
        ]))
        .unwrap();
        let sha = fs::read_to_string(fora.join("contato.sha256")).unwrap();
        assert_eq!(
            sha.trim(),
            pet_core::aprovacao::impressao_da_pasta(&skin).unwrap()
        );
        let _ = fs::remove_dir_all(&pasta);
    }
}
