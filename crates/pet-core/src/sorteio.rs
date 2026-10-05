//! O sorteio do pet (decisão 0082): um gerador pequeno e determinístico
//! (xorshift64*) com a semente injetada. O núcleo não lê relógio nem o
//! sistema: o daemon semeia (pela hora da partida) e os testes usam a
//! semente fixa, para o que é sorteado (as micro-ações do ritmo quieto) ser o
//! mesmo a cada execução.

/// A semente de quem não semeia (os testes, o executor dos cenários).
pub const SEMENTE_PADRAO: u64 = 0x9E37_79B9_7F4A_7C15;

/// O estado do gerador. Nunca zero (o xorshift ficaria preso nele).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sorteio(u64);

impl Default for Sorteio {
    fn default() -> Sorteio {
        Sorteio::novo(SEMENTE_PADRAO)
    }
}

impl Sorteio {
    pub fn novo(semente: u64) -> Sorteio {
        Sorteio(if semente == 0 {
            SEMENTE_PADRAO
        } else {
            semente
        })
    }

    /// O próximo número (xorshift64*).
    pub fn proximo(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Um número de `min` a `max`, os dois inclusive.
    pub fn entre(&mut self, min: u64, max: u64) -> u64 {
        let (min, max) = (min.min(max), min.max(max));
        match (max - min).checked_add(1) {
            Some(faixa) => min + self.proximo() % faixa,
            None => self.proximo(),
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn determinista_na_faixa_e_espalhado() {
        let mut a = Sorteio::novo(42);
        let mut b = Sorteio::novo(42);
        let sa: Vec<u64> = (0..100).map(|_| a.entre(10_000, 30_000)).collect();
        let sb: Vec<u64> = (0..100).map(|_| b.entre(10_000, 30_000)).collect();
        assert_eq!(sa, sb, "a mesma semente, os mesmos números");
        assert!(sa.iter().all(|n| (10_000..=30_000).contains(n)));
        assert!(sa.iter().any(|n| *n < 15_000) && sa.iter().any(|n| *n > 25_000));
        assert_ne!(
            Sorteio::novo(1).proximo(),
            Sorteio::novo(2).proximo(),
            "outra semente, outra sequência"
        );
        // A semente zero não prende o gerador.
        let mut z = Sorteio::novo(0);
        assert_ne!(z.proximo(), 0);
        assert_eq!(Sorteio::novo(5).entre(7, 7), 7);
        assert!(Sorteio::novo(5).entre(9, 3) <= 9);
        Sorteio::novo(5).entre(0, u64::MAX);
    }
}
