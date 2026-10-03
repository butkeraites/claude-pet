//! Cena: o que está no buffer, como lista de elementos em ordem de desenho.
//!
//! Entre dois quadros, o dano é o antes e o depois de cada elemento que
//! mudou (elementos casados pela posição na lista). Redesenhar uma região é
//! limpá-la e desenhar, em ordem, todo elemento que a cruza; assim regiões
//! sobrepostas e elementos sobrepostos sempre terminam corretos.

use crate::geometria::Ret;
use crate::raster::{self, Alvo};
use crate::skin::Skin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elemento {
    /// Um quadro da folha com a célula em (x, y), em pixels do monitor.
    Sprite {
        quadro: usize,
        x: i32,
        y: i32,
        d: i32,
        espelhar: bool,
    },
    /// Um retângulo de cor sólida (BGRA pré-multiplicado): confete, efeitos.
    Bloco { ret: Ret, cor: [u8; 4] },
}

impl Elemento {
    /// Retângulo que o elemento pode pintar.
    pub fn limites(&self, skin: &Skin) -> Ret {
        match *self {
            Elemento::Sprite {
                quadro,
                x,
                y,
                d,
                espelhar,
            } => raster::limites_do_quadro(skin, quadro, x, y, d, espelhar),
            Elemento::Bloco { ret, .. } => ret,
        }
    }

    pub fn desenhar(&self, alvo: &mut Alvo, skin: &Skin, recorte: Ret) {
        match *self {
            Elemento::Sprite {
                quadro,
                x,
                y,
                d,
                espelhar,
            } => raster::desenhar_quadro(alvo, skin, quadro, x, y, d, espelhar, recorte),
            Elemento::Bloco { ret, cor } => raster::preencher(alvo, ret, cor, recorte),
        }
    }
}

/// Retângulos que mudaram de `antes` para `depois`.
pub fn danos(antes: &[Elemento], depois: &[Elemento], skin: &Skin) -> Vec<Ret> {
    let mut saida = Vec::new();
    for i in 0..antes.len().max(depois.len()) {
        let (a, b) = (antes.get(i), depois.get(i));
        if a == b {
            continue;
        }
        let antes = a.map(|e| e.limites(skin));
        let depois = b.map(|e| e.limites(skin)).filter(|r| Some(*r) != antes);
        for r in [antes, depois].into_iter().flatten() {
            if !r.vazio() {
                saida.push(r);
            }
        }
    }
    saida
}

/// Limpa cada região e desenha nela, em ordem, os elementos que a cruzam.
pub fn redesenhar(alvo: &mut Alvo, elementos: &[Elemento], skin: &Skin, regioes: &[Ret]) {
    for &regiao in regioes {
        raster::limpar(alvo, regiao);
        for e in elementos {
            if e.limites(skin).intersecao(&regiao).is_some() {
                e.desenhar(alvo, skin, regiao);
            }
        }
    }
}

