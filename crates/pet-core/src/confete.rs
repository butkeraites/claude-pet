//! Confete: blocos coloridos que voam e quicam nas bordas.
//!
//! No M1 serve ao teste de estresse (`/v1/debug/estresse`), que mede o
//! custo no compositor de dezenas de retângulos pequenos mudando a 30 fps
//! pela tela inteira. Determinístico (semente fixa), para os testes.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pedaco {
    pub ret: Ret,
    pub vx: i32,
    pub vy: i32,
    pub cor: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chuva {
    pub pedacos: Vec<Pedaco>,
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

impl Chuva {
    /// `quantos` pedaços de `lado`×`lado` espalhados por `tela`.
    pub fn nova(quantos: usize, tela: (i32, i32), lado: i32, semente: u64) -> Chuva {
        let mut estado = semente | 1;
        let lado = lado.max(1);
        let pedacos = (0..quantos)
            .map(|i| {
                let x = entre(&mut estado, 0, (tela.0 - lado).max(0));
                let y = entre(&mut estado, 0, (tela.1 - lado).max(0));
                let mut vx = entre(&mut estado, -12, 12);
                let vy = entre(&mut estado, -12, 12);
                if vx == 0 {
                    vx = 3;
                }
                Pedaco {
                    ret: Ret::novo(x, y, lado, lado),
                    vx,
                    vy,
                    cor: CORES[i % CORES.len()],
                }
            })
            .collect();
        Chuva { pedacos }
    }

    /// Um passo: anda e quica nas bordas da tela.
    pub fn passo(&mut self, tela: (i32, i32)) {
        for p in &mut self.pedacos {
            let mut x = p.ret.x + p.vx;
            let mut y = p.ret.y + p.vy;
            let x_max = (tela.0 - p.ret.w).max(0);
            let y_max = (tela.1 - p.ret.h).max(0);
            if x < 0 || x > x_max {
                p.vx = -p.vx;
                x = x.clamp(0, x_max);
            }
            if y < 0 || y > y_max {
                p.vy = -p.vy;
                y = y.clamp(0, y_max);
            }
            p.ret.x = x;
            p.ret.y = y;
        }
    }

    pub fn elementos(&self) -> impl Iterator<Item = Elemento> + '_ {
        self.pedacos.iter().map(|p| Elemento::Bloco {
            ret: p.ret,
            cor: p.cor,
        })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn determinista_e_dentro_da_tela() {
        let a = Chuva::nova(40, (1920, 1200), 15, 7);
        assert_eq!(a, Chuva::nova(40, (1920, 1200), 15, 7));
        assert_eq!(a.pedacos.len(), 40);
        let mut c = a.clone();
        let tela = Ret::novo(0, 0, 1920, 1200);
        for _ in 0..1000 {
            c.passo((1920, 1200));
            for p in &c.pedacos {
                assert_eq!(p.ret.intersecao(&tela), Some(p.ret), "{p:?} saiu da tela");
            }
        }
        assert_ne!(a, c, "o confete andou");
    }

    #[test]
    fn quica_na_borda() {
        let mut c = Chuva {
            pedacos: vec![Pedaco {
                ret: Ret::novo(95, 0, 5, 5),
                vx: 10,
                vy: 0,
                cor: [0, 0, 0, 255],
            }],
        };
        c.passo((100, 100));
        assert_eq!(c.pedacos[0].ret.x, 95);
        assert_eq!(c.pedacos[0].vx, -10);
        c.passo((100, 100));
        assert_eq!(c.pedacos[0].ret.x, 85);
    }
}
