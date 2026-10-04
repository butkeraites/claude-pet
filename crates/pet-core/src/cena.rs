//! Cena: o que está no buffer, como lista de elementos em ordem de desenho.
//!
//! Entre dois quadros, o dano é o antes e o depois de cada elemento que
//! mudou (elementos casados pela posição na lista). Redesenhar uma região é
//! limpá-la e desenhar, em ordem, todo elemento que a cruza; assim regiões
//! sobrepostas e elementos sobrepostos sempre terminam corretos.

use crate::fonte;
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
    /// Um caractere da fonte dos balões ([`crate::fonte`]) com o canto em
    /// (x, y), cada pixel da fonte num bloco `d`×`d`, na cor `cor` (BGRA
    /// pré-multiplicado).
    Glifo {
        c: char,
        x: i32,
        y: i32,
        d: i32,
        cor: [u8; 4],
    },
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
            Elemento::Glifo { x, y, d, .. } => {
                Ret::novo(x, y, fonte::LARGURA_MAX * d, fonte::ALTURA * d)
            }
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
            Elemento::Glifo { c, x, y, d, cor } => desenhar_glifo(alvo, c, x, y, d, cor, recorte),
        }
    }
}

/// Um caractere da fonte em blocos `d`×`d`: cada trecho seguido de pixels
/// acesos numa linha vira um retângulo só.
fn desenhar_glifo(alvo: &mut Alvo, c: char, x: i32, y: i32, d: i32, cor: [u8; 4], recorte: Ret) {
    for (linha, bits) in fonte::glifo(c).iter().enumerate() {
        let mut coluna = 0;
        while coluna < fonte::LARGURA_MAX {
            if bits & (1 << coluna) == 0 {
                coluna += 1;
                continue;
            }
            let inicio = coluna;
            while coluna < fonte::LARGURA_MAX && bits & (1 << coluna) != 0 {
                coluna += 1;
            }
            let ret = Ret::novo(
                x + inicio * d,
                y + linha as i32 * d,
                (coluna - inicio) * d,
                d,
            );
            raster::preencher(alvo, ret, cor, recorte);
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
            Elemento::Glifo { c, x, y, d, cor } => Elemento::Glifo {
                c,
                x: x - area.x,
                y: y - area.y,
                d,
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
    fn glifo_em_blocos_inteiros_e_so_onde_a_fonte_acende() {
        let skin = skin_minima();
        let mut dados = vec![0u8; 30 * 40 * 4];
        let mut alvo = Alvo::novo(&mut dados, 30, 40);
        let cor = [9, 8, 7, 255];
        let a = Elemento::Glifo {
            c: 'a',
            x: 1,
            y: 2,
            d: 3,
            cor,
        };
        assert_eq!(a.limites(&skin), Ret::novo(1, 2, 24, 36));
        let tudo = alvo.limites();
        a.desenhar(&mut alvo, &skin, tudo);
        // O 'a' da monogram: a linha 5 é 30 (colunas 1 a 4); a 6 é 17
        // (colunas 0 e 4).
        let px = |alvo: &Alvo, col: i32, lin: i32| alvo.pixel(1 + col * 3, 2 + lin * 3);
        assert_eq!(px(&alvo, 1, 5), cor);
        assert_eq!(px(&alvo, 0, 5), [0; 4]);
        assert_eq!(px(&alvo, 0, 6), cor);
        assert_eq!(px(&alvo, 1, 6), [0; 4], "o miolo fica vazio");
        // Cada pixel da fonte é um bloco 3×3 uniforme.
        let canto = (1 + 3 * 3, 2 + 5 * 3);
        for dy in 0..3 {
            for dx in 0..3 {
                assert_eq!(alvo.pixel(canto.0 + dx, canto.1 + dy), cor);
            }
        }
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