/// RGBA direto (alfa não multiplicado) de um sprite sozinho, recortado a
/// `area` (pixels do monitor): o quadro esperado que o `/v1/debug/quadro`
/// entrega para a checagem de nitidez. Usa o mesmo caminho de desenho do
/// buffer de verdade.
pub fn rgba_do_sprite(skin: &Skin, sprite: &Elemento, area: Ret) -> Vec<u8> {
    let mut bgra = vec![0u8; (area.w.max(0) * area.h.max(0) * 4) as usize];
    {
        let mut alvo = Alvo::novo(&mut bgra, area.w, area.h);
        let local = match *sprite {
            Elemento::Sprite {
                quadro,
                x,
                y,
                d,
                espelhar,
            } => Elemento::Sprite {
                quadro,
                x: x - area.x,
                y: y - area.y,
                d,
                espelhar,
            },
            Elemento::Bloco { ret, cor } => Elemento::Bloco {
                ret: ret.deslocado(-area.x, -area.y),
                cor,
            },
        };
        let tudo = alvo.limites();
        local.desenhar(&mut alvo, skin, tudo);
    }
    bgra.chunks_exact(4)
        .flat_map(|p| {
            let a = p[3] as u32;
            let direto = |c: u8| match a {
                0 => 0,
                255 => c,
                _ => ((c as u32 * 255 + a / 2) / a).min(255) as u8,
            };
            [direto(p[2]), direto(p[1]), direto(p[0]), p[3]]
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::skin::testes::skin_minima;

    fn sprite(quadro: usize, x: i32) -> Elemento {
        Elemento::Sprite {
            quadro,
            x,
            y: 0,
            d: 2,
            espelhar: false,
        }
    }

    #[test]
    fn so_o_que_mudou_gera_dano() {
        let skin = skin_minima();
        let a = [
            sprite(0, 0),
            Elemento::Bloco {
                ret: Ret::novo(50, 50, 4, 4),
                cor: [1, 2, 3, 255],
            },
        ];
        let mut b = a;
        assert!(danos(&a, &b, &skin).is_empty());
        b[0] = sprite(0, 10);
        assert_eq!(
            danos(&a, &b, &skin),
            vec![Ret::novo(0, 0, 4, 4), Ret::novo(10, 0, 4, 4)]
        );
        // Mudou só o conteúdo, no mesmo lugar: um retângulo, não dois.
        let mut c = a;
        c[1] = Elemento::Bloco {
            ret: Ret::novo(50, 50, 4, 4),
            cor: [9, 9, 9, 255],
        };
        assert_eq!(danos(&a, &c, &skin), vec![Ret::novo(50, 50, 4, 4)]);
        // Elemento que some ou aparece entra só com o lado que existe.
        assert_eq!(danos(&a, &a[..1], &skin), vec![Ret::novo(50, 50, 4, 4)]);
    }

    #[test]
    fn redesenhar_limpa_o_antigo_e_respeita_a_ordem() {
        let skin = skin_minima();
        let mut dados = vec![0u8; 20 * 10 * 4];
        let mut alvo = Alvo::novo(&mut dados, 20, 10);
        let antes = [sprite(0, 0)];
        let tudo = alvo.limites();
        redesenhar(&mut alvo, &antes, &skin, &[tudo]);
        assert_ne!(alvo.pixel(0, 0), [0; 4]);
        // Move e põe um bloco por cima: o lugar antigo fica limpo e o bloco
        // (desenhado depois) cobre o sprite.
        let depois = [
            sprite(0, 10),
            Elemento::Bloco {
                ret: Ret::novo(10, 0, 1, 1),
                cor: [9, 9, 9, 255],
            },
        ];
        let regioes = danos(&antes, &depois, &skin);
        redesenhar(&mut alvo, &depois, &skin, &regioes);
        assert_eq!(alvo.pixel(0, 0), [0; 4]);
        assert_eq!(alvo.pixel(10, 0), [9, 9, 9, 255]);
        assert_eq!(alvo.pixel(11, 0), skin.folha.bgra_em(0, 0));
    }

    #[test]
    fn rgba_do_sprite_recortado_e_direto() {
        let skin = skin_minima();
        let s = Elemento::Sprite {
            quadro: 1,
            x: 100,
            y: 200,
            d: 3,
            espelhar: false,
        };
        // Célula em (100,200); quadro 1 em (2,2) da célula → (106..112, 206..212).
        let rgba = rgba_do_sprite(&skin, &s, Ret::novo(106, 206, 6, 6));
        assert_eq!(rgba.len(), 6 * 6 * 4);
        let opaco = &skin.folha.rgba[2 * 4..2 * 4 + 4];
        assert_eq!(&rgba[0..4], opaco);
        // O pixel de alfa 128 volta ao RGBA direto (com arredondamento).
        let i = ((3 * 6 + 3) * 4) as usize;
        assert_eq!(rgba[i + 3], 128);
        let original = &skin.folha.rgba[(4 + 3) * 4..(4 + 3) * 4 + 4];
        for c in 0..3 {
            assert!((rgba[i + c] as i32 - original[c] as i32).abs() <= 2);
        }
    }
}
