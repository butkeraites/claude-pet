//! `cargo xtask skin-importar`: traz um pack de arte para o formato de skin
//! do pet (PLANO.md, "Zeca: arte e skin", item 2).
//!
//! Duas entradas:
//! - **`.aseprite`** (a preferida), lido pelo crate `asefile`: as camadas
//!   visíveis achatadas, a duração de cada quadro e as tags com direção;
//! - **tiras PNG**: uma linha da imagem por animação, em células de tamanho
//!   fixo, com os quadros da esquerda para a direita até a primeira célula
//!   vazia. As tiras não guardam duração nem nome: todo quadro ganha
//!   `--duracao` (100 ms por padrão, o do pack) e os nomes vêm de `--nomes`,
//!   de uma tabela conhecida (`--tabela cute-parrots`) ou viram `linha_00`,
//!   `linha_01`…
//!
//! Se o `asefile` não ler o arquivo (versão nova do formato, recurso sem
//! suporte, arquivo truncado — inclusive se ele entrar em pânico) e houver
//! um PNG irmão (`Parrot.aseprite` → `Parrot.png`), o importador cai para as
//! tiras e diz por quê.
//!
//! Os nomes das tags são **normalizados** para ids sem espaço nem parêntese
//! (`Sit(Idle)` → `sit_idle`, `End Dive` → `dive_end`); o nome original fica
//! no campo `data` da tag no `sheet.json` e aparece na folha de contato.
//!
//! O `skin.json` sai como **esqueleto**: pés, área de toque e altura do
//! corpo estimados pelo primeiro quadro, `idle` apontando para a primeira
//! tag e `redistribuivel: false` (pack comprado nunca vai para o git,
//! decisão 0011). Revise antes de usar.

use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Component, Path, PathBuf};

use asefile::{AnimationDirection, AsepriteFile};
use pet_core::skin::decodificar_png;
use serde_json::json;

use crate::args::{self, Args};
use crate::folha::{self, QuadroFolha, TagFolha};

/// Duração dos quadros vindos de tiras PNG (o pack Cute Parrots usa 100 ms
/// em todos os quadros).
pub const DURACAO_TIRAS: u32 = 100;

/// Linhas do `Parrot.png` do pack *Cute Parrots!* (exclusiveOlive): uma
/// animação por linha, na ordem das tags do `Parrot.aseprite`. Conferido
/// pixel a pixel contra o `.aseprite` dos três papagaios.
pub const TABELA_CUTE_PARROTS: &[&str] = &[
    "Idle",
    "Walk",
    "Sit(Start)",
    "Sit(Idle)",
    "Sit(End)",
    "Sleep(Start)",
    "Sleep(Idle)",
    "Sleep(End)",
    "Eating",
    "Chirp",
    "Take Off",
    "Fly",
    "Glide",
    "Dive",
    "Dive(Loop)",
    "End Dive",
    "Fly Bite",
    "Fly Hurt",
    "Landing",
    "Hurt",
    "Death",
];

/// Nomes do pack com id escolhido à mão (a lista de animações da página do
/// pack: Sit, Sit Idle, Stand, Sleep, Awake, Dive Start/Loop/End, Bite…).
/// Os demais passam por [`normalizar`].
const NOMES_CONHECIDOS: &[(&str, &str)] = &[
    ("Sit(Start)", "sit"),
    ("Sit(Idle)", "sit_idle"),
    ("Sit(End)", "stand"),
    ("Sleep(Start)", "sleep_start"),
    ("Sleep(Idle)", "sleep"),
    ("Sleep(End)", "awake"),
    ("Dive", "dive_start"),
    ("Dive(Loop)", "dive_loop"),
    ("End Dive", "dive_end"),
    ("Fly Bite", "bite"),
];

