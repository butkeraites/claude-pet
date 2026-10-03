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

pub const USO: &str = "uso: cargo xtask zeca --pack <zip|pasta> [--contorno] [--saida <pasta>] [--arte <pasta>] [--ancoras] [--estrito]";

/// Distância mínima do chapéu solto à borda da célula (o contorno creme de
/// 1 pixel cabe e ainda sobra 1).
pub const FOLGA_BORDA: i32 = 2;
/// Maior salto do centro do chapéu solto entre dois quadros seguidos.
pub const SALTO_SOLTO: i32 = 3;

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

fn aplicar(
    base: Option<compor::Colocacao>,
    ajuste: Option<&Ajuste>,
    variante_padrao: &str,
    onde: &str,
) -> Result<Option<compor::Colocacao>, String> {
    match ajuste {
        Some(a) => compor::ajustar(base, a, variante_padrao)
            .map_err(|e| format!("ancoras.json, {onde}: {e}")),
        None => Ok(base),
    }
}

/// O vestido de cada quadro do pack: regra do olho + `ancoras.json`.
fn vestir_pack(
    importado: &Importado,
    analises: &[Analise],
    fontes: &[Option<usize>],
    arte: &Arte,
    avisos: &mut Vec<String>,
) -> Result<Vec<Vestido>, String> {
    let posicao = posicoes(importado);
    let regra = &arte.ancoras.regra;
    // Toda correção por quadro aponta para um quadro que existe na tag.
    for (tag, at) in &arte.ancoras.tags {
        if let Some(t) = importado.tags.iter().find(|t| &t.nome == tag) {
            let n = t.ate - t.de + 1;
            for indice in at.quadros.keys() {
                if indice.parse::<usize>().is_ok_and(|i| i >= n) {
                    return Err(format!(
                        "ancoras.json, tag {tag}: quadro «{indice}», mas a tag tem {n} quadros (0 a {})",
                        n - 1
                    ));
                }
            }
        }
    }
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
                let excluida = arte.zeca.skin.excluir.contains(tag);
                if olho.is_none() && !excluida {
                    avisos.push(format!(
                        "{tag}[{i}]: sem olho {}; só âncora manual",
                        if a.clarao {
                            "nem silhueta igual"
                        } else {
                            "detectado"
                        }
                    ));
                }
                if let (Some(at), false) = (arte.ancoras.tags.get(tag), excluida) {
                    let (vc, vg) = (&regra.chapeu.variante, &regra.gravata.variante);
                    let onde = format!("{tag}[{i}]");
                    chapeu = aplicar(chapeu, at.chapeu.as_ref(), vc, &onde)?;
                    gravata = aplicar(gravata, at.gravata.as_ref(), vg, &onde)?;
                    if let Some(q) = at.quadros.get(&i.to_string()) {
                        chapeu = aplicar(chapeu, q.chapeu.as_ref(), vc, &onde)?;
                        gravata = aplicar(gravata, q.gravata.as_ref(), vg, &onde)?;
                    }
                }
            }
            Ok(Vestido {
                chapeu,
                gravata,
                branco: a.clarao,
            })
        })
        .collect()
}

/// Quadros de uma tag do pack na ordem em que ela toca (a direção do
/// Aseprite, como `pet_core::animador::sequencia`): as tags do Zeca saem
/// sempre `forward`, com a ordem já expandida.
fn sequencia_do_pack(t: &TagFolha) -> Result<Vec<usize>, String> {
    let ida: Vec<usize> = (t.de..=t.ate).collect();
    let miolo: Vec<usize> = if ida.len() > 2 {
        ida[1..ida.len() - 1].to_vec()
    } else {
        Vec::new()
    };
    Ok(match t.direcao.as_str() {
        "forward" => ida,
        "reverse" => ida.into_iter().rev().collect(),
        "pingpong" => ida.iter().chain(miolo.iter().rev()).copied().collect(),
        "pingpong_reverse" => ida.iter().rev().chain(miolo.iter()).copied().collect(),
        outra => return Err(format!("tag «{}» com direção «{outra}»", t.nome)),
    })
}

