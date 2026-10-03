//! Encaixe dos acessórios quadro a quadro (decisões 0023 e 0024).
//!
//! - **Âncora:** o olho branco do pack. Ele é a maior mancha 8-conexa das
//!   cores do olho; manchas menores (bolhas do sono, o risco da mordida) não
//!   enganam a âncora.
//! - **Clarão:** quadro em que o branco cobre a maior parte do corpo (o
//!   susto começa com a silhueta branca). Ali não há olho: a âncora vem do
//!   quadro comum com a mesma silhueta, e os acessórios também ficam
//!   brancos.
//! - **Regra do olho:** chapéu e gravata ficam a um deslocamento fixo do
//!   canto superior esquerdo do olho (`ancoras.json`, `regra`); as correções
//!   por tag e por quadro vêm depois.
//! - **A gravata vai no peito, dentro do contorno:** no pack, toda cor que
//!   não é o branco fica cercada pela tinta. A gravata (que não tem contorno
//!   próprio) só pinta o miolo do corpo; um pixel que cairia no transparente
//!   ou no contorno da silhueta não é desenhado e vira aviso, para a
//!   posição ser corrigida em `ancoras.json`.
//! - **O chapéu vai por cima:** desenhado inteiro. Assentado, a base de cada
//!   coluna (a aba) pousa no contorno do topo da cabeça; acima dela, nada do
//!   chapéu pode cair sobre o corpo (afundado) e ele precisa encostar no
//!   corpo (senão flutua).
//! - **Contorno creme** (opcional): 1 pixel de arte em volta de tudo que é
//!   opaco, por fora, nos 8 vizinhos.

use std::collections::BTreeMap;

use super::arte::{Ajuste, ChapeuVoo, Regra, Sprite};

/// Os 4 vizinhos.
const VIZINHOS4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

pub type Rgba = [u8; 4];

/// Caixa inclusiva em pixels de arte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caixa {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Analise {
    pub olho: Option<Caixa>,
    pub clarao: bool,
}

