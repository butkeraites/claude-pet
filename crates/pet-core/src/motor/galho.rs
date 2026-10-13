//! Esconder no galho (decisão 0116): a máquina pura do poleiro.
//!
//! Arrastar com o galho ligado mostra um poleiro na borda esquerda, no meio
//! vertical (a zona de pouso), com uma silhueta translúcida do pet pousado e
//! um "Zzz" — o desenho diz "solte aqui que ele cochila e some". Quando o pet
//! entra na zona, o galho **acende** (o sinal de "pode soltar"). Soltar nele
//! leva o pet a assentar no galho, acenar tchau e sumir num poof; ele volta
//! pelo foco numa janela do Claude, pela escalada L1+ ou pelo `mostrar`. Nada
//! aqui toca em janela nem lê o relógio.
//!
//! O poleiro é desenhado pelo Motor (como os selos e o confete), então a
//! feature vale em qualquer skin, sem arte nova nem reaprovação (decisão 0116,
//! correção do híbrido); uma skin pode trazer a pose `perched` para o pet
//! pousado ficar mais bonito.

use crate::cena::Elemento;
use crate::geometria::Ret;

/// Quanto o pet fica assentado (pose `perched`) antes de acenar.
pub const ASSENTA_MS: u64 = 450;
/// Quanto dura o aceno de tchau antes do poof.
pub const TCHAU_MS: u64 = 750;
/// A janela inteira de "pousado" (assenta + tchau), antes do poof.
pub const POUSANDO_MS: u64 = ASSENTA_MS + TCHAU_MS;

/// A fase do esconder no galho.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Estado {
    /// Nada: o arraste é o de sempre.
    #[default]
    Nenhum,
    /// Arrastando com o galho ligado: o poleiro está na tela (a zona de pouso).
    Oferecido,
    /// Soltou no poleiro: assentado na pose (perched, depois o tchau), até
    /// `ate_ms` (→ Sumindo).
    Pousando { ate_ms: u64 },
    /// O poof de saída, o pet já invisível, de `inicio_ms` até `ate_ms`
    /// (→ Escondido).
    Sumindo { inicio_ms: u64, ate_ms: u64 },
    /// Escondido no galho, esperando a volta.
    Escondido,
}

impl Estado {
    /// O pet está fora da tela por causa do galho (não deve aparecer).
    pub fn escondido(self) -> bool {
        matches!(self, Estado::Escondido)
    }

    /// O poleiro está na tela (arrastando, pousando ou sumindo).
    pub fn poleiro_na_tela(self) -> bool {
        matches!(
            self,
            Estado::Oferecido | Estado::Pousando { .. } | Estado::Sumindo { .. }
        )
    }

    /// O pet está assentado na pose (perched/tchau): a volta larga a pose.
    pub fn pousando(self) -> bool {
        matches!(self, Estado::Pousando { .. })
    }

    /// O poof está tocando (o pet já sumiu, mas a nuvem ainda aparece): o
    /// instante em que começou.
    pub fn sumindo(self) -> Option<u64> {
        match self {
            Estado::Sumindo { inicio_ms, .. } => Some(inicio_ms),
            _ => None,
        }
    }

    /// O prazo da fase de agora (pousando ou sumindo).
    pub fn prazo(self) -> Option<u64> {
        match self {
            Estado::Pousando { ate_ms } | Estado::Sumindo { ate_ms, .. } => Some(ate_ms),
            _ => None,
        }
    }

    /// O nome para o `/v1/estado`.
    pub fn nome(self) -> &'static str {
        match self {
            Estado::Nenhum => "nenhum",
            Estado::Oferecido => "oferecido",
            Estado::Pousando { .. } => "pousando",
            Estado::Sumindo { .. } => "sumindo",
            Estado::Escondido => "escondido",
        }
    }
}