/// Id de tag a partir do nome do pack: a tabela acima ou `snake_case` só
/// com `[a-z0-9_]`, até 40 caracteres.
pub fn normalizar(nome: &str) -> String {
    if let Some((_, id)) = NOMES_CONHECIDOS.iter().find(|(n, _)| *n == nome) {
        return (*id).to_owned();
    }
    let mut id = String::new();
    for c in nome.trim().chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
        } else if !id.ends_with('_') && !id.is_empty() {
            id.push('_');
        }
    }
    let id = id.trim_end_matches('_');
    id.chars().take(40).collect()
}

/// De onde vieram os quadros.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origem {
    Aseprite,
    Tiras,
}

/// Um pack importado, ainda em memória.
#[derive(Debug, Clone)]
pub struct Importado {
    pub celula: (u32, u32),
    pub quadros: Vec<QuadroFolha>,
    pub tags: Vec<TagFolha>,
    pub origem: Origem,
    /// Por que caiu para as tiras, quando caiu.
    pub aviso: Option<String>,
    /// Paleta do `.aseprite` (vazia quando veio das tiras): as regras de
    /// estilo travam as cores novas nela.
    pub paleta: Vec<[u8; 4]>,
}

impl Importado {
    #[cfg(test)]
    pub fn tag(&self, nome: &str) -> Option<&TagFolha> {
        self.tags.iter().find(|t| t.nome == nome)
    }
}

/// Garante nomes únicos e não vazios (`nome`, `nome_2`, …).
fn nomes_unicos(tags: &mut [TagFolha]) {
    let mut vistos: Vec<String> = Vec::new();
    for (i, t) in tags.iter_mut().enumerate() {
        if t.nome.is_empty() {
            t.nome = format!("tag_{i}");
        }
        let base = t.nome.clone();
        let mut n = 2;
        while vistos.contains(&t.nome) {
            t.nome = format!("{base}_{n}");
            n += 1;
        }
        vistos.push(t.nome.clone());
    }
}

fn tag_importada(original: &str, de: usize, ate: usize, direcao: &str) -> TagFolha {
    TagFolha {
        nome: normalizar(original),
        original: Some(original.to_owned()),
        de,
        ate,
        direcao: direcao.to_owned(),
    }
}

/// Lê um `.aseprite` com o `asefile`. Um pânico do parser vira erro.
pub fn ler_aseprite(bytes: &[u8]) -> Result<Importado, String> {
    let lido = catch_unwind(AssertUnwindSafe(|| ler_aseprite_direto(bytes)));
    lido.unwrap_or_else(|panico| {
        let motivo = panico
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panico.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_else(|| "pânico sem mensagem".into());
        Err(format!("o asefile entrou em pânico: {motivo}"))
    })
}

