//! `cargo xtask zeca`: o Zeca a partir do pack *Cute Parrots!* (PLANO.md,
//! "Zeca: arte e skin", item 3; decisões 0023 a 0025).
//!
//! 1. Lê o Parrot 2 (o papagaio verde) do zip ou da pasta do pack, pelo
//!    importador (`.aseprite`, com as tiras PNG de reserva).
//! 2. Aplica a troca de paleta de `arte/zeca/paleta.toml` (vazia no visual
//!    "Malandro rosa": o bico rosa original fica).
//! 3. Veste cada quadro: chapéu-palheta e gravata-borboleta pela regra do
//!    olho, com as correções de `arte/zeca/ancoras.json`; nos quadros de
//!    clarão os acessórios ficam brancos.
//! 4. Troca os quadros das tags de `arte/zeca/chapeu_voando.json` ("chapéu
//!    voa e volta" no mergulho e no susto) e monta as tags compostas de
//!    `arte/zeca/zeca.toml`.
//! 5. Opcional (`--contorno`): contorno creme de 1 pixel de arte, como a
//!    skin `zeca-contorno`.
//! 6. Grava `sheet.png`, `sheet.json`, `skin.json` e `CREDITS.md` em
//!    `skins-locais/<id>/` (fora do git: derivado do pack, decisão 0011).
//!
//! A saída é determinística: mesmo pack e mesma arte, mesmos bytes.

pub mod arte;
pub mod compor;
pub mod pack;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use pet_core::geometria;
use serde_json::json;

use crate::args::Args;
use crate::folha::{self, QuadroFolha, TagFolha};
use crate::importar::{self, Importado, TABELA_CUTE_PARROTS, Tiras};
use arte::{Ajuste, Arte};
use compor::{Analise, Protegidas, Relatorio, Vestido};

pub const USO: &str = "uso: cargo xtask zeca --pack <zip|pasta> [--contorno] [--saida <pasta>] [--arte <pasta>] [--ancoras]";

/// Um quadro da folha do Zeca: o corpo (quadro do pack) e o que vai por cima.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuadroZeca {
    pub corpo: usize,
    pub ms: u32,
    pub vestido: Vestido,
    /// O chapéu está solto no ar (chapéu voando), não assentado.
    pub solto: bool,
}

#[derive(Debug, Clone)]
pub struct TagZeca {
    pub nome: String,
    pub original: Option<String>,
    pub quadros: Vec<QuadroZeca>,
}

/// O Zeca montado, ainda em memória.
#[derive(Debug, Clone)]
pub struct Montado {
    pub celula: (u32, u32),
    pub tags: Vec<TagZeca>,
    pub quadros: Vec<QuadroFolha>,
    pub tags_folha: Vec<TagFolha>,
    pub skin_json: String,
    pub toque: [i32; 4],
    pub corpo_px: u32,
    pub avisos: Vec<String>,
    /// Uma linha por quadro do pack: olho, clarão e encaixe (`--ancoras`).
    pub diagnostico: Vec<String>,
}

/// Qual variante sai: a sem contorno (`zeca`) ou a com contorno creme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variante {
    Simples,
    Contorno,
}

impl Variante {
    pub fn id(self, base: &str) -> String {
        match self {
            Variante::Simples => base.to_owned(),
            Variante::Contorno => format!("{base}-contorno"),
        }
    }
}

/// Onde cada quadro do pack está: (tag, índice dentro dela). Um quadro em
/// várias tags fica com a primeira.
fn posicoes(importado: &Importado) -> Vec<Option<(String, usize)>> {
    let mut posicao = vec![None; importado.quadros.len()];
    for t in &importado.tags {
        for (i, q) in (t.de..=t.ate).enumerate() {
            if posicao[q].is_none() {
                posicao[q] = Some((t.nome.clone(), i));
            }
        }
    }
    posicao
}

fn aplicar(base: Option<compor::Colocacao>, ajuste: Option<&Ajuste>) -> Option<compor::Colocacao> {
    match ajuste {
        Some(a) => compor::ajustar(base, a),
        None => base,
    }
}

