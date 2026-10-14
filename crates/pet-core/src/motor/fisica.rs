//! Física do arraste (M6, T6.6, decisão 0128): o **quique** do pet ao ser
//! solto — um repique vertical curto e amortecido antes de assentar.
//!
//! Tudo em passos da grade de arte (múltiplos de D): a pose fica nítida
//! (decisão 0004), e o repique anda na grade de 34 ms do desenho (decisão
//! 0088). O pêndulo ao pendurar e a poeira do pouso ficam para itens próprios
//! (a poeira é partícula de tela, que espera o palco transitório do M8).

/// Um quadro do quique: 34 ms, a grade das rajadas (decisão 0088).
pub const PASSO_MS: u64 = 34;
/// O deslocamento vertical de cada quadro, em **pixels de arte** (negativo =
/// para cima): o pet repica e assenta. O último é 0 (pousado).
const QUIQUE: [i32; 5] = [-2, -3, -2, -1, 0];

/// Quanto o quique dura, do soltar até assentar.
pub const QUIQUE_MS: u64 = QUIQUE.len() as u64 * PASSO_MS;

/// O deslocamento vertical do quique (pixels de arte, para cima) `decorrido_ms`
/// depois do soltar; `None` quando já assentou.
pub fn quique_offset_d(decorrido_ms: u64) -> Option<i32> {
    QUIQUE.get((decorrido_ms / PASSO_MS) as usize).copied()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_quique_repica_para_cima_e_assenta() {
        // Começa subindo, chega ao pico e amortece até 0; depois, nada.
        assert_eq!(quique_offset_d(0), Some(-2));
        assert_eq!(quique_offset_d(PASSO_MS), Some(-3), "o pico");
        assert_eq!(quique_offset_d(3 * PASSO_MS), Some(-1));
        assert_eq!(quique_offset_d(4 * PASSO_MS), Some(0), "assentou");
        assert_eq!(quique_offset_d(QUIQUE_MS), None, "acabou");
        assert_eq!(quique_offset_d(10_000), None);
        // Só sobe (nunca desce abaixo do pouso) e amortece (o pico não volta).
        let seq: Vec<i32> = (0..QUIQUE.len() as u64)
            .map(|i| quique_offset_d(i * PASSO_MS).unwrap())
            .collect();
        assert!(seq.iter().all(|&o| o <= 0), "só para cima: {seq:?}");
        assert_eq!(*seq.last().unwrap(), 0, "termina pousado");
    }
}
