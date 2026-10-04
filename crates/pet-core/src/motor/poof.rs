//! O "poof" da troca de monitor (M4, decisão 0051): uma nuvenzinha
//! procedural em volta do corpo do pet, em blocos de D (nítida como a arte),
//! sem tag nenhuma da skin (a receita do catálogo para `poof_in` e
//! `poof_out`). O M6 troca por partículas de verdade.
//!
//! Quatro passos de 60 ms (≈ 17 quadros por segundo, dentro da rajada de até
//! 30): na saída o pet some no segundo passo e a nuvem abre; na chegada ele
//! aparece no segundo passo e a nuvem fecha.

use crate::cena::Elemento;
use crate::geometria::Ret;

/// Duração de um poof.
pub const DURACAO_MS: u64 = 240;
/// Passos do poof.
pub const PASSOS: u64 = 4;
/// Duração de um passo.
pub const PASSO_MS: u64 = DURACAO_MS / PASSOS;

/// Branco gelo e cinza claro, opacos (BGRA pré-multiplicado).
const CLARO: [u8; 4] = [0xF0, 0xF4, 0xF4, 0xFF];
const CINZA: [u8; 4] = [0xD4, 0xCA, 0xC6, 0xFF];

/// As oito direções da nuvem.
const DIRECOES: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

/// O passo em `agora_ms` de um poof que começou em `inicio_ms` (`None`: já
/// acabou) e quando vem o próximo.
pub fn passo(inicio_ms: u64, agora_ms: u64) -> Option<(u64, u64)> {
    let decorrido = agora_ms.saturating_sub(inicio_ms);
    let passo = decorrido / PASSO_MS;
    (passo < PASSOS).then(|| (passo, inicio_ms + (passo + 1) * PASSO_MS))
}

/// O pet aparece neste passo (na saída só no primeiro; na chegada a partir
/// do segundo).
pub fn mostra_o_pet(passo: u64, entrando: bool) -> bool {
    if entrando { passo >= 1 } else { passo == 0 }
}

/// Os bloquinhos da nuvem em volta do `corpo` (no palco), com D `d`, no
/// `passo`.
pub fn nuvem(corpo: Ret, d: i32, passo: u64, entrando: bool) -> Vec<Elemento> {
    let d = d.max(1);
    // Raio e lado em pixels de arte: na saída abre e afina; na chegada
    // fecha e afina.
    let (raio, lado) = match (entrando, passo) {
        (false, 0) => (2, 3),
        (false, 1) => (5, 3),
        (false, 2) => (8, 2),
        (false, _) => (10, 1),
        (true, 0) => (9, 3),
        (true, 1) => (6, 3),
        (true, 2) => (4, 2),
        (true, _) => (3, 1),
    };
    let cx = corpo.x + corpo.w / 2;
    let cy = corpo.y + corpo.h / 2;
    // O meio do corpo arredondado para a grade de D a partir do canto dele.
    let cx = corpo.x + (cx - corpo.x) / d * d;
    let cy = corpo.y + (cy - corpo.y) / d * d;
    let meia = corpo.w.max(corpo.h) / 2 / d;
    DIRECOES
        .iter()
        .enumerate()
        .map(|(i, &(dx, dy))| {
            let r = meia + raio;
            let x = cx + dx * r * d - lado * d / 2;
            let y = cy + dy * r * d - lado * d / 2;
            Elemento::Bloco {
                ret: Ret::novo(x, y, lado * d, lado * d),
                cor: if i % 2 == 0 { CLARO } else { CINZA },
            }
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn passos_de_60_ms_e_o_fim() {
        assert_eq!(passo(1_000, 1_000), Some((0, 1_060)));
        assert_eq!(passo(1_000, 1_059), Some((0, 1_060)));
        assert_eq!(passo(1_000, 1_060), Some((1, 1_120)));
        assert_eq!(passo(1_000, 1_239), Some((3, 1_240)));
        assert_eq!(passo(1_000, 1_240), None, "acabou");
        assert!(mostra_o_pet(0, false) && !mostra_o_pet(1, false));
        assert!(!mostra_o_pet(0, true) && mostra_o_pet(1, true));
    }

    #[test]
    fn nuvem_em_blocos_de_d_abrindo_na_saida() {
        let corpo = Ret::novo(1_700, 1_000, 114, 114);
        let d = 6;
        let perto = nuvem(corpo, d, 0, false);
        let longe = nuvem(corpo, d, 3, false);
        assert_eq!(perto.len(), 8);
        let distancia = |e: &Elemento| match e {
            Elemento::Bloco { ret, .. } => (ret.x - corpo.x).abs() + (ret.y - corpo.y).abs(),
            _ => 0,
        };
        assert!(distancia(&longe[0]) > distancia(&perto[0]), "abre");
        for e in perto.iter().chain(longe.iter()) {
            let Elemento::Bloco { ret, cor } = e else {
                panic!("só blocos");
            };
            assert_eq!(ret.w % d, 0, "lado em blocos de D");
            assert_eq!(cor[3], 255, "opaco");
        }
        // Na chegada, fecha.
        let entrando = (nuvem(corpo, d, 0, true), nuvem(corpo, d, 3, true));
        assert!(distancia(&entrando.0[0]) > distancia(&entrando.1[0]));
    }
}