fn ler_aseprite_direto(bytes: &[u8]) -> Result<Importado, String> {
    let ase = AsepriteFile::read(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let celula = (ase.width() as u32, ase.height() as u32);
    if ase.num_frames() == 0 {
        return Err("arquivo sem quadros".into());
    }
    let mut quadros = Vec::with_capacity(ase.num_frames() as usize);
    for i in 0..ase.num_frames() {
        let quadro = ase.frame(i);
        let imagem = quadro.image();
        if (imagem.width(), imagem.height()) != celula {
            return Err(format!("quadro {i} com tamanho diferente do sprite"));
        }
        quadros.push(QuadroFolha {
            rgba: imagem.into_raw(),
            duracao_ms: quadro.duration(),
        });
    }
    let mut tags = Vec::with_capacity(ase.num_tags() as usize);
    for t in 0..ase.num_tags() {
        let tag = ase.tag(t);
        let direcao = match tag.animation_direction() {
            AnimationDirection::Forward => "forward",
            AnimationDirection::Reverse => "reverse",
            AnimationDirection::PingPong => "pingpong",
        };
        let (de, ate) = (tag.from_frame() as usize, tag.to_frame() as usize);
        if de > ate || ate >= quadros.len() {
            return Err(format!("tag «{}» fora dos quadros", tag.name()));
        }
        tags.push(tag_importada(tag.name(), de, ate, direcao));
    }
    nomes_unicos(&mut tags);
    let paleta = ase
        .palette()
        .map(|p| {
            (0..p.num_colors())
                .filter_map(|i| p.color(i))
                .map(|c| [c.red(), c.green(), c.blue(), c.alpha()])
                .collect()
        })
        .unwrap_or_default();
    Ok(Importado {
        celula,
        quadros,
        tags,
        origem: Origem::Aseprite,
        aviso: None,
        paleta,
    })
}

/// Lê tiras PNG: uma linha por animação, `nomes[i]` para a linha `i`.
pub fn ler_tiras(
    png: &[u8],
    celula: (u32, u32),
    nomes: &[String],
    duracao_ms: u32,
) -> Result<Importado, String> {
    let imagem = decodificar_png(png).map_err(|e| e.to_string())?;
    let (cw, ch) = (celula.0 as i32, celula.1 as i32);
    if imagem.largura % cw != 0 || imagem.altura % ch != 0 {
        return Err(format!(
            "a imagem tem {}x{}, que não é múltiplo da célula {cw}x{ch}",
            imagem.largura, imagem.altura
        ));
    }
    let (colunas, linhas) = (imagem.largura / cw, imagem.altura / ch);
    let celula_rgba = |coluna: i32, linha: i32| -> Vec<u8> {
        let mut rgba = Vec::with_capacity((cw * ch * 4) as usize);
        for y in 0..ch {
            let inicio = (((linha * ch + y) * imagem.largura + coluna * cw) * 4) as usize;
            rgba.extend_from_slice(&imagem.rgba[inicio..inicio + (cw * 4) as usize]);
        }
        rgba
    };
    let mut quadros = Vec::new();
    let mut tags = Vec::new();
    for linha in 0..linhas {
        let de = quadros.len();
        for coluna in 0..colunas {
            let rgba = celula_rgba(coluna, linha);
            if rgba.chunks_exact(4).all(|p| p[3] == 0) {
                break;
            }
            quadros.push(QuadroFolha { rgba, duracao_ms });
        }
        if quadros.len() == de {
            continue; // linha vazia: não vira tag
        }
        let original = nomes
            .get(linha as usize)
            .cloned()
            .unwrap_or_else(|| format!("linha_{linha:02}"));
        tags.push(tag_importada(&original, de, quadros.len() - 1, "forward"));
    }
    if quadros.is_empty() {
        return Err("as tiras não têm nenhum quadro com pixel opaco".into());
    }
    if !nomes.is_empty() && nomes.len() != tags.len() {
        return Err(format!(
            "{} nomes para {} linhas com quadros: a tabela não é deste pack",
            nomes.len(),
            tags.len()
        ));
    }
    nomes_unicos(&mut tags);
    Ok(Importado {
        celula,
        quadros,
        tags,
        origem: Origem::Tiras,
        aviso: None,
        paleta: Vec::new(),
    })
}

/// Tiras PNG de reserva para um `.aseprite`.
#[derive(Debug, Clone, Copy)]
pub struct Tiras<'a> {
    pub png: &'a [u8],
    pub celula: (u32, u32),
    /// Nome de cada linha, de cima para baixo.
    pub nomes: &'a [String],
}

/// O `.aseprite` e, se ele não ler, as tiras PNG com a tabela dada.
pub fn ler_com_reserva(aseprite: &[u8], tiras: Option<Tiras<'_>>) -> Result<Importado, String> {
    match ler_aseprite(aseprite) {
        Ok(importado) => Ok(importado),
        Err(motivo) => {
            let Some(Tiras { png, celula, nomes }) = tiras else {
                return Err(format!(
                    "o .aseprite não leu ({motivo}) e não há PNG de reserva"
                ));
            };
            let mut importado = ler_tiras(png, celula, nomes, DURACAO_TIRAS).map_err(|e| {
                format!("o .aseprite não leu ({motivo}) e as tiras também não: {e}")
            })?;
            importado.aviso = Some(format!(
                "o .aseprite não leu ({motivo}); usei as tiras PNG com {DURACAO_TIRAS} ms por quadro"
            ));
            Ok(importado)
        }
    }
}