pub fn pixel(rgba: &[u8], largura: i32, x: i32, y: i32) -> Rgba {
    let i = ((y * largura + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

fn mesma_cor(p: Rgba, c: Rgba) -> bool {
    p[3] != 0 && p[..3] == c[..3]
}

/// Olho e clarão de uma célula.
pub fn analisar(rgba: &[u8], celula: (u32, u32), olho: &[Rgba], clarao: Rgba) -> Analise {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    let opacos = rgba.chunks_exact(4).filter(|p| p[3] != 0).count();
    let brancos = rgba
        .chunks_exact(4)
        .filter(|p| mesma_cor([p[0], p[1], p[2], p[3]], clarao))
        .count();
    if opacos > 0 && brancos * 10 >= opacos * 6 {
        return Analise {
            olho: None,
            clarao: true,
        };
    }
    let eh_olho = |x: i32, y: i32| olho.iter().any(|&c| mesma_cor(pixel(rgba, w, x, y), c));
    let mut visto = vec![false; (w * h) as usize];
    let mut melhor: Option<(usize, Caixa)> = None;
    for y in 0..h {
        for x in 0..w {
            if visto[(y * w + x) as usize] || !eh_olho(x, y) {
                continue;
            }
            let mut pilha = vec![(x, y)];
            visto[(y * w + x) as usize] = true;
            let mut caixa = Caixa {
                x0: x,
                y0: y,
                x1: x,
                y1: y,
            };
            let mut n = 0;
            while let Some((cx, cy)) = pilha.pop() {
                n += 1;
                caixa.x0 = caixa.x0.min(cx);
                caixa.y0 = caixa.y0.min(cy);
                caixa.x1 = caixa.x1.max(cx);
                caixa.y1 = caixa.y1.max(cy);
                for (dx, dy) in [
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if (0..w).contains(&nx)
                        && (0..h).contains(&ny)
                        && !visto[(ny * w + nx) as usize]
                        && eh_olho(nx, ny)
                    {
                        visto[(ny * w + nx) as usize] = true;
                        pilha.push((nx, ny));
                    }
                }
            }
            if melhor.is_none_or(|(m, _)| n > m) {
                melhor = Some((n, caixa));
            }
        }
    }
    Analise {
        olho: melhor.filter(|(n, _)| *n >= 4).map(|(_, c)| c),
        clarao: false,
    }
}

fn mascara(rgba: &[u8]) -> Vec<bool> {
    rgba.chunks_exact(4).map(|p| p[3] != 0).collect()
}

/// Para cada quadro de clarão, o primeiro quadro comum com a mesma
/// silhueta (máscara de opacidade idêntica).
pub fn fontes_de_clarao(quadros: &[&[u8]], analises: &[Analise]) -> Vec<Option<usize>> {
    let mascaras: Vec<Vec<bool>> = quadros.iter().map(|q| mascara(q)).collect();
    analises
        .iter()
        .enumerate()
        .map(|(i, a)| {
            if !a.clarao {
                return None;
            }
            (0..quadros.len()).find(|&j| !analises[j].clarao && mascaras[j] == mascaras[i])
        })
        .collect()
}

/// Uma peça posta na célula: a variante e o canto superior esquerdo.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Colocacao {
    pub variante: String,
    pub x: i32,
    pub y: i32,
}

/// O que vai por cima de um corpo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Vestido {
    pub chapeu: Option<Colocacao>,
    pub gravata: Option<Colocacao>,
    /// Quadro de clarão: os acessórios ficam brancos como a silhueta.
    pub branco: bool,
}

pub fn pela_regra(olho: Option<Caixa>, regra: &Regra) -> Option<Colocacao> {
    olho.map(|o| Colocacao {
        variante: regra.variante.clone(),
        x: o.x0 + regra.dx,
        y: o.y0 + regra.dy,
    })
}

/// Aplica um ajuste: `oculto` tira a peça, `x`/`y` fixam, `dx`/`dy` somam,
/// `variante` troca o desenho. Sem peça de base (a tag escondeu a peça ou
/// não há olho), `x` e `y` criam uma, com a `variante` do ajuste ou a da
/// regra (`variante_padrao`); um ajuste que só desloca ou troca o desenho de
/// uma peça que não existe é erro, em vez de sumir calado.
pub fn ajustar(
    base: Option<Colocacao>,
    a: &Ajuste,
    variante_padrao: &str,
) -> Result<Option<Colocacao>, String> {
    if a.oculto {
        return Ok(None);
    }
    let mut c = match (base, a.x, a.y) {
        (Some(b), ..) => b,
        (None, Some(x), Some(y)) => Colocacao {
            variante: a
                .variante
                .clone()
                .unwrap_or_else(|| variante_padrao.to_owned()),
            x,
            y,
        },
        (None, ..) => {
            return Err(
                "ajuste sem peça para ajustar (a tag a escondeu ou não há olho): dê x e y para criar uma"
                    .into(),
            );
        }
    };
    if let Some(v) = &a.variante {
        c.variante = v.clone();
    }
    c.x = a.x.unwrap_or(c.x) + a.dx.unwrap_or(0);
    c.y = a.y.unwrap_or(c.y) + a.dy.unwrap_or(0);
    Ok(Some(c))
}

/// Canto superior esquerdo de um desenho `w`x`h` com centro em (`cx`, `cy`)
/// (o centro de um lado par fica no pixel de cima/da esquerda).
pub fn canto_pelo_centro(cx: i32, cy: i32, w: i32, h: i32) -> (i32, i32) {
    (cx - (w - 1) / 2, cy - (h - 1) / 2)
}

/// Centro de uma peça, em meios pixels (para comparar sem arredondar).
pub fn centro2(c: &Colocacao, s: &Sprite) -> (i32, i32) {
    (2 * c.x + s.largura - 1, 2 * c.y + s.altura - 1)
}

/// O chapéu de um quadro do voo; `assentado` é o encaixe normal do corpo.
pub fn chapeu_do_voo(
    c: &ChapeuVoo,
    assentado: Option<&Colocacao>,
    sprites: &BTreeMap<String, Sprite>,
) -> Result<Option<Colocacao>, String> {
    let mut colocacao = if c.assentado {
        let Some(a) = assentado else {
            return Ok(None);
        };
        a.clone()
    } else {
        let variante = c.variante.clone().ok_or("chapéu solto sem variante")?;
        let (x, y) = match (c.x, c.y, c.cx, c.cy) {
            (Some(x), Some(y), None, None) => (x, y),
            (None, None, Some(cx), Some(cy)) => {
                let s = sprites
                    .get(&variante)
                    .ok_or_else(|| format!("variante «{variante}» não existe"))?;
                canto_pelo_centro(cx, cy, s.largura, s.altura)
            }
            _ => return Err("chapéu solto precisa de x/y ou de cx/cy".into()),
        };
        Colocacao { variante, x, y }
    };
    if let Some(v) = &c.variante {
        colocacao.variante = v.clone();
    }
    colocacao.x += c.dx.unwrap_or(0);
    colocacao.y += c.dy.unwrap_or(0);
    Ok(Some(colocacao))
}

/// O que o encaixe cobriu ou deixou de fora, para os avisos.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Relatorio {
    /// Pixels de acessório fora da célula (cortados).
    pub fora: usize,
    /// Pixels do olho cobertos.
    pub sobre_olho: usize,
    /// Pixels do bico cobertos.
    pub sobre_bico: usize,
    /// Pixels da gravata que cairiam fora do miolo do corpo (no
    /// transparente ou no contorno da silhueta) e não foram desenhados.
    pub fora_do_corpo: usize,
    /// Pixels de cor dos acessórios encostados no transparente onde o
    /// desenho não previa (no pack, toda cor fica cercada pela tinta).
    pub sangria: usize,
    /// Pixels do chapéu assentado acima da aba que caem sobre o corpo: o
    /// chapéu afundando na cabeça.
    pub afundado: usize,
    /// O chapéu assentado não encosta no corpo.
    pub flutuando: bool,
}

