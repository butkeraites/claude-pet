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
//! - **Contorno creme** (opcional): 1 pixel de arte em volta de tudo que é
//!   opaco, por fora, nos 8 vizinhos.

use std::collections::BTreeMap;

use super::arte::{Ajuste, ChapeuVoo, Regra, Sprite};

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
/// `variante` troca o desenho. Sem peça de base, só um ajuste com `x`, `y`
/// e `variante` cria uma.
pub fn ajustar(base: Option<Colocacao>, a: &Ajuste) -> Option<Colocacao> {
    if a.oculto {
        return None;
    }
    let mut c = match (base, &a.variante, a.x, a.y) {
        (Some(b), ..) => b,
        (None, Some(v), Some(x), Some(y)) => Colocacao {
            variante: v.clone(),
            x,
            y,
        },
        (None, ..) => return None,
    };
    if let Some(v) = &a.variante {
        c.variante = v.clone();
    }
    c.x = a.x.unwrap_or(c.x) + a.dx.unwrap_or(0);
    c.y = a.y.unwrap_or(c.y) + a.dy.unwrap_or(0);
    Some(c)
}

/// O chapéu de um quadro do voo; `assentado` é o encaixe normal do corpo.
pub fn chapeu_do_voo(c: &ChapeuVoo, assentado: Option<&Colocacao>) -> Option<Colocacao> {
    let mut colocacao = if c.assentado {
        assentado.cloned()?
    } else {
        Colocacao {
            variante: c.variante.clone()?,
            x: c.x?,
            y: c.y?,
        }
    };
    if let Some(v) = &c.variante {
        colocacao.variante = v.clone();
    }
    colocacao.x += c.dx.unwrap_or(0);
    colocacao.y += c.dy.unwrap_or(0);
    Some(colocacao)
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
}

/// Cores do corpo que o encaixe não pode cobrir sem avisar.
pub struct Protegidas<'a> {
    pub olho: &'a [Rgba],
    pub bico: &'a [Rgba],
}

/// Desenha `vestido` sobre `corpo`: primeiro a gravata, depois o chapéu.
pub fn vestir(
    corpo: &[u8],
    celula: (u32, u32),
    vestido: &Vestido,
    sprites: &BTreeMap<String, Sprite>,
    clarao: Rgba,
    protegidas: &Protegidas<'_>,
) -> Result<(Vec<u8>, Relatorio), String> {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    let mut saida = corpo.to_vec();
    let mut r = Relatorio::default();
    for peca in [&vestido.gravata, &vestido.chapeu].into_iter().flatten() {
        let s = sprites
            .get(&peca.variante)
            .ok_or_else(|| format!("variante «{}» não existe", peca.variante))?;
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
                let antes = pixel(corpo, w, x, y);
                if protegidas.olho.iter().any(|&o| mesma_cor(antes, o)) && !vestido.branco {
                    r.sobre_olho += 1;
                }
                if protegidas.bico.iter().any(|&b| mesma_cor(antes, b)) {
                    r.sobre_bico += 1;
                }
                if vestido.branco {
                    c = clarao;
                }
                let i = ((y * w + x) * 4) as usize;
                saida[i..i + 4].copy_from_slice(&c);
            }
        }
    }
    Ok((saida, r))
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