/// Caixa dos pixels opacos de uma célula RGBA: (x, y, w, h).
pub fn caixa_opaca(rgba: &[u8], celula: (u32, u32)) -> Option<(i32, i32, i32, i32)> {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, -1, -1);
    for y in 0..h {
        for x in 0..w {
            if rgba[((y * w + x) * 4 + 3) as usize] != 0 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    (x1 >= 0).then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// Metadados do esqueleto do `skin.json`.
#[derive(Debug, Clone)]
pub struct Metadados {
    pub id: String,
    pub nome: String,
    pub autor: String,
    pub licenca: String,
    pub fonte: Option<String>,
    pub redistribuivel: bool,
}

/// Esqueleto do `skin.json`: pés, toque e corpo pelo primeiro quadro da
/// primeira tag; `idle` aponta para a primeira tag.
pub fn esqueleto_skin(importado: &Importado, meta: &Metadados) -> Result<String, String> {
    let primeira = importado
        .tags
        .first()
        .ok_or("o pack não tem tags: dê nomes com --nomes ou use um .aseprite com tags")?;
    let rgba = &importado.quadros[primeira.de].rgba;
    let (x, y, w, h) = caixa_opaca(rgba, importado.celula)
        .ok_or_else(|| format!("o primeiro quadro de «{}» está vazio", primeira.nome))?;
    let mut skin = json!({
        "formato": pet_core::skin::FORMATO,
        "id": meta.id,
        "nome": meta.nome,
        "autor": meta.autor,
        "licenca": meta.licenca,
        "redistribuivel": meta.redistribuivel,
        "folha": "sheet.png",
        "dados": "sheet.json",
        "celula": [importado.celula.0, importado.celula.1],
        "pe": [x + w / 2, y + h],
        "toque": [x, y, w, h],
        "corpo_px": h,
        "estados": {"idle": [primeira.nome]},
    });
    if let Some(fonte) = &meta.fonte {
        skin["fonte"] = json!(fonte);
    }
    folha::json_bonito(&skin)
}

/// `[a-z0-9_-]{1,40}`, como o `pet_core::skin` exige.
pub fn id_valido(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Resolve `.` e `..` sem tocar no disco (a pasta pode ainda não existir).
fn normalizar_caminho(caminho: &Path) -> PathBuf {
    let mut saida = PathBuf::new();
    for c in caminho.components() {
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

/// Uma skin não redistribuível nunca é gravada em `skins/` (que vai para o
/// git): o lugar dela é `skins-locais/` (decisão 0011).
pub fn conferir_destino(saida: &Path, redistribuivel: bool) -> Result<(), String> {
    let skins = crate::raiz().join("skins");
    let absoluto = if saida.is_absolute() {
        saida.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(saida)
    };
    if !redistribuivel && normalizar_caminho(&absoluto).starts_with(normalizar_caminho(&skins)) {
        return Err(format!(
            "{} fica em skins/, que vai para o git; um pack não redistribuível vai para skins-locais/ (decisão 0011)",
            saida.display()
        ));
    }
    Ok(())
}

pub const USO: &str = "uso: cargo xtask skin-importar (--aseprite <arquivo> | --tiras <png> --celula LxA [--nomes a,b,c | --tabela cute-parrots] [--duracao MS]) --saida <pasta> [--id ID] [--nome NOME] [--autor AUTOR] [--licenca TEXTO] [--fonte URL] [--redistribuivel]";

pub fn executar(lista: &[String]) -> Result<(), String> {
    let a = Args::ler(
        lista,
        &[
            "--aseprite",
            "--tiras",
            "--celula",
            "--nomes",
            "--tabela",
            "--duracao",
            "--saida",
            "--id",
            "--nome",
            "--autor",
            "--licenca",
            "--fonte",
        ],
        &["--redistribuivel"],
    )?;
    let saida = PathBuf::from(a.obrigatorio("--saida")?);
    let redistribuivel = a.bandeira("--redistribuivel");
    conferir_destino(&saida, redistribuivel)?;
    let nomes: Vec<String> = match (a.valor("--nomes"), a.valor("--tabela")) {
        (Some(_), Some(_)) => return Err("use --nomes ou --tabela, não os dois".into()),
        (Some(lista), None) => lista.split(',').map(|s| s.trim().to_owned()).collect(),
        (None, Some("cute-parrots")) => TABELA_CUTE_PARROTS
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        (None, Some(outra)) => {
            return Err(format!(
                "tabela desconhecida: «{outra}» (conheço: cute-parrots)"
            ));
        }
        (None, None) => Vec::new(),
    };
    let duracao = match a.valor("--duracao") {
        Some(d) => d
            .parse::<u32>()
            .ok()
            .filter(|d| (16..=5000).contains(d))
            .ok_or_else(|| format!("--duracao inválida: «{d}» (16 a 5000 ms)"))?,
        None => DURACAO_TIRAS,
    };
    let ler = |caminho: &str| std::fs::read(caminho).map_err(|e| format!("{caminho}: {e}"));
    let importado = match (a.valor("--aseprite"), a.valor("--tiras")) {
        (Some(ase), tiras) => {
            let irmao = Path::new(ase).with_extension("png");
            let png = match tiras {
                Some(t) => Some(ler(t)?),
                None if irmao.is_file() => Some(ler(&irmao.to_string_lossy())?),
                None => None,
            };
            let celula = a.valor("--celula").map(args::tamanho).transpose()?;
            let bytes = ler(ase)?;
            match &png {
                Some(png) => {
                    // Sem --celula, a das tiras é a do .aseprite (se ele ler)
                    // ou 48x48, a do pack Cute Parrots.
                    let celula = celula.unwrap_or((48, 48));
                    ler_com_reserva(
                        &bytes,
                        Some(Tiras {
                            png,
                            celula,
                            nomes: &nomes,
                        }),
                    )?
                }
                None => ler_com_reserva(&bytes, None)?,
            }
        }
        (None, Some(tiras)) => {
            let celula = args::tamanho(a.obrigatorio("--celula")?)?;
            ler_tiras(&ler(tiras)?, celula, &nomes, duracao)?
        }
        (None, None) => return Err("falta --aseprite ou --tiras".into()),
    };
    if let Some(aviso) = &importado.aviso {
        eprintln!("aviso: {aviso}");
    }
    let id = match a.valor("--id") {
        Some(id) => id.to_owned(),
        None => normalizar(
            &saida
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ),
    };
    if !id_valido(&id) {
        return Err(format!("id «{id}» fora de [a-z0-9_-]{{1,40}} (use --id)"));
    }
    let meta = Metadados {
        nome: a.valor("--nome").unwrap_or(&id).to_owned(),
        id,
        autor: a.valor("--autor").unwrap_or("desconhecido").to_owned(),
        licenca: a
            .valor("--licenca")
            .unwrap_or("veja a licença do pack de origem")
            .to_owned(),
        fonte: a.valor("--fonte").map(str::to_owned),
        redistribuivel,
    };
    let folha = folha::montar(
        importado.celula,
        &importado.quadros,
        &importado.tags,
        "claude-pet cargo xtask skin-importar",
    )?;
    let skin_json = esqueleto_skin(&importado, &meta)?;
    let skin = folha::gravar(&saida, &folha, &skin_json)?;
    let origem = match importado.origem {
        Origem::Aseprite => "aseprite",
        Origem::Tiras => "tiras PNG",
    };
    println!(
        "skin «{}» importada ({origem}) em {}: {} quadros ({} células únicas numa folha {}x{}), {} tags",
        skin.id,
        saida.display(),
        skin.quadros.len(),
        folha.celulas,
        folha.largura,
        folha.altura,
        skin.tags.len()
    );
    for t in &importado.tags {
        let original = t.original.as_deref().unwrap_or(&t.nome);
        let duracoes: Vec<String> = importado.quadros[t.de..=t.ate]
            .iter()
            .map(|q| q.duracao_ms.to_string())
            .collect();
        println!(
            "  {:<12} ← {:<14} quadros {:>2}..={:<2} {} ms",
            t.nome,
            original,
            t.de,
            t.ate,
            duracoes.join("/")
        );
    }
    println!("esqueleto do skin.json: revise pe, toque, corpo_px e estados antes de usar");
    Ok(())
}

#[cfg(test)]
pub(crate) mod testes {
    use super::*;
    use pet_core::skin::codificar_png;

    /// Escreve um `.aseprite` RGBA mínimo (uma camada, quadros brutos e
    /// tags), seguindo a especificação do formato. Só para testes: nenhum
    /// arquivo do pack entra no repositório.
    pub(crate) fn aseprite_sintetico(
        celula: (u16, u16),
        quadros: &[(Vec<u8>, u16)],
        tags: &[(&str, u16, u16, u8)],
    ) -> Vec<u8> {
        fn pedaco(tipo: u16, dados: &[u8]) -> Vec<u8> {
            let mut c = Vec::new();
            c.extend_from_slice(&((dados.len() + 6) as u32).to_le_bytes());
            c.extend_from_slice(&tipo.to_le_bytes());
            c.extend_from_slice(dados);
            c
        }
        fn texto(s: &str) -> Vec<u8> {
            let mut t = (s.len() as u16).to_le_bytes().to_vec();
            t.extend_from_slice(s.as_bytes());
            t
        }
        let mut corpo = Vec::new();
        for (i, (rgba, duracao)) in quadros.iter().enumerate() {
            let mut pedacos = Vec::new();
            if i == 0 {
                let mut camada = Vec::new();
                camada.extend_from_slice(&3u16.to_le_bytes()); // visível + editável
                camada.extend_from_slice(&[0; 2 * 5]); // tipo, nível, w, h, mistura
                camada.push(255); // opacidade
                camada.extend_from_slice(&[0; 3]);
                camada.extend_from_slice(&texto("Camada 1"));
                pedacos.push(pedaco(0x2004, &camada));
                let mut t = (tags.len() as u16).to_le_bytes().to_vec();
                t.extend_from_slice(&[0; 8]);
                for (nome, de, ate, direcao) in tags {
                    t.extend_from_slice(&de.to_le_bytes());
                    t.extend_from_slice(&ate.to_le_bytes());
                    t.push(*direcao);
                    t.extend_from_slice(&[0; 2 + 6 + 4]); // repetição, reservado, cor
                    t.extend_from_slice(&texto(nome));
                }
                pedacos.push(pedaco(0x2018, &t));
            }
            let mut cel = Vec::new();
            cel.extend_from_slice(&[0; 2 + 2 + 2]); // camada 0 em (0, 0)
            cel.push(255);
            cel.extend_from_slice(&0u16.to_le_bytes()); // célula bruta
            cel.extend_from_slice(&[0; 7]);
            cel.extend_from_slice(&celula.0.to_le_bytes());
            cel.extend_from_slice(&celula.1.to_le_bytes());
            cel.extend_from_slice(rgba);
            pedacos.push(pedaco(0x2005, &cel));
            let dados: Vec<u8> = pedacos.concat();
            corpo.extend_from_slice(&((dados.len() + 16) as u32).to_le_bytes());
            corpo.extend_from_slice(&0xF1FAu16.to_le_bytes());
            corpo.extend_from_slice(&(pedacos.len() as u16).to_le_bytes());
            corpo.extend_from_slice(&duracao.to_le_bytes());
            corpo.extend_from_slice(&[0; 2]);
            corpo.extend_from_slice(&(pedacos.len() as u32).to_le_bytes());
            corpo.extend_from_slice(&dados);
        }
        let mut arquivo = Vec::new();
        arquivo.extend_from_slice(&((corpo.len() + 128) as u32).to_le_bytes());
        arquivo.extend_from_slice(&0xA5E0u16.to_le_bytes());
        arquivo.extend_from_slice(&(quadros.len() as u16).to_le_bytes());
        arquivo.extend_from_slice(&celula.0.to_le_bytes());
        arquivo.extend_from_slice(&celula.1.to_le_bytes());
        arquivo.extend_from_slice(&32u16.to_le_bytes()); // RGBA
        arquivo.extend_from_slice(&1u32.to_le_bytes());
        arquivo.extend_from_slice(&100u16.to_le_bytes());
        arquivo.extend_from_slice(&[0; 8]);
        arquivo.extend_from_slice(&[0; 4]); // índice transparente + ignorados
        arquivo.extend_from_slice(&0u16.to_le_bytes()); // cores
        arquivo.extend_from_slice(&[1, 1]); // pixel 1:1
        arquivo.extend_from_slice(&[0; 8]); // grade
        arquivo.extend_from_slice(&[0; 84]);
        assert_eq!(arquivo.len(), 128);
        arquivo.extend_from_slice(&corpo);
        arquivo
    }

    /// Célula 4x4 com um bloco opaco de cor `cor` em (1..3, 1..4).
    pub(crate) fn celula_com(cor: u8) -> Vec<u8> {
        let mut rgba = vec![0u8; 4 * 4 * 4];
        for y in 1..4 {
            for x in 1..3 {
                let i = (y * 4 + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[cor, 10, 20, 255]);
            }
        }
        rgba
    }

    #[test]
    fn nomes_do_pack_viram_ids() {
        assert_eq!(normalizar("Idle"), "idle");
        assert_eq!(normalizar("Sit(Idle)"), "sit_idle");
        assert_eq!(normalizar("Sit(End)"), "stand");
        assert_eq!(normalizar("End Dive"), "dive_end");
        assert_eq!(normalizar("Take Off"), "take_off");
        assert_eq!(normalizar("  Fly  Hurt!! "), "fly_hurt");
        assert_eq!(normalizar("((("), "");
        for nome in TABELA_CUTE_PARROTS {
            assert!(id_valido(&normalizar(nome)), "{nome}");
        }
    }

    #[test]
    fn le_aseprite_sintetico_com_duracoes_e_tags() {
        let ase = aseprite_sintetico(
            (4, 4),
            &[
                (celula_com(1), 100),
                (celula_com(2), 250),
                (celula_com(3), 80),
            ],
            &[
                ("Sit(Idle)", 0, 1, 0),
                ("Eating", 1, 2, 2),
                ("Eating", 2, 2, 1),
            ],
        );
        let i = ler_aseprite(&ase).unwrap();
        assert_eq!(i.origem, Origem::Aseprite);
        assert_eq!(i.celula, (4, 4));
        assert_eq!(i.quadros.len(), 3);
        assert_eq!(i.quadros[1].duracao_ms, 250);
        assert_eq!(i.quadros[2].rgba, celula_com(3));
        let nomes: Vec<&str> = i.tags.iter().map(|t| t.nome.as_str()).collect();
        assert_eq!(nomes, ["sit_idle", "eating", "eating_2"]);
        assert_eq!(i.tags[0].original.as_deref(), Some("Sit(Idle)"));
        assert_eq!(i.tags[1].direcao, "pingpong");
        assert_eq!(i.tags[2].direcao, "reverse");
    }

    #[test]
    fn aseprite_quebrado_cai_para_as_tiras() {
        let mut ase = aseprite_sintetico((4, 4), &[(celula_com(1), 100)], &[("A", 0, 0, 0)]);
        ase.truncate(150);
        assert!(ler_aseprite(&ase).is_err());
        assert!(ler_aseprite(b"lixo").is_err());
        // Tiras: duas linhas de 3 colunas; a primeira com 2 quadros.
        let mut rgba = vec![0u8; 12 * 8 * 4];
        let mut pintar = |coluna: usize, linha: usize, cor: u8| {
            let c = celula_com(cor);
            for y in 0..4 {
                for x in 0..4 {
                    let o = (y * 4 + x) * 4;
                    let d = ((linha * 4 + y) * 12 + coluna * 4 + x) * 4;
                    rgba[d..d + 4].copy_from_slice(&c[o..o + 4]);
                }
            }
        };
        pintar(0, 0, 1);
        pintar(1, 0, 2);
        pintar(0, 1, 3);
        let png = codificar_png(12, 8, &rgba).unwrap();
        let nomes = vec!["Idle".to_owned(), "Sit(End)".to_owned()];
        let tiras = Tiras {
            png: &png,
            celula: (4, 4),
            nomes: &nomes,
        };
        let i = ler_com_reserva(&ase, Some(tiras)).unwrap();
        assert_eq!(i.origem, Origem::Tiras);
        assert!(i.aviso.as_deref().unwrap().contains("tiras"));
        assert_eq!(i.quadros.len(), 3);
        assert_eq!(i.tag("idle").map(|t| (t.de, t.ate)), Some((0, 1)));
        assert_eq!(i.tag("stand").map(|t| (t.de, t.ate)), Some((2, 2)));
        assert!(i.quadros.iter().all(|q| q.duracao_ms == DURACAO_TIRAS));
        // Tabela com outro número de linhas: não é este pack.
        let poucos = vec!["Idle".to_owned()];
        assert!(ler_tiras(&png, (4, 4), &poucos, 100).is_err());
        // Sem tabela, as linhas ganham nome.
        let sem_nome = ler_tiras(&png, (4, 4), &[], 100).unwrap();
        assert_eq!(sem_nome.tags[1].nome, "linha_01");
        assert!(ler_com_reserva(&ase, None).is_err());
    }

    #[test]
    fn esqueleto_e_folha_carregam_no_core() {
        let ase = aseprite_sintetico(
            (4, 4),
            &[(celula_com(1), 100), (celula_com(2), 100)],
            &[("Idle", 0, 1, 0)],
        );
        let i = ler_aseprite(&ase).unwrap();
        let meta = Metadados {
            id: "mini".into(),
            nome: "Mini".into(),
            autor: "testes".into(),
            licenca: "MIT".into(),
            fonte: Some("https://exemplo".into()),
            redistribuivel: false,
        };
        let skin = esqueleto_skin(&i, &meta).unwrap();
        let folha = folha::montar(i.celula, &i.quadros, &i.tags, "t").unwrap();
        let s = pet_core::skin::Skin::de_partes(&skin, &folha.json, &folha.png).unwrap();
        assert_eq!(s.ancoras.toque, pet_core::geometria::Ret::novo(1, 1, 2, 3));
        assert_eq!(s.ancoras.pe, (2, 4));
        assert_eq!(s.corpo_px, 3);
        assert!(!s.redistribuivel);
        assert_eq!(s.fonte.as_deref(), Some("https://exemplo"));
        assert_eq!(s.tags_do_estado("idle"), vec![0]);
    }

    #[test]
    fn pack_nao_redistribuivel_nunca_vai_para_skins() {
        let raiz = crate::raiz();
        assert!(conferir_destino(&raiz.join("skins/zeca"), false).is_err());
        assert!(conferir_destino(&raiz.join("skins/../skins/zeca"), false).is_err());
        assert!(conferir_destino(&raiz.join("skins-locais/../skins/zeca"), false).is_err());
        assert!(conferir_destino(&raiz.join("skins-locais/zeca"), false).is_ok());
        assert!(conferir_destino(&raiz.join("skins/minha"), true).is_ok());
    }
}