// O poleiro em pixels de arte (multiplicados por D no palco). Um galho
// horizontal saindo da borda esquerda, no meio vertical do monitor; sobre ele,
// na oferta, a silhueta translúcida do pet pousado e um "Zzz".
/// Largura do composto (a barra é larga o bastante para o pet pousar nela e
/// para o "Zzz" caber à direita da silhueta).
pub const LARGURA: i32 = 36;
/// Altura do composto (galho + silhueta + "Zzz"), em pixels de arte.
pub const ALTURA: i32 = 18;
/// A linha de cima da barra do galho, em pixels de arte (de onde o topo do
/// composto). O pet pousa com os pés nela.
pub const BARRA_Y: i32 = 16;
/// A margem da borda da área útil, em pixels de arte.
const MARGEM: i32 = 2;
/// A folga da zona de pouso em volta do galho, em pixels de arte.
const FOLGA: i32 = 8;

// A madeira do galho, apagado e aceso (BGRA pré-multiplicado).
const MARROM: [u8; 4] = [0x2B, 0x5A, 0x8B, 0xFF];
const MARROM_ESCURO: [u8; 4] = [0x1C, 0x3C, 0x5C, 0xFF];
const MARROM_ACESO: [u8; 4] = [0x4A, 0x8A, 0xC8, 0xFF];
const FOLHA: [u8; 4] = [0x3C, 0x82, 0x34, 0xFF];
const FOLHA_ACESA: [u8; 4] = [0x5C, 0xC0, 0x50, 0xFF];
// O brilho de "pode soltar": uma listra clara em cima da barra, só quando aceso.
const BRILHO: [u8; 4] = [0x9A, 0xD8, 0xF0, 0xFF];
// A silhueta translúcida do pet pousado (alfa < 255 → o `preencher` compõe com
// `sobre()`, então fica vazada na tela); mais forte quando aceso.
const SILHUETA: [u8; 4] = [0x60, 0x60, 0x60, 0x60];
const SILHUETA_ACESA: [u8; 4] = [0x96, 0x96, 0x96, 0x96];
// O "Zzz" no estilo do selo de soneca (creme com sombra de tinta), opaco.
const ZZ_FUNDO: [u8; 4] = [0xE3, 0xF1, 0xF7, 0xFF];
const ZZ_TINTA: [u8; 4] = [0x2A, 0x1B, 0x1D, 0xFF];