/// Cores do corpo que o encaixe precisa conhecer.
pub struct Protegidas<'a> {
    pub olho: &'a [Rgba],
    pub bico: &'a [Rgba],
    /// A tinta do contorno (`#1D2427`): pixel de acessório nessa cor pode
    /// encostar no transparente; as outras cores não.
    pub tinta: Rgba,
}

fn opaco(rgba: &[u8], w: i32, h: i32, x: i32, y: i32) -> bool {
    (0..w).contains(&x) && (0..h).contains(&y) && rgba[((y * w + x) * 4 + 3) as usize] != 0
}

/// Miolo do corpo: pixel opaco com os 4 vizinhos opacos (o contorno da
/// silhueta, que encosta no transparente, fica de fora).
pub fn miolo(rgba: &[u8], celula: (u32, u32), x: i32, y: i32) -> bool {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    opaco(rgba, w, h, x, y)
        && VIZINHOS4
            .iter()
            .all(|(dx, dy)| opaco(rgba, w, h, x + dx, y + dy))
}

/// Desenha `vestido` sobre `corpo`: primeiro a gravata (só no miolo do
/// corpo), depois o chapéu (por cima de tudo). `solto`: o chapéu está no ar
/// (chapéu voando), e as checagens de chapéu assentado não valem.
pub fn vestir(
    corpo: &[u8],
    celula: (u32, u32),
    vestido: &Vestido,
    sprites: &BTreeMap<String, Sprite>,
    clarao: Rgba,
    protegidas: &Protegidas<'_>,
    solto: bool,
) -> Result<(Vec<u8>, Relatorio), String> {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    let mut saida = corpo.to_vec();
    let mut r = Relatorio::default();
    // Pixels de cor desenhados: (x, y na célula, peça, sx, sy no desenho).
    let mut de_cor: Vec<(i32, i32, &Sprite, i32, i32)> = Vec::new();
    for (peca, no_corpo) in [(&vestido.gravata, true), (&vestido.chapeu, false)] {
        let Some(peca) = peca else {
            continue;
        };
        let s = sprites
            .get(&peca.variante)
            .ok_or_else(|| format!("variante «{}» não existe", peca.variante))?;
        // Base de cada coluna do desenho: o pixel opaco mais baixo (a aba).
        let base: Vec<i32> = (0..s.largura)
            .map(|sx| {
                (0..s.altura)
                    .rev()
                    .find(|&sy| s.em(sx, sy).is_some())
                    .unwrap_or(-1)
            })
            .collect();
        let mut encosta = false;
        for sy in 0..s.altura {
            for sx in 0..s.largura {
                let Some(mut c) = s.em(sx, sy) else {
                    continue;
                };
                let (x, y) = (peca.x + sx, peca.y + sy);
                if !(0..w).contains(&x) || !(0..h).contains(&y) {
                    r.fora += 1;
                    continue;
                }
                if no_corpo && !miolo(corpo, celula, x, y) {
                    r.fora_do_corpo += 1;
                    continue;
                }
                let antes = pixel(corpo, w, x, y);
                if protegidas.olho.iter().any(|&o| mesma_cor(antes, o)) && !vestido.branco {
                    r.sobre_olho += 1;
                }
                if protegidas.bico.iter().any(|&b| mesma_cor(antes, b)) {
                    r.sobre_bico += 1;
                }
                if !no_corpo && !solto {
                    if antes[3] != 0 && sy < base[sx as usize] {
                        r.afundado += 1;
                    }
                    encosta |=
                        (-1..=1).any(|dy| (-1..=1).any(|dx| opaco(corpo, w, h, x + dx, y + dy)));
                }
                if c[..3] != protegidas.tinta[..3] {
                    de_cor.push((x, y, s, sx, sy));
                }
                if vestido.branco {
                    c = clarao;
                }
                let i = ((y * w + x) * 4) as usize;
                saida[i..i + 4].copy_from_slice(&c);
            }
        }
        if !no_corpo && !solto && !encosta {
            r.flutuando = true;
        }
    }
    // Sangria: cor de acessório encostada no transparente, a não ser que o
    // próprio desenho deixe aquele lado aberto (um `.` dentro da grade, como
    // a ponta da aba do chapéu). No clarão tudo é o branco do pack, que pode.
    if !vestido.branco {
        for (x, y, s, sx, sy) in de_cor {
            let exposto = VIZINHOS4.iter().any(|&(dx, dy)| {
                if opaco(&saida, w, h, x + dx, y + dy) {
                    return false;
                }
                let (nx, ny) = (sx + dx, sy + dy);
                let aberto_no_desenho = (0..s.largura).contains(&nx)
                    && (0..s.altura).contains(&ny)
                    && s.em(nx, ny).is_none();
                !aberto_no_desenho
            });
            if exposto {
                r.sangria += 1;
            }
        }
    }
    Ok((saida, r))
}