/// O vestido de cada quadro do pack: regra do olho + `ancoras.json`.
fn vestir_pack(
    importado: &Importado,
    analises: &[Analise],
    fontes: &[Option<usize>],
    arte: &Arte,
    avisos: &mut Vec<String>,
) -> Vec<Vestido> {
    let posicao = posicoes(importado);
    let regra = &arte.ancoras.regra;
    (0..importado.quadros.len())
        .map(|b| {
            let a = analises[b];
            let olho = if a.clarao {
                fontes[b].and_then(|j| analises[j].olho)
            } else {
                a.olho
            };
            let mut chapeu = compor::pela_regra(olho, &regra.chapeu);
            let mut gravata = compor::pela_regra(olho, &regra.gravata);
            if let Some((tag, i)) = &posicao[b] {
                if olho.is_none() && !arte.zeca.skin.excluir.contains(tag) {
                    avisos.push(format!(
                        "{tag}[{i}]: sem olho {}; só âncora manual",
                        if a.clarao {
                            "nem silhueta igual"
                        } else {
                            "detectado"
                        }
                    ));
                }
                if let Some(at) = arte.ancoras.tags.get(tag) {
                    chapeu = aplicar(chapeu, at.chapeu.as_ref());
                    gravata = aplicar(gravata, at.gravata.as_ref());
                    if let Some(q) = at.quadros.get(&i.to_string()) {
                        chapeu = aplicar(chapeu, q.chapeu.as_ref());
                        gravata = aplicar(gravata, q.gravata.as_ref());
                    }
                }
            }
            Vestido {
                chapeu,
                gravata,
                branco: a.clarao,
            }
        })
        .collect()
}