/// Os quadros de um voo do chapéu sobre os corpos da tag `t` do pack.
fn quadros_do_voo(
    nome: &str,
    voo: &arte::Voo,
    t: &TagFolha,
    vestidos: &[Vestido],
    arte: &Arte,
) -> Result<Vec<QuadroZeca>, String> {
    voo.quadros
        .iter()
        .enumerate()
        .map(|(i, q)| {
            let corpo = t.de + q.corpo;
            if corpo > t.ate {
                return Err(format!(
                    "chapeu_voando.json, {nome}[{i}]: corpo {} além dos {} quadros de «{}»",
                    q.corpo,
                    t.ate - t.de + 1,
                    t.nome
                ));
            }
            let base = &vestidos[corpo];
            let chapeu = compor::chapeu_do_voo(&q.chapeu, base.chapeu.as_ref(), &arte.sprites)
                .map_err(|e| format!("chapeu_voando.json, {nome}[{i}]: {e}"))?;
            Ok(QuadroZeca {
                corpo,
                ms: q.ms,
                solto: !q.chapeu.assentado,
                vestido: Vestido {
                    chapeu,
                    ..base.clone()
                },
            })
        })
        .collect()
}

/// As tags do Zeca: as do pack (menos as excluídas, na ordem e direção do
/// pack), as do chapéu voando no lugar das originais, as tags novas do
/// chapéu voando (com `base`) e as compostas no fim.
fn roteiro(
    importado: &Importado,
    vestidos: &[Vestido],
    arte: &Arte,
) -> Result<Vec<TagZeca>, String> {
    let do_pack = |nome: &str| importado.tags.iter().find(|t| t.nome == nome);
    for (nome, voo) in &arte.voos.tags {
        match &voo.base {
            None if do_pack(nome).is_none() => {
                return Err(format!(
                    "chapeu_voando.json: a tag «{nome}» não existe no pack (para uma tag nova, dê a «base»)"
                ));
            }
            Some(base) if do_pack(base).is_none() => {
                return Err(format!(
                    "chapeu_voando.json, «{nome}»: a base «{base}» não existe no pack"
                ));
            }
            Some(_) if do_pack(nome).is_some() => {
                return Err(format!(
                    "chapeu_voando.json: «{nome}» já é uma tag do pack; a tag nova precisa de outro nome"
                ));
            }
            _ => {}
        }
    }
    for nome in arte.ancoras.tags.keys() {
        if do_pack(nome).is_none() {
            return Err(format!("ancoras.json: a tag «{nome}» não existe no pack"));
        }
    }
    let mut tags = Vec::new();
    for t in &importado.tags {
        if arte.zeca.skin.excluir.contains(&t.nome) {
            continue;
        }
        let quadros = match arte.voos.tags.get(&t.nome).filter(|v| v.base.is_none()) {
            Some(voo) => quadros_do_voo(&t.nome, voo, t, vestidos, arte)?,
            None => sequencia_do_pack(t)?
                .into_iter()
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
    for (nome, voo) in &arte.voos.tags {
        let Some(base) = &voo.base else {
            continue;
        };
        let t = do_pack(base).expect("conferida acima");
        tags.push(TagZeca {
            nome: nome.clone(),
            original: t.original.clone(),
            quadros: quadros_do_voo(nome, voo, t, vestidos, arte)?,
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

/// O chapéu solto anda num caminho limpo: entre dois quadros seguidos no ar,
/// o centro não pula mais de [`SALTO_SOLTO`] pixels e a altura muda de
/// sentido no máximo uma vez (sobe e desce, sem tremer).
fn conferir_caminhos(tags: &[TagZeca], arte: &Arte, avisos: &mut Vec<String>) {
    for t in tags {
        let mut trecho: Vec<(usize, (i32, i32))> = Vec::new();
        let fechar = |trecho: &mut Vec<(usize, (i32, i32))>, avisos: &mut Vec<String>| {
            let mut sentidos = Vec::new();
            for par in trecho.windows(2) {
                let ((i, a), (j, b)) = (par[0], par[1]);
                let salto = (a.0 - b.0).abs().max((a.1 - b.1).abs());
                if salto > 2 * SALTO_SOLTO {
                    avisos.push(format!(
                        "{}: o centro do chapéu solto pula {} px entre os quadros {i} e {j} (máximo {SALTO_SOLTO})",
                        t.nome,
                        salto as f32 / 2.0
                    ));
                }
                let dy = (b.1 - a.1).signum();
                if dy != 0 && sentidos.last() != Some(&dy) {
                    sentidos.push(dy);
                }
            }
            if sentidos.len() > 2 {
                let alturas: Vec<String> = trecho
                    .iter()
                    .map(|(_, c)| format!("{}", c.1 as f32 / 2.0))
                    .collect();
                avisos.push(format!(
                    "{}: o chapéu solto sobe e desce aos trancos (centro y: {})",
                    t.nome,
                    alturas.join(", ")
                ));
            }
            trecho.clear();
        };
        for (i, q) in t.quadros.iter().enumerate() {
            let centro = match (&q.vestido.chapeu, q.solto) {
                (Some(c), true) => arte.sprites.get(&c.variante).map(|s| compor::centro2(c, s)),
                _ => None,
            };
            match centro {
                Some(c) => trecho.push((i, c)),
                None => fechar(&mut trecho, avisos),
            }
        }
        fechar(&mut trecho, avisos);
    }
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
    let vestidos = vestir_pack(importado, &analises, &fontes, arte, &mut avisos)?;
    let tags = roteiro(importado, &vestidos, arte)?;
    conferir_caminhos(&tags, arte, &mut avisos);
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
    // paleta do .aseprite, e nenhuma é preto (#000000 nunca é contorno; no
    // pack ele é só o índice transparente, que o importador já tira).
    let mut nossas: Vec<[u8; 4]> = arte.paleta.letras.values().copied().collect();
    nossas.push(arte.paleta.contorno);
    nossas.extend(arte.paleta.troca.iter().map(|(_, para)| *para));
    if let Some(preto) = nossas.iter().find(|c| c[..3] == [0, 0, 0]) {
        return Err(format!(
            "cor {} nos acessórios: o contorno é a tinta do pack, nunca preto",
            arte::hex(*preto)
        ));
    }
    if importado.paleta.is_empty() {
        avisos.push("paleta do pack indisponível (tiras PNG): conferência de cores pulada".into());
    } else {
        for c in nossas {
            if !importado.paleta.iter().any(|p| p[..3] == c[..3]) {
                avisos.push(format!("cor {} fora da paleta do pack", arte::hex(c)));
            }
        }
    }

    let tinta = *arte
        .paleta
        .letras
        .get(&'#')
        .ok_or("paleta.toml: falta a letra «#» (a tinta do contorno)")?;
    let protegidas = Protegidas {
        olho: &arte.paleta.olho,
        bico: &arte.paleta.bico,
        tinta,
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
                        q.solto,
                    )?;
                    let Relatorio {
                        fora,
                        sobre_olho,
                        sobre_bico,
                        fora_do_corpo,
                        sangria,
                        afundado,
                        flutuando,
                    } = r;
                    let quadro = format!("{}[{i}]", t.nome);
                    for (n, o_que) in [
                        (fora, "pixel(s) de acessório fora da célula"),
                        (sobre_olho, "pixel(s) de acessório sobre o olho"),
                        (sobre_bico, "pixel(s) de acessório sobre o bico"),
                        (
                            fora_do_corpo,
                            "pixel(s) da gravata fora do miolo do corpo, não desenhados (o contorno do pack ganha): mude a gravata em ancoras.json",
                        ),
                        (
                            sangria,
                            "pixel(s) de cor de acessório encostados no transparente, sem tinta em volta",
                        ),
                        (afundado, "pixel(s) do chapéu assentado afundando no corpo"),
                    ] {
                        if n > 0 {
                            avisos.push(format!("{quadro}: {n} {o_que}"));
                        }
                    }
                    if flutuando {
                        avisos.push(format!("{quadro}: chapéu assentado sem encostar na cabeça"));
                    }
                    if let (true, Some(chapeu)) = (q.solto, &q.vestido.chapeu)
                        && let Some(sprite) = arte.sprites.get(&chapeu.variante)
                    {
                        if let Some(d) = compor::distancia(&corpos[q.corpo], celula, chapeu, sprite)
                            && d < compor::FOLGA_SOLTO
                        {
                            avisos.push(format!(
                                "{quadro}: chapéu solto a {d} px do corpo (com menos de {} o contorno creme junta os dois)",
                                compor::FOLGA_SOLTO
                            ));
                        }
                        let borda = (chapeu.x)
                            .min(chapeu.y)
                            .min(celula.0 as i32 - (chapeu.x + sprite.largura))
                            .min(celula.1 as i32 - (chapeu.y + sprite.altura));
                        if borda < FOLGA_BORDA {
                            avisos.push(format!(
                                "{quadro}: chapéu solto a {borda} px da borda da célula (mínimo {FOLGA_BORDA})"
                            ));
                        }
                    }
                    if variante == Variante::Contorno {
                        let antes = compor::manchas(&rgba, celula);
                        compor::contornar(&mut rgba, celula, arte.paleta.contorno);
                        let depois = compor::manchas(&rgba, celula);
                        if depois < antes {
                            avisos.push(format!(
                                "{quadro}: o contorno creme juntou manchas soltas ({antes} → {depois})"
                            ));
                        }
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
        false,
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
        &["--contorno", "--ancoras", "--estrito"],
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
        duracao_ms: importar::DURACAO_TIRAS,
    });
    let importado = importar::ler_com_reserva(&p2.aseprite, tiras)?;
    let m = montar(&importado, &arte, variante)?;
    let novos: Vec<&String> = m
        .avisos
        .iter()
        .filter(|a| !arte.aceitos.contains(a))
        .collect();
    if a.bandeira("--estrito") && !novos.is_empty() {
        let lista: Vec<String> = novos.iter().map(|a| format!("  aviso: {a}")).collect();
        return Err(format!(
            "{} aviso(s) novo(s) na arte (--estrito; nada foi gravado). Corrija em arte/zeca/ ou, se for do próprio pack e já olhado, ponha em arte/zeca/avisos-aceitos.txt:\n{}",
            novos.len(),
            lista.join("\n")
        ));
    }
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
        if arte.aceitos.contains(aviso) {
            println!("aviso aceito: {aviso}");
        } else {
            println!("aviso: {aviso}");
        }
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
                base: None,
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

    /// Um pack sintético com o layout do Cute Parrots (21 tags, 90 quadros,
    /// clarão no primeiro quadro de Fly Hurt, Hurt e Death), para conferir a
    /// arte de verdade (`arte/zeca`) sem o pack, que nunca entra no repo.
    fn pack_sintetico() -> Importado {
        const QUANTOS: [usize; 21] = [
            4, 6, 3, 4, 3, 3, 4, 3, 10, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6,
        ];
        let mut quadros = Vec::new();
        let mut tags = Vec::new();
        for (nome, n) in TABELA_CUTE_PARROTS.iter().zip(QUANTOS) {
            let de = quadros.len();
            let clarao = matches!(*nome, "Fly Hurt" | "Hurt" | "Death");
            for i in 0..n {
                quadros.push(QuadroFolha {
                    rgba: papagaio(0, (i % 2) as i32, clarao && i == 0),
                    duracao_ms: 100,
                });
            }
            tags.push(TagFolha {
                nome: importar::normalizar(nome),
                original: Some((*nome).to_owned()),
                de,
                ate: quadros.len() - 1,
                direcao: "forward".into(),
            });
        }
        assert_eq!(quadros.len(), 90);
        Importado {
            celula: (48, 48),
            quadros,
            tags,
            origem: importar::Origem::Aseprite,
            aviso: None,
            paleta: Vec::new(),
        }
    }

    #[test]
    fn a_arte_do_repositorio_monta_sobre_o_layout_do_pack() {
        let arte = Arte::carregar(&crate::raiz().join("arte/zeca")).unwrap();
        let m = montar(&pack_sintetico(), &arte, Variante::Contorno).unwrap();
        let tag = |nome: &str| m.tags.iter().find(|t| t.nome == nome).unwrap();
        assert!(m.tags.iter().all(|t| t.nome != "death"), "o Zeca não morre");
        assert_eq!(
            tag("dive_end").quadros.len(),
            4,
            "o chapéu não cai no mergulho: o mergulho tem os 4 quadros do pack"
        );
        let pouso = tag("landing_mergulho");
        assert!(
            pouso.quadros[0].solto,
            "no pouso do mergulho o chapéu ainda está no ar"
        );
        assert!(!pouso.quadros.last().unwrap().solto, "e termina na cabeça");
        let grande = tag("big_flight");
        assert_eq!(
            grande.quadros.last().unwrap().corpo,
            tag("landing_mergulho").quadros.last().unwrap().corpo
        );
        // O pouso comum continua com o chapéu na cabeça o tempo todo.
        assert!(tag("landing").quadros.iter().all(|q| !q.solto));
        // Toda tag dos estados existe (o montar confere) e o idle começa sentado.
        assert!(m.skin_json.contains("\"sit_idle\""));
    }

    #[test]
    fn dados_errados_da_arte_sao_erro_e_nao_somem() {
        let base = Arte::carregar(&crate::raiz().join("arte/zeca")).unwrap();
        let pack = pack_sintetico();
        // Correção para um quadro que a tag não tem.
        let mut arte = base.clone();
        arte.ancoras
            .tags
            .get_mut("dive_end")
            .unwrap()
            .quadros
            .insert("7".into(), arte::AjustesQuadro::default());
        let erro = montar(&pack, &arte, Variante::Simples).unwrap_err();
        assert!(erro.contains("quadro «7»"), "{erro}");
        // Deslocar uma peça escondida pela tag é erro, não um nada calado.
        let mut arte = base.clone();
        arte.ancoras
            .tags
            .get_mut("dive_start")
            .unwrap()
            .quadros
            .insert(
                "1".into(),
                arte::AjustesQuadro {
                    gravata: Some(Ajuste {
                        dx: Some(1),
                        ..Ajuste::default()
                    }),
                    ..arte::AjustesQuadro::default()
                },
            );
        let erro = montar(&pack, &arte, Variante::Simples).unwrap_err();
        assert!(erro.contains("dive_start[1]"), "{erro}");
        // Tag nova do chapéu voando com o nome de uma tag do pack.
        let mut arte = base.clone();
        let voo = arte.voos.tags.remove("landing_mergulho").unwrap();
        arte.voos.tags.insert("walk".into(), voo);
        assert!(montar(&pack, &arte, Variante::Simples).is_err());
    }

    #[test]
    fn direcao_do_pack_vira_ordem_dos_quadros() {
        let t = |direcao: &str| TagFolha {
            nome: "t".into(),
            original: None,
            de: 2,
            ate: 5,
            direcao: direcao.into(),
        };
        assert_eq!(sequencia_do_pack(&t("forward")), Ok(vec![2, 3, 4, 5]));
        assert_eq!(sequencia_do_pack(&t("reverse")), Ok(vec![5, 4, 3, 2]));
        assert_eq!(
            sequencia_do_pack(&t("pingpong")),
            Ok(vec![2, 3, 4, 5, 4, 3])
        );
        assert_eq!(
            sequencia_do_pack(&t("pingpong_reverse")),
            Ok(vec![5, 4, 3, 2, 3, 4])
        );
        assert!(sequencia_do_pack(&t("de lado")).is_err());
    }

    #[test]
    fn caminho_do_chapeu_solto_sem_tranco() {
        let arte = Arte::carregar(&crate::raiz().join("arte/zeca")).unwrap();
        let solto = |cx: i32, cy: i32| QuadroZeca {
            corpo: 0,
            ms: 100,
            solto: true,
            vestido: Vestido {
                // «chapeu» tem 11x5: canto = centro − (5, 2).
                chapeu: Some(compor::Colocacao {
                    variante: "chapeu".into(),
                    x: cx - 5,
                    y: cy - 2,
                }),
                ..Vestido::default()
            },
        };
        let tag = |centros: &[(i32, i32)]| TagZeca {
            nome: "t".into(),
            original: None,
            quadros: centros.iter().map(|&(x, y)| solto(x, y)).collect(),
        };
        let mut avisos = Vec::new();
        conferir_caminhos(
            &[tag(&[(26, 9), (26, 7), (26, 7), (27, 8), (27, 10)])],
            &arte,
            &mut avisos,
        );
        assert!(
            avisos.is_empty(),
            "sobe e desce, passos pequenos: {avisos:?}"
        );
        conferir_caminhos(&[tag(&[(26, 9), (26, 4)])], &arte, &mut avisos);
        assert!(avisos[0].contains("pula 5 px"), "{avisos:?}");
        avisos.clear();
        conferir_caminhos(
            &[tag(&[(26, 7), (26, 5), (26, 7), (26, 5)])],
            &arte,
            &mut avisos,
        );
        assert!(avisos[0].contains("aos trancos"), "{avisos:?}");
    }

    #[test]
    fn creditos_falam_da_licenca() {
        let c = creditos("zeca");
        assert!(c.contains("exclusiveOlive"));
        assert!(c.contains("não pode ser redistribuído"));
    }
}