/// Manchas opacas 8-conexas de uma célula (o corpo, o chapéu solto, uma
/// bolha…). O contorno creme não pode juntar duas manchas.
pub fn manchas(rgba: &[u8], celula: (u32, u32)) -> usize {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    let mut visto = vec![false; (w * h) as usize];
    let mut n = 0;
    for y in 0..h {
        for x in 0..w {
            if visto[(y * w + x) as usize] || !opaco(rgba, w, h, x, y) {
                continue;
            }
            n += 1;
            visto[(y * w + x) as usize] = true;
            let mut pilha = vec![(x, y)];
            while let Some((px, py)) = pilha.pop() {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (px + dx, py + dy);
                        if opaco(rgba, w, h, nx, ny) && !visto[(ny * w + nx) as usize] {
                            visto[(ny * w + nx) as usize] = true;
                            pilha.push((nx, ny));
                        }
                    }
                }
            }
        }
    }
    n
}

/// Menor distância (Chebyshev, em pixels de arte) entre os pixels opacos da
/// peça e os do corpo. Abaixo de 4, o contorno creme de 1 pixel em volta dos
/// dois vira uma ponte entre eles.
pub fn distancia(
    corpo: &[u8],
    celula: (u32, u32),
    peca: &Colocacao,
    sprite: &Sprite,
) -> Option<i32> {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    let corpo_px: Vec<(i32, i32)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| corpo[((y * w + x) * 4 + 3) as usize] != 0)
        .collect();
    let mut menor: Option<i32> = None;
    for sy in 0..sprite.altura {
        for sx in 0..sprite.largura {
            if sprite.em(sx, sy).is_none() {
                continue;
            }
            let (x, y) = (peca.x + sx, peca.y + sy);
            for &(cx, cy) in &corpo_px {
                let d = (cx - x).abs().max((cy - y).abs());
                menor = Some(menor.map_or(d, |m| m.min(d)));
            }
        }
    }
    menor
}

/// Folga mínima entre o chapéu solto e o corpo para o contorno não juntar
/// os dois.
pub const FOLGA_SOLTO: i32 = 4;

