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
/// para cima): o pet repica alto, desce, dá um repique menor e assenta (um
/// duplo quique, bem perceptível). O último é 0 (pousado).
const QUIQUE: [i32; 7] = [-4, -7, -5, -1, -3, -1, 0];

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
        // Começa subindo, tem um pico, e termina pousado (0); depois, nada.
        assert!(quique_offset_d(0).unwrap() < 0, "começa subindo");
        assert_eq!(quique_offset_d(QUIQUE_MS), None, "acabou");
        assert_eq!(quique_offset_d(10_000), None);
        let seq: Vec<i32> = (0..QUIQUE.len() as u64)
            .map(|i| quique_offset_d(i * PASSO_MS).unwrap())
            .collect();
        // Só sobe (nunca abaixo do pouso), tem um pico nítido e assenta no 0.
        assert!(seq.iter().all(|&o| o <= 0), "só para cima: {seq:?}");
        assert!(
            seq.iter().min().unwrap() <= &-5,
            "pico bem visível: {seq:?}"
        );
        assert_eq!(*seq.last().unwrap(), 0, "termina pousado");
        // Duplo quique: depois de descer, torna a subir um pouco (segundo pico).
        let vale = seq
            .iter()
            .position(|&o| o == *seq.iter().min().unwrap())
            .unwrap();
        assert!(
            seq[vale..].windows(2).any(|w| w[1] < w[0]),
            "um segundo repique: {seq:?}"
        );
    }
}