/// As tags do Zeca: as do pack (menos as excluídas), as do chapéu voando no
/// lugar das originais e as compostas no fim.
fn roteiro(
    importado: &Importado,
    vestidos: &[Vestido],
    arte: &Arte,
) -> Result<Vec<TagZeca>, String> {
    for nome in arte.voos.tags.keys() {
        if importado.tags.iter().all(|t| &t.nome != nome) {
            return Err(format!(
                "chapeu_voando.json: a tag «{nome}» não existe no pack"
            ));
        }
    }
    for nome in arte.ancoras.tags.keys() {
        if importado.tags.iter().all(|t| &t.nome != nome) {
            return Err(format!("ancoras.json: a tag «{nome}» não existe no pack"));
        }
    }
    let mut tags = Vec::new();
    for t in &importado.tags {
        if arte.zeca.skin.excluir.contains(&t.nome) {
            continue;
        }
        let quadros = match arte.voos.tags.get(&t.nome) {
            Some(voo) => voo
                .quadros
                .iter()
                .enumerate()
                .map(|(i, q)| {
                    let corpo = t.de + q.corpo;
                    if corpo > t.ate {
                        return Err(format!(
                            "chapeu_voando.json, {}[{i}]: corpo {} além dos {} quadros da tag",
                            t.nome,
                            q.corpo,
                            t.ate - t.de + 1
                        ));
                    }
                    let base = &vestidos[corpo];
                    Ok(QuadroZeca {
                        corpo,
                        ms: q.ms,
                        solto: !q.chapeu.assentado,
                        vestido: Vestido {
                            chapeu: compor::chapeu_do_voo(&q.chapeu, base.chapeu.as_ref()),
                            ..base.clone()
                        },
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            None => (t.de..=t.ate)
                .map(|corpo| QuadroZeca {
                    corpo,
                    ms: importado.quadros[corpo].duracao_ms,
                    vestido: vestidos[corpo].clone(),
                    solto: false,
                })
                .collect(),
        };
        tags.push(TagZeca {
            nome: t.nome.clone(),
            original: t.original.clone(),
            quadros,
        });
    }
    for c in &arte.zeca.composicao {
        if tags.iter().any(|t| t.nome == c.nome) {
            return Err(format!(
                "zeca.toml: a composição «{}» repete um nome de tag",
                c.nome
            ));
        }
        let mut quadros = Vec::new();
        for parte in &c.partes {
            let tag = tags.iter().find(|t| t.nome == parte.tag).ok_or_else(|| {
                format!("zeca.toml, «{}»: a tag «{}» não existe", c.nome, parte.tag)
            })?;
            for _ in 0..parte.vezes.max(1) {
                quadros.extend(tag.quadros.iter().cloned());
            }
        }
        tags.push(TagZeca {
            nome: c.nome.clone(),
            original: None,
            quadros,
        });
    }
    Ok(tags)
}

fn caixa(rgba: &[u8], celula: (u32, u32)) -> Option<[i32; 4]> {
    importar::caixa_opaca(rgba, celula).map(|(x, y, w, h)| [x, y, w, h])
}

/// Monta o Zeca em memória.
pub fn montar(importado: &Importado, arte: &Arte, variante: Variante) -> Result<Montado, String> {
    let celula = importado.celula;
    let mut avisos = Vec::new();
    if let Some(aviso) = &importado.aviso {
        avisos.push(aviso.clone());
    }
    // Troca de paleta no corpo (visual 1: nenhuma).
    let corpos: Vec<Vec<u8>> = importado
        .quadros
        .iter()
        .map(|q| {
            let mut rgba = q.rgba.clone();
            for p in rgba.chunks_exact_mut(4) {
                if let Some((_, para)) = arte
                    .paleta
                    .troca
                    .iter()
                    .find(|(de, _)| p[3] != 0 && p[..3] == de[..3])
                {
                    p[..3].copy_from_slice(&para[..3]);
                }
            }
            rgba
        })
        .collect();
    let analises: Vec<Analise> = corpos
        .iter()
        .map(|c| compor::analisar(c, celula, &arte.paleta.olho, arte.paleta.clarao))
        .collect();
    let refs: Vec<&[u8]> = corpos.iter().map(Vec::as_slice).collect();
    let fontes = compor::fontes_de_clarao(&refs, &analises);
    let vestidos = vestir_pack(importado, &analises, &fontes, arte, &mut avisos);
    let tags = roteiro(importado, &vestidos, arte)?;
    let posicao = posicoes(importado);
    let pos = |c: &Option<compor::Colocacao>| match c {
        Some(c) => format!("{} ({}, {})", c.variante, c.x, c.y),
        None => "—".into(),
    };
    let diagnostico = (0..importado.quadros.len())
        .map(|b| {
            let onde = posicao[b]
                .as_ref()
                .map(|(t, i)| format!("{t}[{i}]"))
                .unwrap_or_else(|| "(sem tag)".into());
            let olho = match (analises[b].olho, fontes[b]) {
                (Some(o), _) => format!("olho {}..={} x {}..={}", o.x0, o.x1, o.y0, o.y1),
                (None, Some(j)) => format!("clarão com a silhueta do quadro {j}"),
                (None, None) => "sem olho".into(),
            };
            let caixa = importar::caixa_opaca(&corpos[b], celula)
                .map(|(x, y, w, h)| format!("corpo {x},{y} {w}x{h}"))
                .unwrap_or_default();
            format!(
                "{b:>3} {onde:<16} {olho:<28} {caixa:<18} chapéu {:<24} gravata {}",
                pos(&vestidos[b].chapeu),
                pos(&vestidos[b].gravata)
            )
        })
        .collect();

    // Paleta travada no pack (regras de estilo): toda cor nossa existe na
    // paleta do .aseprite.
    if importado.paleta.is_empty() {
        avisos.push("paleta do pack indisponível (tiras PNG): conferência de cores pulada".into());
    } else {
        let mut nossas: Vec<[u8; 4]> = arte.paleta.letras.values().copied().collect();
        nossas.push(arte.paleta.contorno);
        nossas.extend(arte.paleta.troca.iter().map(|(_, para)| *para));
        for c in nossas {
            if !importado.paleta.iter().any(|p| p[..3] == c[..3]) {
                avisos.push(format!("cor {} fora da paleta do pack", arte::hex(c)));
            }
        }
    }

    let protegidas = Protegidas {
        olho: &arte.paleta.olho,
        bico: &arte.paleta.bico,
    };
    let mut cache: HashMap<(usize, Vestido), Vec<u8>> = HashMap::new();
    let mut quadros = Vec::new();
    let mut tags_folha = Vec::new();
    for t in &tags {
        let de = quadros.len();
        for (i, q) in t.quadros.iter().enumerate() {
            let chave = (q.corpo, q.vestido.clone());
            let rgba = match cache.get(&chave) {
                Some(rgba) => rgba.clone(),
                None => {
                    let (mut rgba, r) = compor::vestir(
                        &corpos[q.corpo],
                        celula,
                        &q.vestido,
                        &arte.sprites,
                        arte.paleta.clarao,
                        &protegidas,
                    )?;
                    let Relatorio {
                        fora,
                        sobre_olho,
                        sobre_bico,
                    } = r;
                    for (n, o_que) in [
                        (fora, "fora da célula"),
                        (sobre_olho, "sobre o olho"),
                        (sobre_bico, "sobre o bico"),
                    ] {
                        if n > 0 {
                            avisos.push(format!(
                                "{}[{i}]: {n} pixel(s) de acessório {o_que}",
                                t.nome
                            ));
                        }
                    }
                    if let (true, Some(chapeu)) = (q.solto, &q.vestido.chapeu)
                        && let Some(sprite) = arte.sprites.get(&chapeu.variante)
                        && let Some(d) = compor::distancia(&corpos[q.corpo], celula, chapeu, sprite)
                        && d < compor::FOLGA_SOLTO
                    {
                        avisos.push(format!(
                            "{}[{i}]: chapéu solto a {d} px do corpo (com menos de {} o contorno creme junta os dois)",
                            t.nome,
                            compor::FOLGA_SOLTO
                        ));
                    }
                    if variante == Variante::Contorno {
                        compor::contornar(&mut rgba, celula, arte.paleta.contorno);
                    }
                    cache.insert(chave, rgba.clone());
                    rgba
                }
            };
            quadros.push(QuadroFolha {
                rgba,
                duracao_ms: q.ms,
            });
        }
        tags_folha.push(TagFolha {
            nome: t.nome.clone(),
            original: t.original.clone(),
            de,
            ate: quadros.len() - 1,
            direcao: "forward".into(),
        });
    }

    let z = &arte.zeca;
    for (estado, nomes) in &z.estados {
        for nome in nomes {
            if tags.iter().all(|t| &t.nome != nome) {
                return Err(format!(
                    "zeca.toml, estado «{estado}»: a tag «{nome}» não existe"
                ));
            }
        }
    }
    for nome in &z.skin.chao {
        if tags.iter().all(|t| &t.nome != nome) {
            return Err(format!("zeca.toml, chao: a tag «{nome}» não existe"));
        }
    }
    // Toque e corpo pela pose parada vestida, sem contorno: as duas
    // variantes ficam do mesmo tamanho na tela.
    let pose = z
        .estados
        .get("idle")
        .and_then(|n| n.first())
        .and_then(|nome| tags.iter().find(|t| &t.nome == nome))
        .and_then(|t| t.quadros.first())
        .ok_or("zeca.toml: falta o estado «idle»")?;
    let (pose_rgba, _) = compor::vestir(
        &corpos[pose.corpo],
        celula,
        &pose.vestido,
        &arte.sprites,
        arte.paleta.clarao,
        &protegidas,
    )?;
    let caixa_pose = caixa(&pose_rgba, celula).ok_or("a pose parada está vazia")?;
    let toque = z.skin.toque.unwrap_or(caixa_pose);
    let corpo_px = z.skin.corpo_px.unwrap_or(caixa_pose[3] as u32);
    let id = variante.id(&z.skin.id);
    let nome = match variante {
        Variante::Simples => z.skin.nome.clone(),
        Variante::Contorno => format!("{} (contorno creme)", z.skin.nome),
    };
    let skin = json!({
        "formato": pet_core::skin::FORMATO,
        "id": id,
        "nome": nome,
        "autor": z.skin.autor,
        "licenca": z.skin.licenca,
        "fonte": z.skin.fonte,
        "redistribuivel": false,
        "folha": "sheet.png",
        "dados": "sheet.json",
        "celula": [celula.0, celula.1],
        // Com contorno, a linha creme debaixo dos pés vira o chão.
        "pe": match variante {
            Variante::Simples => z.skin.pe,
            Variante::Contorno => [z.skin.pe[0], z.skin.pe[1] + 1],
        },
        "toque": toque,
        "corpo_px": corpo_px,
        "escala_padrao": z.skin.escala_padrao,
        "estados": z.estados,
        "chao": z.skin.chao,
    });
    Ok(Montado {
        celula,
        tags,
        quadros,
        tags_folha,
        skin_json: folha::json_bonito(&skin)?,
        toque,
        corpo_px,
        avisos,
        diagnostico,
    })
}

/// `CREDITS.md` da skin: de quem é a arte e por que não pode sair daqui.
pub fn creditos(id: &str) -> String {
    format!(
        "# Créditos da skin «{id}»\n\
         \n\
         - **Corpo e animações:** *Cute Parrots! — Pixel Art Asset Pack* (Parrot 2), por\n  \
           **exclusiveOlive** — https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack\n  \
           Licença do autor: pode ser usado em projetos comerciais e não comerciais e pode ser\n  \
           editado; **não pode ser redistribuído nem revendido, mesmo editado**.\n\
         - **Chapéu-palheta, gravata-borboleta, chapéu voando, encaixe e contorno:** claude-pet\n  \
           (`arte/zeca/` no repositório, licença MIT).\n\
         \n\
         **Estes arquivos são derivados do pack e não podem ser redistribuídos.** Eles ficam em\n\
         `skins-locais/` (fora do git) e só entram na imagem Docker local, que nunca vai a\n\
         registry nenhum (decisão 0011). Quem quiser o Zeca compra o próprio pack e roda\n\
         `bin/pet skin-instalar <zip>`.\n\
         \n\
         Obrigado, exclusiveOlive!\n"
    )
}

/// D (pixels do monitor por pixel de arte) nos dois monitores do Renan.
fn d_nos_monitores(corpo_px: u32) -> String {
    format!(
        "D={} no eDP-1 (800 lógicos a 1.5) e D={} no 4K (1440 lógicos a 1.5)",
        geometria::calcular_d(800, 1.5, corpo_px),
        geometria::calcular_d(1440, 1.5, corpo_px)
    )
}

pub fn executar(lista: &[String]) -> Result<(), String> {
    let a = Args::ler(
        lista,
        &["--pack", "--saida", "--arte"],
        &["--contorno", "--ancoras"],
    )?;
    let pack_caminho = PathBuf::from(a.obrigatorio("--pack")?);
    let variante = if a.bandeira("--contorno") {
        Variante::Contorno
    } else {
        Variante::Simples
    };
    let pasta_arte = a
        .valor("--arte")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::raiz().join("arte/zeca"));
    let arte = Arte::carregar(&pasta_arte)?;
    let id = variante.id(&arte.zeca.skin.id);
    let saida = a
        .valor("--saida")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::raiz().join("skins-locais").join(&id));
    importar::conferir_destino(&saida, false)?;
    let p2 = pack::parrot2(&pack_caminho)?;
    let nomes: Vec<String> = TABELA_CUTE_PARROTS
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let tiras = p2.tiras.as_deref().map(|png| Tiras {
        png,
        celula: (48, 48),
        nomes: &nomes,
    });
    let importado = importar::ler_com_reserva(&p2.aseprite, tiras)?;
    let m = montar(&importado, &arte, variante)?;
    let f = folha::montar(
        m.celula,
        &m.quadros,
        &m.tags_folha,
        "claude-pet cargo xtask zeca",
    )?;
    let skin = folha::gravar(&saida, &f, &m.skin_json)?;
    fs::write(saida.join("CREDITS.md"), creditos(&id))
        .map_err(|e| format!("{}: {e}", saida.join("CREDITS.md").display()))?;
    println!("Parrot 2 lido de {}", p2.onde);
    println!(
        "skin «{}» em {}: {} quadros ({} células únicas), {} tags, toque {:?}, corpo_px {} ({})",
        skin.id,
        saida.display(),
        skin.quadros.len(),
        f.celulas,
        skin.tags.len(),
        m.toque,
        m.corpo_px,
        d_nos_monitores(m.corpo_px)
    );
    for t in &m.tags {
        let total: u32 = t.quadros.iter().map(|q| q.ms).sum();
        let origem = match &t.original {
            Some(o) => format!(" ← {o}"),
            None => arte
                .zeca
                .composicao
                .iter()
                .find(|c| c.nome == t.nome)
                .and_then(|c| c.nota.as_deref())
                .map(|n| format!(" (composta: {n})"))
                .unwrap_or_default(),
        };
        println!(
            "  {:<16} {:>2} quadros, {:>5} ms{origem}",
            t.nome,
            t.quadros.len(),
            total
        );
    }
    if a.bandeira("--ancoras") {
        println!("encaixe por quadro do pack (antes do chapéu voando):");
        for linha in &m.diagnostico {
            println!("  {linha}");
        }
    }
    for aviso in m.avisos.iter().chain(&skin.avisos) {
        println!("aviso: {aviso}");
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeMap;

    use super::*;
    use crate::importar::testes::aseprite_sintetico;

    /// Um "papagaio" sintético 48x48: cabeça verde com olho branco 3x3 e
    /// bico rosa, no lugar do pack (que nunca entra no repositório).
    fn papagaio(dx: i32, dy: i32, flash: bool) -> Vec<u8> {
        let mut rgba = vec![0u8; 48 * 48 * 4];
        let mut por = |x: i32, y: i32, c: [u8; 4]| {
            let i = (((y + dy) * 48 + x + dx) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&c);
        };
        for y in 15..32 {
            for x in 18..32 {
                por(x, y, [0x9C, 0xC9, 0x70, 255]);
            }
        }
        for y in 17..20 {
            for x in 25..28 {
                por(x, y, [0xF5, 0xF7, 0xF4, 255]);
            }
        }
        por(32, 19, [0xE1, 0x53, 0x6F, 255]);
        if flash {
            for p in rgba.chunks_exact_mut(4).filter(|p| p[3] != 0) {
                p.copy_from_slice(&[0xF5, 0xF7, 0xF4, 255]);
            }
        }
        rgba
    }

    fn importado_sintetico() -> Importado {
        let ase = aseprite_sintetico(
            (48, 48),
            &[
                (papagaio(0, 0, false), 100),
                (papagaio(0, 1, false), 100),
                (papagaio(0, 0, true), 100),
                (papagaio(0, 1, false), 100),
                (papagaio(2, 0, false), 100),
            ],
            &[
                ("Sit(Idle)", 0, 1, 0),
                ("Hurt", 2, 3, 0),
                ("Death", 4, 4, 0),
            ],
        );
        importar::ler_aseprite(&ase).unwrap()
    }

    fn arte_de_teste() -> Arte {
        let mut arte = Arte::carregar(&crate::raiz().join("arte/zeca")).unwrap();
        arte.ancoras.tags.clear();
        arte.voos.tags.clear();
        arte.zeca.composicao.clear();
        arte.zeca.estados = BTreeMap::from([("idle".to_owned(), vec!["sit_idle".to_owned()])]);
        arte.zeca.skin.chao = vec!["sit_idle".into()];
        arte.zeca.skin.toque = None;
        arte.zeca.skin.corpo_px = None;
        arte
    }

    #[test]
    fn chapeu_e_gravata_seguem_o_olho_e_o_clarao_fica_branco() {
        let i = importado_sintetico();
        let arte = arte_de_teste();
        let m = montar(&i, &arte, Variante::Simples).unwrap();
        let nomes: Vec<&str> = m.tags.iter().map(|t| t.nome.as_str()).collect();
        assert_eq!(nomes, ["sit_idle", "hurt"], "death fica de fora");
        let regra = &arte.ancoras.regra;
        let q0 = &m.tags[0].quadros[0].vestido;
        let chapeu = q0.chapeu.as_ref().unwrap();
        assert_eq!(
            (chapeu.x, chapeu.y),
            (25 + regra.chapeu.dx, 17 + regra.chapeu.dy)
        );
        let q1 = &m.tags[0].quadros[1].vestido;
        assert_eq!(
            q1.chapeu.as_ref().unwrap().y,
            chapeu.y + 1,
            "o chapéu desce com a cabeça"
        );
        let clarao = &m.tags[1].quadros[0].vestido;
        assert!(clarao.branco);
        assert_eq!(
            clarao.chapeu, q0.chapeu,
            "a âncora vem do quadro com a mesma silhueta"
        );
        // No clarão, todo pixel opaco é o branco do pack.
        assert!(
            m.quadros[2]
                .rgba
                .chunks_exact(4)
                .filter(|p| p[3] != 0)
                .all(|p| p == [0xF5, 0xF7, 0xF4, 255])
        );
        let skin = pet_core::skin::Skin::de_partes(
            &m.skin_json,
            &folha::montar(m.celula, &m.quadros, &m.tags_folha, "t")
                .unwrap()
                .json,
            &folha::montar(m.celula, &m.quadros, &m.tags_folha, "t")
                .unwrap()
                .png,
        )
        .unwrap();
        assert!(!skin.redistribuivel);
        assert_eq!(skin.id, "zeca");
        assert_eq!(skin.ancoras.toque.y, chapeu.y, "o toque inclui o chapéu");
    }

    #[test]
    fn ancoras_e_voo_corrigem_o_encaixe() {
        let i = importado_sintetico();
        let mut arte = arte_de_teste();
        arte.ancoras.tags.insert(
            "sit_idle".into(),
            arte::AncorasTag {
                gravata: Some(Ajuste {
                    oculto: true,
                    ..Ajuste::default()
                }),
                quadros: BTreeMap::from([(
                    "1".to_owned(),
                    arte::AjustesQuadro {
                        chapeu: Some(Ajuste {
                            dx: Some(2),
                            ..Ajuste::default()
                        }),
                        ..arte::AjustesQuadro::default()
                    },
                )]),
                ..arte::AncorasTag::default()
            },
        );
        arte.voos.tags.insert(
            "hurt".into(),
            arte::Voo {
                quadros: vec![
                    arte::QuadroVoo {
                        corpo: 0,
                        ms: 80,
                        chapeu: arte::ChapeuVoo {
                            variante: Some("chapeu".into()),
                            x: Some(20),
                            y: Some(2),
                            ..arte::ChapeuVoo::default()
                        },
                        _nota: None,
                    },
                    arte::QuadroVoo {
                        corpo: 1,
                        ms: 300,
                        chapeu: arte::ChapeuVoo {
                            assentado: true,
                            ..arte::ChapeuVoo::default()
                        },
                        _nota: None,
                    },
                ],
                _nota: None,
            },
        );
        let m = montar(&i, &arte, Variante::Contorno).unwrap();
        assert!(
            m.tags[0]
                .quadros
                .iter()
                .all(|q| q.vestido.gravata.is_none())
        );
        let c0 = m.tags[0].quadros[0].vestido.chapeu.clone().unwrap();
        let c1 = m.tags[0].quadros[1].vestido.chapeu.clone().unwrap();
        assert_eq!(c1.x, c0.x + 2);
        let hurt = &m.tags[1].quadros;
        assert_eq!(hurt[0].ms, 80);
        assert_eq!(
            hurt[0].vestido.chapeu.as_ref().map(|c| (c.x, c.y)),
            Some((20, 2))
        );
        assert!(
            hurt[0].vestido.branco,
            "o chapéu solto também fica branco no clarão"
        );
        assert_eq!(hurt[1].ms, 300);
        assert!(hurt[1].vestido.chapeu.is_some(), "volta para a cabeça");
        assert!(m.skin_json.contains("\"zeca-contorno\""));
        // Corpo de voo além da tag é erro.
        arte.voos.tags.get_mut("hurt").unwrap().quadros[0].corpo = 7;
        assert!(montar(&i, &arte, Variante::Simples).is_err());
    }

    #[test]
    fn creditos_falam_da_licenca() {
        let c = creditos("zeca");
        assert!(c.contains("exclusiveOlive"));
        assert!(c.contains("não pode ser redistribuído"));
    }
}
