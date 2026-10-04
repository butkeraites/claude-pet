//! Raster em pixels do monitor (decisão 0004).
//!
//! O alvo é um buffer BGRA pré-multiplicado, que é exatamente o ARGB8888 do
//! `wl_shm` em little-endian. Cada pixel de arte vira um bloco D×D:
//! alfa 0 é pulado, alfa 255 é copiado, o resto é composto "over"
//! pré-multiplicado. Espelhar é só trocar o índice da coluna. Todo desenho é
//! recortado ao alvo e a uma região: quem chama redesenha só o que mudou.
//!
//! Dano: cada commit leva uma **lista** de retângulos (o antes e o depois de
//! cada elemento que mudou). Com mais de 128, eles viram uma grade de
//! ladrilhos de 64×64. Nunca um único retângulo envolvente: confete
//! espalhado viraria upload da tela inteira (revisão de viabilidade).

use crate::geometria::Ret;
use crate::skin::Skin;

/// Acima disto, os retângulos de dano viram ladrilhos.
pub const MAX_RETANGULOS: usize = 128;
/// Lado do ladrilho de dano, em pixels do monitor.
pub const LADRILHO: i32 = 64;

/// Um buffer BGRA pré-multiplicado.
pub struct Alvo<'a> {
    pub dados: &'a mut [u8],
    pub largura: i32,
    pub altura: i32,
    /// Bytes por linha.
    pub passo: usize,
}