/// O retângulo todo do poleiro (barra, silhueta e "Zzz"), no palco, encostado
/// na borda esquerda da área útil e centrado na vertical.
pub fn poleiro(area: Ret, d: i32) -> Ret {
    let w = LARGURA * d;
    let h = ALTURA * d;
    let x = area.x + MARGEM * d;
    let y = area.y + (area.h - h) / 2;
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

/// Os elementos do galho desenhados pelo Motor, no palco.
///
/// `aceso`: o pet está sobre a zona (o galho brilha = "pode soltar"), ou já
/// pousou. `preview`: desenha a silhueta e o "Zzz" (só na oferta; pousado, o
/// próprio pet ocupa o galho).
pub fn elementos(area: Ret, d: i32, aceso: bool, preview: bool) -> Vec<Elemento> {
    let p = poleiro(area, d);
    let bloco = |ax: i32, ay: i32, aw: i32, ah: i32, cor: [u8; 4]| Elemento::Bloco {
        ret: Ret::novo(p.x + ax * d, p.y + ay * d, aw * d, ah * d),
        cor,
    };
    let madeira = if aceso { MARROM_ACESO } else { MARROM };
    let folha = if aceso { FOLHA_ACESA } else { FOLHA };
    let mut v = vec![
        // o galho: barra horizontal, larga o bastante para o pet pousar
        bloco(0, BARRA_Y, LARGURA, 2, madeira),
        // o toco de encaixe, à esquerda (de onde o galho "nasce" da parede)
        bloco(0, BARRA_Y - 2, 3, 4, MARROM_ESCURO),
        // folhas sobre a barra, à direita
        bloco(LARGURA - 6, BARRA_Y - 1, 2, 2, folha),
        bloco(LARGURA - 4, BARRA_Y - 2, 2, 2, folha),
        bloco(LARGURA - 3, BARRA_Y - 1, 2, 1, folha),
    ];
    if aceso {
        // a listra clara em cima da barra: "pode soltar aqui"
        v.push(bloco(0, BARRA_Y - 1, LARGURA, 1, BRILHO));
    }
    if preview {
        let sil = if aceso { SILHUETA_ACESA } else { SILHUETA };
        // a silhueta de um papagaio agachado (topete, cabeça redonda, bico
        // curvo, corpo e rabo): não um pato.
        v.push(bloco(7, 3, 3, 2, sil)); // topete
        v.push(bloco(5, 5, 7, 5, sil)); // cabeça
        v.push(bloco(12, 6, 2, 3, sil)); // bico curvo
        v.push(bloco(5, 10, 8, 6, sil)); // corpo
        v.push(bloco(3, 12, 3, 4, sil)); // rabo
        // o "Zzz" à direita da cabeça, subindo (como quem cochila). O `zy`
        // começa fundo o bastante para a letra mais alta não sair do poleiro.
        let gd = (d / 2 + 1).max(2);
        let zx = p.x + 15 * d;
        let zy = p.y + 4 * gd;
        for i in 0..3 {
            let cx = zx + i * crate::fonte::AVANCO * gd;
            let cy = zy - i * 2 * gd;
            v.push(Elemento::Glifo {
                c: 'z',
                x: cx + gd,
                y: cy + gd,
                d: gd,
                cor: ZZ_TINTA,
            });
            v.push(Elemento::Glifo {
                c: 'z',
                x: cx,
                y: cy,
                d: gd,
                cor: ZZ_FUNDO,
            });
        }
    }
    v
}

#[cfg(test)]
mod testes {
    use super::*;

    fn area() -> Ret {
        Ret::novo(0, 0, 1000, 800)
    }

    #[test]
    fn poleiro_na_esquerda_meio() {
        let d = 4;
        let a = area();
        let p = poleiro(a, d);
        // Encosta na margem da esquerda e fica centrado na vertical.
        assert_eq!(p.x, a.x + MARGEM * d);
        assert_eq!(p.y + p.h / 2, a.y + a.h / 2);
        assert_eq!(p.w, LARGURA * d);
        assert_eq!(p.h, ALTURA * d);
    }

    #[test]
    fn a_zona_e_maior_que_o_poleiro_e_pega_o_centro() {
        let d = 4;
        let a = area();
        let p = poleiro(a, d);
        assert!(na_zona(a, d, p.x + p.w / 2, p.y + p.h / 2));
        // Longe do poleiro (o canto inferior direito) não pega.
        assert!(!na_zona(a, d, a.direita() - 1, a.baixo() - 1));
    }

    #[test]
    fn os_elementos_cabem_no_poleiro() {
        let d = 4;
        let p = poleiro(area(), d);
        // A oferta (com silhueta e "Zzz"), acesa, é o caso com mais elementos.
        for e in elementos(area(), d, true, true) {
            let r = match e {
                Elemento::Bloco { ret, .. } => ret,
                Elemento::Glifo { x, y, d, .. } => Ret::novo(
                    x,
                    y,
                    crate::fonte::LARGURA_MAX * d,
                    crate::fonte::ALTURA * d,
                ),
                _ => panic!("o galho é feito de blocos e glifos"),
            };
            assert!(
                r.intersecao(&p) == Some(r),
                "o elemento {r:?} sai do poleiro {p:?}"
            );
        }
    }

    #[test]
    fn aceso_muda_o_desenho_e_preview_adiciona() {
        let d = 4;
        let a = area();
        let apagado = elementos(a, d, false, false);
        let aceso = elementos(a, d, true, false);
        assert!(aceso.len() > apagado.len(), "o brilho entra quando aceso");
        let com_preview = elementos(a, d, false, true);
        assert!(
            com_preview.len() > apagado.len(),
            "a silhueta e o Zzz entram no preview"
        );
        assert!(
            com_preview
                .iter()
                .any(|e| matches!(e, Elemento::Glifo { .. })),
            "o Zzz é glifo"
        );
    }
}
