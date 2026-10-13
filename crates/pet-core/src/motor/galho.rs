//! Esconder no galho (decisão 0116): a máquina pura do poleiro.
//!
//! Arrastar com o galho ligado mostra um poleiro numa borda (a zona de pouso);
//! soltar nele leva o pet a pousar (a pose `perched`, ou a de sempre se a skin
//! não a tem) e, depois de um instante, sumir. Ele volta pelo foco numa janela
//! do Claude, pela escalada L1+ ou pelo `mostrar`. Nada aqui toca em janela nem
//! lê o relógio.
//!
//! O poleiro é desenhado pelo Motor (como os selos e o confete), então a
//! feature vale em qualquer skin, sem arte nova nem reaprovação (decisão 0116,
//! correção do híbrido); uma skin pode trazer a pose `perched` para o pet
//! pousado ficar mais bonito.

use crate::cena::Elemento;
use crate::geometria::Ret;

/// Quanto tempo o pet fica pousado na pose antes de sumir.
pub const POUSO_MS: u64 = 900;

/// A fase do esconder no galho.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Estado {
    /// Nada: o arraste é o de sempre.
    #[default]
    Nenhum,
    /// Arrastando com o galho ligado: o poleiro está na tela (a zona de pouso).
    Oferecido,
    /// Soltou no poleiro: pousado (a pose), some em `ate_ms`.
    Pousando { ate_ms: u64 },
    /// Escondido no galho, esperando a volta.
    Escondido,
}

impl Estado {
    /// O pet está fora da tela por causa do galho (não deve aparecer).
    pub fn escondido(self) -> bool {
        matches!(self, Estado::Escondido)
    }

    /// O poleiro está na tela (arrastando ou pousando).
    pub fn poleiro_na_tela(self) -> bool {
        matches!(self, Estado::Oferecido | Estado::Pousando { .. })
    }

    /// O pet está pousado na pose.
    pub fn pousando(self) -> bool {
        matches!(self, Estado::Pousando { .. })
    }

    /// O prazo para o pet sumir, se pousando.
    pub fn prazo(self) -> Option<u64> {
        match self {
            Estado::Pousando { ate_ms } => Some(ate_ms),
            _ => None,
        }
    }

    /// O nome para o `/v1/estado`.
    pub fn nome(self) -> &'static str {
        match self {
            Estado::Nenhum => "nenhum",
            Estado::Oferecido => "oferecido",
            Estado::Pousando { .. } => "pousando",
            Estado::Escondido => "escondido",
        }
    }
}

// O poleiro em pixels de arte (multiplicados por D no palco). Um galho
// horizontal no alto, perto da direita, com um toco e umas folhas.
/// Largura do galho, em pixels de arte.
pub const LARGURA: i32 = 16;
/// Altura da parte desenhada (galho + folhas), em pixels de arte.
pub const ALTURA: i32 = 8;
/// A margem da borda da área útil, em pixels de arte.
const MARGEM: i32 = 2;
/// A folga da zona de pouso em volta do galho, em pixels de arte.
const FOLGA: i32 = 8;

const MARROM: [u8; 4] = [0x2B, 0x5A, 0x8B, 0xFF];
const MARROM_ESCURO: [u8; 4] = [0x1C, 0x3C, 0x5C, 0xFF];
const FOLHA: [u8; 4] = [0x46, 0xA0, 0x3C, 0xFF];

/// O retângulo todo do poleiro (galho e folhas), no palco, no canto superior
/// direito da área útil.
pub fn poleiro(area: Ret, d: i32) -> Ret {
    let w = LARGURA * d;
    let h = ALTURA * d;
    let x = area.direita() - MARGEM * d - w;
    let y = area.y + MARGEM * d;
    Ret::novo(x, y, w, h)
}

/// A zona de pouso: o poleiro com uma folga, para acertar mais fácil ao
/// soltar.
pub fn zona(area: Ret, d: i32) -> Ret {
    let p = poleiro(area, d);
    let f = FOLGA * d;
    Ret::novo(p.x - f, p.y - f, p.w + 2 * f, p.h + 2 * f)
}

/// O ponto (x, y) do palco caiu na zona de pouso.
pub fn na_zona(area: Ret, d: i32, x: i32, y: i32) -> bool {
    zona(area, d).contem(x, y)
}

/// Os blocos do galho desenhado pelo Motor, no palco.
pub fn elementos(area: Ret, d: i32) -> Vec<Elemento> {
    let p = poleiro(area, d);
    // O galho é uma barra horizontal de 2 px de arte, no terço de baixo; o
    // toco de encaixe à direita e três folhas acima, à esquerda.
    let bloco = |ax: i32, ay: i32, aw: i32, ah: i32, cor: [u8; 4]| Elemento::Bloco {
        ret: Ret::novo(p.x + ax * d, p.y + ay * d, aw * d, ah * d),
        cor,
    };
    vec![
        // o galho
        bloco(0, 5, LARGURA, 2, MARROM),
        // o toco de encaixe, à direita
        bloco(LARGURA - 3, 4, 3, 1, MARROM_ESCURO),
        // folhas
        bloco(1, 3, 2, 2, FOLHA),
        bloco(3, 2, 2, 2, FOLHA),
        bloco(5, 3, 2, 1, FOLHA),
    ]
}

#[cfg(test)]
mod testes {
    use super::*;

    fn area() -> Ret {
        Ret::novo(0, 0, 1000, 800)
    }

    #[test]
    fn poleiro_no_canto_superior_direito() {
        let d = 4;
        let p = poleiro(area(), d);
        // Encosta na margem de cima e fica a MARGEM*d da direita.
        assert_eq!(p.y, MARGEM * d);
        assert_eq!(p.direita(), area().direita() - MARGEM * d);
        assert_eq!(p.w, LARGURA * d);
        assert_eq!(p.h, ALTURA * d);
    }

    #[test]
    fn a_zona_e_maior_que_o_poleiro_e_pega_o_centro() {
        let d = 4;
        let p = poleiro(area(), d);
        assert!(na_zona(area(), d, p.x + p.w / 2, p.y + p.h / 2));
        // Fora da folga não pega.
        assert!(!na_zona(area(), d, p.x - (FOLGA + 1) * d, p.y + p.h / 2));
    }

    #[test]
    fn os_blocos_cabem_no_poleiro() {
        let d = 4;
        let p = poleiro(area(), d);
        for e in elementos(area(), d) {
            let Elemento::Bloco { ret, .. } = e else {
                panic!("o galho é feito de blocos");
            };
            assert!(
                ret.intersecao(&p) == Some(ret),
                "o bloco {ret:?} sai do poleiro {p:?}"
            );
        }
    }

    #[test]
    fn fases() {
        assert!(Estado::Escondido.escondido());
        assert!(!Estado::Pousando { ate_ms: 10 }.escondido());
        assert!(Estado::Oferecido.poleiro_na_tela());
        assert!(Estado::Pousando { ate_ms: 10 }.poleiro_na_tela());
        assert!(!Estado::Escondido.poleiro_na_tela());
        assert_eq!(Estado::Pousando { ate_ms: 42 }.prazo(), Some(42));
        assert_eq!(Estado::Nenhum.prazo(), None);
    }
}
