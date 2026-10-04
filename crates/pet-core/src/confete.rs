//! Confete: blocos coloridos que voam e quicam nas bordas.
//!
//! No M1 serve ao teste de estresse (`/v1/debug/estresse`), que mede o
//! custo no compositor de dezenas de retângulos pequenos mudando a 30 fps
//! pela tela inteira. Determinístico (semente fixa), para os testes.
//!
//! Tudo anda na **grade de arte** do pet (decisão 0004): posição, tamanho e
//! velocidade são em pixels de arte, e cada pixel de arte é um bloco D×D a
//! partir da origem da grade (o canto da célula do pet). Assim o confete
//! nunca sai do passo da arte (sem "mixels").

use crate::cena::Elemento;
use crate::geometria::Ret;

/// Cores do confete, BGRA pré-multiplicado (todas opacas).
const CORES: [[u8; 4]; 6] = [
    [60, 60, 230, 255],
    [60, 200, 250, 255],
    [90, 210, 90, 255],
    [230, 140, 60, 255],
    [200, 80, 200, 255],
    [255, 255, 255, 255],
];

/// Velocidade máxima, em pixels de arte por passo.
const VELOCIDADE_MAX: i32 = 3;

/// A grade de arte: origem em pixels do monitor e D (pixels do monitor por
/// pixel de arte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grade {
    pub x: i32,
    pub y: i32,
    pub d: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pedaco {
    /// Canto do pedaço em pixels de arte, relativo à origem da grade.
    pub i: i32,
    pub j: i32,
    /// Velocidade em pixels de arte por passo.
    pub vi: i32,
    pub vj: i32,
    pub cor: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chuva {
    pub pedacos: Vec<Pedaco>,
    grade: Grade,
    /// Lado do pedaço em pixels de arte.
    lado: i32,
    /// Faixa de `i` e de `j` em que o pedaço cabe inteiro na tela.
    faixa_i: (i32, i32),
    faixa_j: (i32, i32),
}

/// xorshift64: suficiente para espalhar confete.
fn proximo(estado: &mut u64) -> u64 {
    let mut x = *estado;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *estado = x;
    x
}

fn entre(estado: &mut u64, min: i32, max: i32) -> i32 {
    min + (proximo(estado) % (max - min + 1) as u64) as i32
}

/// Índices de arte `k` com `origem + k*d` em `0..=limite` (pixels do
/// monitor). Se não couber nenhum, fica só o primeiro.
fn faixa(origem: i32, limite: i32, d: i32) -> (i32, i32) {
    // menor k com origem + k·d ≥ 0: ⌈−origem/d⌉ = −⌊origem/d⌋.
    let min = -(origem.div_euclid(d));
    // maior k com origem + k·d ≤ limite.
    let max = (limite - origem).div_euclid(d);
    (min, max.max(min))
}

impl Chuva {
    /// `quantos` pedaços de `lado`×`lado` pixels de arte, espalhados pela
    /// tela (`tela` em pixels do monitor) na grade `grade`.
    pub fn nova(quantos: usize, tela: (i32, i32), grade: Grade, lado: i32, semente: u64) -> Chuva {
        let grade = Grade {
            d: grade.d.max(1),
            ..grade
        };
        let lado = lado.max(1);
        let lado_px = lado * grade.d;
        let faixa_i = faixa(grade.x, tela.0 - lado_px, grade.d);
        let faixa_j = faixa(grade.y, tela.1 - lado_px, grade.d);
        let mut estado = semente | 1;
        let pedacos = (0..quantos)
            .map(|n| {
                let i = entre(&mut estado, faixa_i.0, faixa_i.1);
                let j = entre(&mut estado, faixa_j.0, faixa_j.1);
                let mut vi = entre(&mut estado, -VELOCIDADE_MAX, VELOCIDADE_MAX);
                let vj = entre(&mut estado, -VELOCIDADE_MAX, VELOCIDADE_MAX);
                if vi == 0 {
                    vi = 1;
                }
                Pedaco {
                    i,
                    j,
                    vi,
                    vj,
                    cor: CORES[n % CORES.len()],
                }
            })
            .collect();
        Chuva {
            pedacos,
            grade,
            lado,
            faixa_i,
            faixa_j,
        }
    }

    /// Um passo: anda e quica nas bordas da tela.
    pub fn passo(&mut self) {
        let quicar = |pos: &mut i32, vel: &mut i32, (min, max): (i32, i32)| {
            let novo = *pos + *vel;
            if novo < min || novo > max {
                *vel = -*vel;
                *pos = novo.clamp(min, max);
            } else {
                *pos = novo;
            }
        };
        for p in &mut self.pedacos {
            quicar(&mut p.i, &mut p.vi, self.faixa_i);
            quicar(&mut p.j, &mut p.vj, self.faixa_j);
        }
    }

    /// O pedaço em pixels do monitor.
    pub fn retangulo(&self, p: &Pedaco) -> Ret {
        let g = self.grade;
        Ret::novo(
            g.x + p.i * g.d,
            g.y + p.j * g.d,
            self.lado * g.d,
            self.lado * g.d,
        )
    }

    pub fn elementos(&self) -> impl Iterator<Item = Elemento> + '_ {
        self.pedacos.iter().map(|p| Elemento::Bloco {
            ret: self.retangulo(p),
            cor: p.cor,
        })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const GRADE: Grade = Grade {
        x: 1706,
        y: 951,
        d: 5,
    };

    #[test]
    fn determinista_dentro_da_tela_e_na_grade() {
        let a = Chuva::nova(40, (1920, 1200), GRADE, 3, 7);
        assert_eq!(a, Chuva::nova(40, (1920, 1200), GRADE, 3, 7));
        assert_eq!(a.pedacos.len(), 40);
        let mut c = a.clone();
        let tela = Ret::novo(0, 0, 1920, 1200);
        for _ in 0..1000 {
            c.passo();
            for p in &c.pedacos {
                let r = c.retangulo(p);
                assert_eq!(r.intersecao(&tela), Some(r), "{p:?} saiu da tela");
                assert_eq!((r.x - GRADE.x).rem_euclid(5), 0, "{r:?} fora da grade");
                assert_eq!((r.y - GRADE.y).rem_euclid(5), 0, "{r:?} fora da grade");
                assert_eq!((r.w, r.h), (15, 15));
            }
        }
        assert_ne!(a, c, "o confete andou");
    }

    #[test]
    fn faixa_cobre_a_tela_inteira_na_grade() {
        // Origem 1706 com D=5: o primeiro x da grade ≥ 0 é 1 (1706 = 341·5 + 1).
        assert_eq!(faixa(1706, 1905, 5), (-341, 39));
        assert_eq!(1706 - 341 * 5, 1);
        assert_eq!(1706 + 39 * 5, 1901);
        assert_eq!(faixa(0, 100, 10), (0, 10));
        // Tela menor que o pedaço: fica um índice só.
        assert_eq!(faixa(0, -5, 10), (0, 0));
    }

    #[test]
    fn quica_na_borda() {
        let mut c = Chuva::nova(1, (100, 100), Grade { x: 0, y: 0, d: 5 }, 1, 3);
        c.pedacos[0] = Pedaco {
            i: 18,
            j: 0,
            vi: 2,
            vj: 0,
            cor: [0, 0, 0, 255],
        };
        // 100 px de tela, pedaço de 5: i vai até 19.
        c.passo();
        assert_eq!((c.pedacos[0].i, c.pedacos[0].vi), (19, -2));
        c.passo();
        assert_eq!(c.pedacos[0].i, 17);
    }
}