impl<'a> Alvo<'a> {
    pub fn novo(dados: &'a mut [u8], largura: i32, altura: i32) -> Alvo<'a> {
        Alvo {
            dados,
            largura,
            altura,
            passo: largura as usize * 4,
        }
    }

    pub fn limites(&self) -> Ret {
        Ret::novo(0, 0, self.largura, self.altura)
    }

    fn linha(&mut self, y: i32, x0: i32, x1: i32) -> &mut [u8] {
        let inicio = y as usize * self.passo + x0 as usize * 4;
        &mut self.dados[inicio..inicio + (x1 - x0) as usize * 4]
    }

    /// Pixel BGRA em (x, y).
    pub fn pixel(&self, x: i32, y: i32) -> [u8; 4] {
        let i = y as usize * self.passo + x as usize * 4;
        [
            self.dados[i],
            self.dados[i + 1],
            self.dados[i + 2],
            self.dados[i + 3],
        ]
    }
}

/// "Over" pré-multiplicado: `dst = src + dst × (255 − αsrc) / 255`.
#[inline]
pub fn sobre(dst: [u8; 4], src: [u8; 4]) -> [u8; 4] {
    let resto = 255 - src[3] as u32;
    let c = |s: u8, d: u8| (s as u32 + (d as u32 * resto + 127) / 255).min(255) as u8;
    [
        c(src[0], dst[0]),
        c(src[1], dst[1]),
        c(src[2], dst[2]),
        c(src[3], dst[3]),
    ]
}

/// Deixa o retângulo transparente.
pub fn limpar(alvo: &mut Alvo, r: Ret) {
    let Some(r) = r.intersecao(&alvo.limites()) else {
        return;
    };
    for y in r.y..r.baixo() {
        alvo.linha(y, r.x, r.direita()).fill(0);
    }
}

/// Pinta o retângulo com uma cor BGRA pré-multiplicada, por cima do que
/// houver.
pub fn preencher(alvo: &mut Alvo, r: Ret, cor: [u8; 4], recorte: Ret) {
    if cor[3] == 0 {
        return;
    }
    let Some(r) = r
        .intersecao(&recorte)
        .and_then(|r| r.intersecao(&alvo.limites()))
    else {
        return;
    };
    for y in r.y..r.baixo() {
        let linha = alvo.linha(y, r.x, r.direita());
        for px in linha.chunks_exact_mut(4) {
            let novo = if cor[3] == 255 {
                cor
            } else {
                sobre([px[0], px[1], px[2], px[3]], cor)
            };
            px.copy_from_slice(&novo);
        }
    }
}

/// Retângulo (pixels do monitor) que o quadro `q` ocupa com a célula em
/// (x, y) e escala `d`.
pub fn limites_do_quadro(skin: &Skin, q: usize, x: i32, y: i32, d: i32, espelhar: bool) -> Ret {
    let na_celula = skin.quadros[q].na_celula();
    let cx = if espelhar {
        skin.ancoras.celula.0 - na_celula.direita()
    } else {
        na_celula.x
    };
    Ret::novo(
        x + cx * d,
        y + na_celula.y * d,
        na_celula.w * d,
        na_celula.h * d,
    )
}

/// Desenha o quadro `q` da folha com a célula em (x, y): cada pixel de arte
/// é um bloco D×D. Só toca pixels dentro de `recorte` (e do alvo).
#[allow(clippy::too_many_arguments)]
pub fn desenhar_quadro(
    alvo: &mut Alvo,
    skin: &Skin,
    q: usize,
    x: i32,
    y: i32,
    d: i32,
    espelhar: bool,
    recorte: Ret,
) {
    if d <= 0 {
        return;
    }
    let quadro = skin.quadros[q];
    let limites = limites_do_quadro(skin, q, x, y, d, espelhar);
    let Some(area) = limites
        .intersecao(&recorte)
        .and_then(|r| r.intersecao(&alvo.limites()))
    else {
        return;
    };
    let folha = &skin.folha;
    // Colunas de arte (já espelhadas) que cruzam a área, com o trecho de
    // pixels do monitor de cada uma.
    let mut colunas: Vec<(i32, i32, i32)> = Vec::with_capacity(quadro.origem.w as usize);
    for u in 0..quadro.origem.w {
        // Coluna u da folha cai na coluna `u` (ou espelhada) do retângulo.
        let destino = if espelhar { quadro.origem.w - 1 - u } else { u };
        let x0 = (limites.x + destino * d).max(area.x);
        let x1 = (limites.x + destino * d + d).min(area.direita());
        if x1 > x0 {
            colunas.push((u, x0, x1));
        }
    }
    for linha_dispositivo in area.y..area.baixo() {
        let v = (linha_dispositivo - limites.y) / d;
        let fy = quadro.origem.y + v;
        let linha = alvo.linha(linha_dispositivo, area.x, area.direita());
        for &(u, x0, x1) in &colunas {
            let px = folha.bgra_em(quadro.origem.x + u, fy);
            if px[3] == 0 {
                continue;
            }
            let ini = (x0 - area.x) as usize * 4;
            let fim = (x1 - area.x) as usize * 4;
            for destino in linha[ini..fim].chunks_exact_mut(4) {
                let novo = if px[3] == 255 {
                    px
                } else {
                    sobre([destino[0], destino[1], destino[2], destino[3]], px)
                };
                destino.copy_from_slice(&novo);
            }
        }
    }
}

/// Recorta os danos ao alvo e, se forem muitos, agrupa em ladrilhos de
/// 64×64 (juntando ladrilhos vizinhos de uma mesma linha). Nunca devolve um
/// retângulo envolvente de tudo.
pub fn consolidar_danos(danos: &[Ret], limites: Ret) -> Vec<Ret> {
    let recortados: Vec<Ret> = danos
        .iter()
        .filter_map(|r| r.intersecao(&limites))
        .collect();
    if recortados.len() <= MAX_RETANGULOS {
        return recortados;
    }
    let colunas = (limites.w + LADRILHO - 1) / LADRILHO;
    let linhas = (limites.h + LADRILHO - 1) / LADRILHO;
    let mut marcados = vec![false; (colunas * linhas) as usize];
    for r in &recortados {
        let c0 = (r.x - limites.x) / LADRILHO;
        let c1 = (r.direita() - 1 - limites.x) / LADRILHO;
        let l0 = (r.y - limites.y) / LADRILHO;
        let l1 = (r.baixo() - 1 - limites.y) / LADRILHO;
        for l in l0..=l1 {
            for c in c0..=c1 {
                marcados[(l * colunas + c) as usize] = true;
            }
        }
    }
    let mut saida = Vec::new();
    for l in 0..linhas {
        let mut c = 0;
        while c < colunas {
            if !marcados[(l * colunas + c) as usize] {
                c += 1;
                continue;
            }
            let inicio = c;
            while c < colunas && marcados[(l * colunas + c) as usize] {
                c += 1;
            }
            let faixa = Ret::novo(
                limites.x + inicio * LADRILHO,
                limites.y + l * LADRILHO,
                (c - inicio) * LADRILHO,
                LADRILHO,
            );
            if let Some(faixa) = faixa.intersecao(&limites) {
                saida.push(faixa);
            }
        }
    }
    saida
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::skin::testes::skin_minima;

    fn buffer(l: i32, a: i32) -> Vec<u8> {
        vec![0; (l * a * 4) as usize]
    }

    #[test]
    fn blocos_6x6_exatos_e_uniformes() {
        let skin = skin_minima();
        let mut dados = buffer(30, 30);
        let mut alvo = Alvo::novo(&mut dados, 30, 30);
        let tudo = alvo.limites();
        desenhar_quadro(&mut alvo, &skin, 0, 3, 5, 6, false, tudo);
        // Quadro 0: pixels (0..2, 0..2) da folha nos blocos (3..15, 5..17).
        for by in 0..2 {
            for bx in 0..2 {
                let esperado = skin.folha.bgra_em(bx, by);
                for dy in 0..6 {
                    for dx in 0..6 {
                        assert_eq!(alvo.pixel(3 + bx * 6 + dx, 5 + by * 6 + dy), esperado);
                    }
                }
            }
        }
        // Fora do quadro continua transparente.
        assert_eq!(alvo.pixel(2, 5), [0; 4]);
        assert_eq!(alvo.pixel(15, 5), [0; 4]);
        assert_eq!(alvo.pixel(3, 17), [0; 4]);
    }

    #[test]
    fn quadro_recortado_usa_o_deslocamento_na_celula() {
        let skin = skin_minima();
        let mut dados = buffer(30, 30);
        let mut alvo = Alvo::novo(&mut dados, 30, 30);
        let tudo = alvo.limites();
        desenhar_quadro(&mut alvo, &skin, 1, 0, 0, 2, false, tudo);
        // Quadro 1 começa em (2,2) da célula → pixels (4..8, 4..8).
        assert_eq!(alvo.pixel(3, 3), [0; 4]);
        assert_eq!(alvo.pixel(4, 4), skin.folha.bgra_em(2, 0));
        assert_eq!(
            limites_do_quadro(&skin, 1, 0, 0, 2, false),
            Ret::novo(4, 4, 4, 4)
        );
    }

    #[test]
    fn espelhar_troca_as_colunas() {
        let skin = skin_minima();
        let mut dados = buffer(8, 8);
        let mut alvo = Alvo::novo(&mut dados, 8, 8);
        let tudo = alvo.limites();
        desenhar_quadro(&mut alvo, &skin, 0, 0, 0, 1, true, tudo);
        // Célula 4 de largura, quadro 2x2 em (0,0): espelhado vai para x 2..4,
        // com a coluna 0 da folha em x=3 e a 1 em x=2.
        assert_eq!(alvo.pixel(3, 0), skin.folha.bgra_em(0, 0));
        assert_eq!(alvo.pixel(2, 0), skin.folha.bgra_em(1, 0));
        assert_eq!(alvo.pixel(0, 0), [0; 4]);
        assert_eq!(
            limites_do_quadro(&skin, 0, 0, 0, 1, true),
            Ret::novo(2, 0, 2, 2)
        );
    }

    #[test]
    fn recorte_na_borda_e_fora_da_tela() {
        let skin = skin_minima();
        let mut dados = buffer(10, 10);
        let mut alvo = Alvo::novo(&mut dados, 10, 10);
        let tudo = alvo.limites();
        // Metade para fora à esquerda e acima: não pode estourar índice.
        desenhar_quadro(&mut alvo, &skin, 0, -6, -6, 6, false, tudo);
        assert_eq!(alvo.pixel(0, 0), skin.folha.bgra_em(1, 1));
        assert_eq!(alvo.pixel(5, 5), skin.folha.bgra_em(1, 1));
        assert_eq!(alvo.pixel(6, 6), [0; 4]);
        // Totalmente fora.
        desenhar_quadro(&mut alvo, &skin, 0, 50, 50, 6, false, tudo);
        // Recorte parcial: só a região pedida é tocada.
        let mut dados2 = buffer(20, 20);
        let mut alvo2 = Alvo::novo(&mut dados2, 20, 20);
        desenhar_quadro(&mut alvo2, &skin, 0, 0, 0, 6, false, Ret::novo(0, 0, 3, 20));
        assert_eq!(alvo2.pixel(2, 0), skin.folha.bgra_em(0, 0));
        assert_eq!(alvo2.pixel(3, 0), [0; 4]);
    }

    #[test]
    fn over_premultiplicado() {
        // Meio-transparente sobre opaco.
        assert_eq!(sobre([0, 0, 200, 255], [50, 0, 0, 128]), [50, 0, 100, 255]);
        // Opaco cobre; transparente não muda.
        assert_eq!(sobre([1, 2, 3, 255], [9, 9, 9, 255]), [9, 9, 9, 255]);
        assert_eq!(sobre([1, 2, 3, 255], [0, 0, 0, 0]), [1, 2, 3, 255]);
        // No quadro: o pixel (3,1) tem alfa 128 e compõe sobre o fundo.
        let skin = skin_minima();
        let mut dados = buffer(4, 4);
        let mut alvo = Alvo::novo(&mut dados, 4, 4);
        preencher(
            &mut alvo,
            Ret::novo(0, 0, 4, 4),
            [0, 0, 255, 255],
            Ret::novo(0, 0, 4, 4),
        );
        let tudo = alvo.limites();
        desenhar_quadro(&mut alvo, &skin, 1, 0, 0, 1, false, tudo);
        let src = skin.folha.bgra_em(3, 1);
        assert_eq!(alvo.pixel(3, 3), sobre([0, 0, 255, 255], src));
    }

    #[test]
    fn limpar_e_preencher_respeitam_os_limites() {
        let mut dados = vec![9u8; 4 * 4 * 4];
        let mut alvo = Alvo::novo(&mut dados, 4, 4);
        limpar(&mut alvo, Ret::novo(-2, -2, 4, 4));
        assert_eq!(alvo.pixel(1, 1), [0; 4]);
        assert_eq!(alvo.pixel(2, 2), [9; 4]);
        preencher(
            &mut alvo,
            Ret::novo(2, 2, 9, 9),
            [1, 2, 3, 255],
            Ret::novo(0, 0, 3, 3),
        );
        assert_eq!(alvo.pixel(2, 2), [1, 2, 3, 255]);
        assert_eq!(alvo.pixel(3, 3), [9; 4]);
    }

    #[test]
    fn danos_em_lista_e_em_ladrilhos() {
        let tela = Ret::novo(0, 0, 1920, 1200);
        let poucos = vec![Ret::novo(10, 10, 5, 5), Ret::novo(-5, -5, 10, 10)];
        assert_eq!(
            consolidar_danos(&poucos, tela),
            vec![Ret::novo(10, 10, 5, 5), Ret::novo(0, 0, 5, 5)]
        );
        // 200 confetes num canto da tela: viram ladrilhos só em volta deles,
        // nunca um retângulo da tela toda.
        let muitos: Vec<Ret> = (0..200)
            .map(|i| Ret::novo(100 + (i * 97) % 588, 100 + (i * 61) % 388, 12, 12))
            .collect();
        let ladrilhos = consolidar_danos(&muitos, tela);
        assert!(ladrilhos.len() < muitos.len());
        let vizinhanca = Ret::novo(64, 64, 704, 448);
        for l in &ladrilhos {
            assert_eq!(
                l.intersecao(&vizinhanca),
                Some(*l),
                "{l:?} longe dos confetes"
            );
        }
        let area: i64 = ladrilhos.iter().map(Ret::area).sum();
        assert!(
            area <= vizinhanca.area() && area < tela.area() / 4,
            "área {area}"
        );
        for r in &muitos {
            for (x, y) in [
                (r.x, r.y),
                (r.direita() - 1, r.y),
                (r.x, r.baixo() - 1),
                (r.direita() - 1, r.baixo() - 1),
            ] {
                assert!(ladrilhos.iter().any(|l| l.contem(x, y)), "{r:?} sem dano");
            }
        }
        // Ladrilhos da borda são recortados à tela (1200 não é múltiplo de 64).
        let na_borda = consolidar_danos(
            &(0..130)
                .map(|i| Ret::novo(i * 14, 1190, 10, 10))
                .collect::<Vec<_>>(),
            tela,
        );
        assert!(
            na_borda
                .iter()
                .all(|l| l.baixo() <= 1200 && l.direita() <= 1920)
        );
        assert!(na_borda.iter().any(|l| l.baixo() == 1200));
    }
}