/// Contorno de 1 pixel de arte por fora de tudo que é opaco (8 vizinhos).
pub fn contornar(rgba: &mut [u8], celula: (u32, u32), cor: Rgba) {
    let (w, h) = (celula.0 as i32, celula.1 as i32);
    let opaco = |r: &[u8], x: i32, y: i32| {
        (0..w).contains(&x) && (0..h).contains(&y) && r[((y * w + x) * 4 + 3) as usize] != 0
    };
    let original = rgba.to_vec();
    for y in 0..h {
        for x in 0..w {
            if opaco(&original, x, y) {
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

    /// Célula a partir de uma grade: `.` transparente, `W` branco do olho,
    /// `G` verde, `p` bico.
    fn celula(grade: &[&str]) -> (Vec<u8>, (u32, u32)) {
        let (w, h) = (grade[0].len() as u32, grade.len() as u32);
        let rgba = grade
            .iter()
            .flat_map(|l| l.chars())
            .flat_map(|c| match c {
                'W' => B,
                'G' => V,
                'p' => P,
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
            ajustar(base.clone(), &desloca).map(|c| (c.x, c.y)),
            Some((22, 9))
        );
        let fixa = Ajuste {
            variante: Some("outro".into()),
            x: Some(3),
            dy: Some(1),
            ..Ajuste::default()
        };
        assert_eq!(
            ajustar(base.clone(), &fixa),
            Some(Colocacao {
                variante: "outro".into(),
                x: 3,
                y: 12
            })
        );
        let some = Ajuste {
            oculto: true,
            ..Ajuste::default()
        };
        assert_eq!(ajustar(base, &some), None);
        assert_eq!(
            ajustar(None, &desloca),
            None,
            "sem olho, deslocar não cria peça"
        );
        let cria = Ajuste {
            variante: Some("chapeu".into()),
            x: Some(1),
            y: Some(2),
            ..Ajuste::default()
        };
        assert_eq!(ajustar(None, &cria).map(|c| (c.x, c.y)), Some((1, 2)));
    }

    #[test]
    fn chapeu_do_voo_solto_ou_assentado() {
        let assentado = Colocacao {
            variante: "chapeu".into(),
            x: 21,
            y: 11,
        };
        let pulinho = ChapeuVoo {
            assentado: true,
            dy: Some(-1),
            ..ChapeuVoo::default()
        };
        assert_eq!(
            chapeu_do_voo(&pulinho, Some(&assentado)).map(|c| c.y),
            Some(10)
        );
        assert_eq!(chapeu_do_voo(&pulinho, None), None);
        let solto = ChapeuVoo {
            variante: Some("voando".into()),
            x: Some(5),
            y: Some(1),
            ..ChapeuVoo::default()
        };
        assert_eq!(
            chapeu_do_voo(&solto, Some(&assentado)),
            Some(Colocacao {
                variante: "voando".into(),
                x: 5,
                y: 1
            })
        );
    }

    fn sprites() -> BTreeMap<String, Sprite> {
        let letras = BTreeMap::from([('o', P), ('#', [29, 36, 39, 255])]);
        BTreeMap::from([
            (
                "chapeu".to_owned(),
                Sprite::de_grade("##\n", &letras).unwrap(),
            ),
            (
                "gravata".to_owned(),
                Sprite::de_grade("o.o\n", &letras).unwrap(),
            ),
        ])
    }

    #[test]
    fn vestir_desenha_corta_e_avisa() {
        let (corpo, c) = celula(&["....", ".WW.", ".pp.", "...."]);
        let vestido = Vestido {
            chapeu: Some(Colocacao {
                variante: "chapeu".into(),
                x: 3,
                y: 0,
            }),
            gravata: Some(Colocacao {
                variante: "gravata".into(),
                x: 0,
                y: 1,
            }),
            branco: false,
        };
        let protegidas = Protegidas {
            olho: &[B],
            bico: &[P],
        };
        let (rgba, r) = vestir(&corpo, c, &vestido, &sprites(), B, &protegidas).unwrap();
        assert_eq!(r.fora, 1, "metade do chapéu passa da borda");
        assert_eq!(r.sobre_olho, 1, "a gravata cobre um pixel do olho");
        assert_eq!(pixel(&rgba, 4, 3, 0), [29, 36, 39, 255]);
        assert_eq!(pixel(&rgba, 4, 2, 1), P);
        let branco = Vestido {
            branco: true,
            ..vestido
        };
        let (rgba, _) = vestir(&corpo, c, &branco, &sprites(), B, &protegidas).unwrap();
        assert_eq!(pixel(&rgba, 4, 3, 0), B, "no clarão o chapéu fica branco");
        let sem = Vestido {
            chapeu: Some(Colocacao {
                variante: "nada".into(),
                x: 0,
                y: 0,
            }),
            ..Vestido::default()
        };
        assert!(vestir(&corpo, c, &sem, &sprites(), B, &protegidas).is_err());
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