/// Contorno de 1 pixel de arte por fora de tudo que é opaco (8 vizinhos),
/// só do lado de fora: furos fechados (o meio da bolha do sono, o vão entre
/// asa e corpo) continuam transparentes. "Fora" é o transparente que chega na
/// borda da célula andando nos 4 vizinhos.
pub fn contornar(rgba: &mut [u8], celula: (u32, u32), cor: Rgba) {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    let opaco = |r: &[u8], x: i32, y: i32| {
        (0..w).contains(&x) && (0..h).contains(&y) && r[((y * w + x) * 4 + 3) as usize] != 0
    };
    let original = rgba.to_vec();
    let mut fora = vec![false; (w * h) as usize];
    let mut pilha: Vec<(i32, i32)> = (0..w)
        .flat_map(|x| [(x, 0), (x, h - 1)])
        .chain((0..h).flat_map(|y| [(0, y), (w - 1, y)]))
        .filter(|&(x, y)| !opaco(&original, x, y))
        .collect();
    while let Some((x, y)) = pilha.pop() {
        let i = (y * w + x) as usize;
        if fora[i] {
            continue;
        }
        fora[i] = true;
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if (0..w).contains(&nx) && (0..h).contains(&ny) && !opaco(&original, nx, ny) {
                pilha.push((nx, ny));
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            if !fora[(y * w + x) as usize] {
                continue;
            }
            let vizinho = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .any(|(dx, dy)| opaco(&original, x + dx, y + dy));
            if vizinho {
                let i = ((y * w + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&cor);
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const T: Rgba = [0, 0, 0, 0];
    const B: Rgba = [245, 247, 244, 255];
    const V: Rgba = [156, 201, 112, 255];
    const P: Rgba = [225, 83, 111, 255];
    const TINTA: Rgba = [29, 36, 39, 255];

    /// Célula a partir de uma grade: `.` transparente, `W` branco do olho,
    /// `G` verde, `p` bico, `#` tinta.
    fn celula(grade: &[&str]) -> (Vec<u8>, (u32, u32)) {
        let (w, h) = (grade[0].len() as u32, grade.len() as u32);
        let rgba = grade
            .iter()
            .flat_map(|l| l.chars())
            .flat_map(|c| match c {
                'W' => B,
                'G' => V,
                'p' => P,
                '#' => TINTA,
                _ => T,
            })
            .collect();
        (rgba, (w, h))
    }

    #[test]
    fn olho_e_a_maior_mancha_branca() {
        let (rgba, c) = celula(&[
            "..........",
            ".GGGGG..W.",
            ".GWWWG.W.W",
            ".GWWWG..W.",
            ".GWWWG....",
            ".GGGGG....",
        ]);
        let a = analisar(&rgba, c, &[B], B);
        assert!(!a.clarao);
        assert_eq!(
            a.olho,
            Some(Caixa {
                x0: 2,
                y0: 2,
                x1: 4,
                y1: 4
            }),
            "a bolha do sono (anel de 4 px) não ganha do olho (9 px)"
        );
    }

    #[test]
    fn mancha_pequena_nao_e_olho_e_silhueta_branca_e_clarao() {
        let (rgba, c) = celula(&["GGG", "GWG", "GGG"]);
        assert_eq!(analisar(&rgba, c, &[B], B).olho, None);
        let (branco, c2) = celula(&["WWW", "WWW", "WG."]);
        let a = analisar(&branco, c2, &[B], B);
        assert!(a.clarao);
        assert_eq!(a.olho, None);
    }

    #[test]
    fn clarao_herda_a_ancora_do_quadro_com_a_mesma_silhueta() {
        let (normal, _) = celula(&["GGG.", "GWWG", "GWWG", "GGG."]);
        let (outro, _) = celula(&["GGGG", "GWWG", "GWWG", "GGG."]);
        let (flash, _) = celula(&["WWW.", "WWWW", "WWWW", "WWW."]);
        let quadros = [outro.as_slice(), normal.as_slice(), flash.as_slice()];
        let analises: Vec<Analise> = quadros
            .iter()
            .map(|q| analisar(q, (4, 4), &[B], B))
            .collect();
        assert_eq!(
            fontes_de_clarao(&quadros, &analises),
            vec![None, None, Some(1)]
        );
    }

    #[test]
    fn regra_e_ajustes() {
        let regra = Regra {
            variante: "chapeu".into(),
            dx: -4,
            dy: -6,
        };
        let olho = Some(Caixa {
            x0: 25,
            y0: 17,
            x1: 28,
            y1: 21,
        });
        let base = pela_regra(olho, &regra);
        assert_eq!(
            base,
            Some(Colocacao {
                variante: "chapeu".into(),
                x: 21,
                y: 11
            })
        );
        let desloca = Ajuste {
            dx: Some(1),
            dy: Some(-2),
            ..Ajuste::default()
        };
        assert_eq!(
            ajustar(base.clone(), &desloca, "chapeu").map(|c| c.map(|c| (c.x, c.y))),
            Ok(Some((22, 9)))
        );
        let fixa = Ajuste {
            variante: Some("outro".into()),
            x: Some(3),
            dy: Some(1),
            ..Ajuste::default()
        };
        assert_eq!(
            ajustar(base.clone(), &fixa, "chapeu"),
            Ok(Some(Colocacao {
                variante: "outro".into(),
                x: 3,
                y: 12
            }))
        );
        let some = Ajuste {
            oculto: true,
            ..Ajuste::default()
        };
        assert_eq!(ajustar(base, &some, "chapeu"), Ok(None));
        assert!(
            ajustar(None, &desloca, "chapeu").is_err(),
            "deslocar o que não existe é erro, não some calado"
        );
        // Sem peça (a tag escondeu): x e y criam, com a variante da regra
        // (o quadro 0 do dive_start, que a revisão achou ignorado).
        let cria = Ajuste {
            x: Some(1),
            y: Some(2),
            ..Ajuste::default()
        };
        assert_eq!(
            ajustar(None, &cria, "gravata"),
            Ok(Some(Colocacao {
                variante: "gravata".into(),
                x: 1,
                y: 2
            }))
        );
        let cria_outra = Ajuste {
            variante: Some("chapeu".into()),
            ..cria
        };
        assert_eq!(
            ajustar(None, &cria_outra, "gravata").map(|c| c.map(|c| c.variante)),
            Ok(Some("chapeu".into()))
        );
    }

    #[test]
    fn chapeu_do_voo_solto_assentado_ou_pelo_centro() {
        let assentado = Colocacao {
            variante: "chapeu".into(),
            x: 21,
            y: 11,
        };
        let s = sprites();
        let pulinho = ChapeuVoo {
            assentado: true,
            dy: Some(-1),
            ..ChapeuVoo::default()
        };
        assert_eq!(
            chapeu_do_voo(&pulinho, Some(&assentado), &s).map(|c| c.map(|c| c.y)),
            Ok(Some(10))
        );
        assert_eq!(chapeu_do_voo(&pulinho, None, &s), Ok(None));
        let solto = ChapeuVoo {
            variante: Some("gravata".into()),
            x: Some(5),
            y: Some(1),
            ..ChapeuVoo::default()
        };
        assert_eq!(
            chapeu_do_voo(&solto, Some(&assentado), &s),
            Ok(Some(Colocacao {
                variante: "gravata".into(),
                x: 5,
                y: 1
            }))
        );
        // Pelo centro: a gravata de teste tem 3x1, centro no pixel do meio.
        let centro = ChapeuVoo {
            variante: Some("gravata".into()),
            cx: Some(10),
            cy: Some(4),
            ..ChapeuVoo::default()
        };
        let c = chapeu_do_voo(&centro, None, &s).unwrap().unwrap();
        assert_eq!((c.x, c.y), (9, 4));
        assert_eq!(centro2(&c, &s["gravata"]), (20, 8), "meio pixel × 2");
        assert_eq!(canto_pelo_centro(26, 7, 11, 5), (21, 5));
        assert_eq!(canto_pelo_centro(26, 7, 5, 11), (24, 2));
        assert_eq!(canto_pelo_centro(26, 7, 11, 4), (21, 6));
    }

    fn sprites() -> BTreeMap<String, Sprite> {
        let letras = BTreeMap::from([('o', P), ('#', TINTA)]);
        BTreeMap::from([
            (
                "chapeu".to_owned(),
                Sprite::de_grade("##\n", &letras).unwrap(),
            ),
            (
                "gravata".to_owned(),
                Sprite::de_grade("o.o\n", &letras).unwrap(),
            ),
            (
                // Chapéu com a ponta da aba aberta em cima, como o de
                // verdade: «..###» / «#o#o#» / «#####».
                "palheta".to_owned(),
                Sprite::de_grade("..###\n#o#o#\n#####\n", &letras).unwrap(),
            ),
        ])
    }

    fn protegidas() -> Protegidas<'static> {
        Protegidas {
            olho: &[B],
            bico: &[P],
            tinta: TINTA,
        }
    }

    #[test]
    fn vestir_desenha_corta_e_avisa() {
        let (corpo, c) = celula(&[
            ".......", ".GGGGG.", ".GWWGG.", ".GGGGG.", ".GppGG.", ".GGGGG.", ".......",
        ]);
        let vestido = Vestido {
            chapeu: Some(Colocacao {
                variante: "chapeu".into(),
                x: 6,
                y: 0,
            }),
            gravata: Some(Colocacao {
                variante: "gravata".into(),
                x: 2,
                y: 2,
            }),
            branco: false,
        };
        let (rgba, r) = vestir(&corpo, c, &vestido, &sprites(), B, &protegidas(), true).unwrap();
        assert_eq!(r.fora, 1, "metade do chapéu passa da borda");
        assert_eq!(r.sobre_olho, 1, "a gravata cobre um pixel do olho");
        assert_eq!(r.fora_do_corpo, 0);
        assert_eq!(r.sangria, 0, "a gravata está no miolo");
        assert_eq!(pixel(&rgba, 7, 6, 0), TINTA);
        assert_eq!(pixel(&rgba, 7, 2, 2), P);
        assert_eq!(pixel(&rgba, 7, 3, 2), B, "o vão do meio deixa o corpo");
        let branco = Vestido {
            branco: true,
            ..vestido
        };
        let (rgba, _) = vestir(&corpo, c, &branco, &sprites(), B, &protegidas(), true).unwrap();
        assert_eq!(pixel(&rgba, 7, 6, 0), B, "no clarão o chapéu fica branco");
        let sem = Vestido {
            chapeu: Some(Colocacao {
                variante: "nada".into(),
                x: 0,
                y: 0,
            }),
            ..Vestido::default()
        };
        assert!(vestir(&corpo, c, &sem, &sprites(), B, &protegidas(), true).is_err());
    }

    #[test]
    fn gravata_so_no_miolo_e_o_contorno_da_silhueta_ganha() {
        // A gravata na beirada do peito: o pixel da direita cairia no
        // contorno (que encosta no transparente), o de baixo no ar.
        let (corpo, c) = celula(&["#####.", "#GGG#.", "#GGG#.", "#####.", "......"]);
        let na_beira = Vestido {
            gravata: Some(Colocacao {
                variante: "gravata".into(),
                x: 2,
                y: 2,
            }),
            ..Vestido::default()
        };
        let (rgba, r) = vestir(&corpo, c, &na_beira, &sprites(), B, &protegidas(), true).unwrap();
        assert_eq!(r.fora_do_corpo, 1, "o pixel sobre o contorno não vai");
        assert_eq!(pixel(&rgba, 6, 4, 2), TINTA, "o contorno fica");
        assert_eq!(pixel(&rgba, 6, 2, 2), P, "o do miolo vai");
        assert_eq!(r.sangria, 0);
        let no_ar = Vestido {
            gravata: Some(Colocacao {
                variante: "gravata".into(),
                x: 3,
                y: 4,
            }),
            ..Vestido::default()
        };
        let (rgba, r) = vestir(&corpo, c, &no_ar, &sprites(), B, &protegidas(), true).unwrap();
        assert_eq!(r.fora_do_corpo, 2, "gravata nunca flutua no ar");
        assert_eq!(rgba, corpo);
    }

    #[test]
    fn chapeu_assentado_afundado_flutuando_e_sangria() {
        // Cabeça com o contorno de cima na linha 3.
        let (corpo, c) = celula(&[
            "......", "......", "......", ".####.", ".#GG#.", ".#GG#.", ".####.",
        ]);
        let em = |x: i32, y: i32| Vestido {
            chapeu: Some(Colocacao {
                variante: "palheta".into(),
                x,
                y,
            }),
            ..Vestido::default()
        };
        // Aba na linha 3, em cima do contorno da cabeça: assentado certo.
        let (_, r) = vestir(&corpo, c, &em(1, 1), &sprites(), B, &protegidas(), false).unwrap();
        assert_eq!((r.afundado, r.flutuando, r.sangria), (0, false, 0), "{r:?}");
        // Um pixel mais baixo: a copa entra na cabeça.
        let (_, r) = vestir(&corpo, c, &em(1, 2), &sprites(), B, &protegidas(), false).unwrap();
        assert!(r.afundado > 0, "{r:?}");
        // Dois pixels acima: não encosta na cabeça.
        let (_, r) = vestir(&corpo, c, &em(1, -2), &sprites(), B, &protegidas(), false).unwrap();
        assert!(r.flutuando, "{r:?}");
        // Solto (voando), longe da cabeça, nada disso é aviso.
        let (_, r) = vestir(&corpo, c, &em(1, -2), &sprites(), B, &protegidas(), true).unwrap();
        assert!(!r.flutuando && r.afundado == 0);
        // A cor da aba encosta no ar só onde o desenho deixou aberto (o «.»
        // em cima dela); a cor do meio, cercada de tinta, não.
        let (_, r) = vestir(&corpo, c, &em(0, 0), &sprites(), B, &protegidas(), true).unwrap();
        assert_eq!(r.sangria, 0, "{r:?}");
        let letras = BTreeMap::from([('o', P), ('#', TINTA)]);
        let mut s = sprites();
        s.insert(
            "vaza".into(),
            Sprite::de_grade("o#\n##\n", &letras).unwrap(),
        );
        let vaza = Vestido {
            chapeu: Some(Colocacao {
                variante: "vaza".into(),
                x: 0,
                y: 0,
            }),
            ..Vestido::default()
        };
        let (_, r) = vestir(&corpo, c, &vaza, &s, B, &protegidas(), true).unwrap();
        assert_eq!(r.sangria, 1, "cor na borda do desenho, encostada no ar");
        let branco = Vestido {
            branco: true,
            ..vaza
        };
        let (_, r) = vestir(&corpo, c, &branco, &s, B, &protegidas(), true).unwrap();
        assert_eq!(r.sangria, 0, "no clarão o branco pode encostar no ar");
    }

    #[test]
    fn manchas_oito_conexas() {
        let (rgba, c) = celula(&["G...", ".G..", "...G", "...G"]);
        assert_eq!(manchas(&rgba, c), 2, "a diagonal junta");
        let (vazio, c) = celula(&["....", "...."]);
        assert_eq!(manchas(&vazio, c), 0);
    }

    #[test]
    fn distancia_do_chapeu_solto_ao_corpo() {
        let (corpo, c) = celula(&["......", "......", "......", "......", "GG....", "GG...."]);
        let s = sprites();
        let longe = Colocacao {
            variante: "chapeu".into(),
            x: 4,
            y: 0,
        };
        assert_eq!(distancia(&corpo, c, &longe, &s["chapeu"]), Some(4));
        let perto = Colocacao {
            variante: "chapeu".into(),
            x: 2,
            y: 2,
        };
        assert_eq!(distancia(&corpo, c, &perto, &s["chapeu"]), Some(2));
    }

    #[test]
    fn contorno_nao_enche_furo_fechado() {
        // Anel (a bolha do sono): o meio fica transparente.
        let (mut rgba, c) = celula(&["......", "..GG..", ".G..G.", ".G..G.", "..GG..", "......"]);
        let creme = [247, 231, 197, 255];
        contornar(&mut rgba, c, creme);
        assert_eq!(pixel(&rgba, 6, 2, 2), T, "o meio da bolha continua vazio");
        assert_eq!(pixel(&rgba, 6, 1, 1), creme, "por fora, encostado no anel");
        assert_eq!(pixel(&rgba, 6, 0, 0), T, "longe do anel, nada");
    }

    #[test]
    fn contorno_por_fora_nos_oito_vizinhos() {
        let (mut rgba, c) = celula(&[".....", ".....", "..G..", ".....", "....."]);
        let creme = [247, 231, 197, 255];
        contornar(&mut rgba, c, creme);
        let contados = rgba.chunks_exact(4).filter(|p| p == &creme).count();
        assert_eq!(contados, 8);
        assert_eq!(pixel(&rgba, 5, 2, 2), V, "o corpo não muda");
        assert_eq!(pixel(&rgba, 5, 0, 0), T, "só 1 pixel de largura");
    }
}
