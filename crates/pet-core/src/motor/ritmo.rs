//! Ritmo da tela: os commits contados (orçamento da decisão 0005) e o teste
//! de estresse (`/v1/debug/estresse`), os dois no relógio inteiro em
//! milissegundos do Motor.

use std::collections::VecDeque;

use crate::confete::Chuva;

/// Janela da contagem de commits.
pub const JANELA_COMMITS_MS: u64 = 60_000;

/// Commits na tela: o total e os do último minuto (orçamento da decisão
/// 0005: média de até 2/s parado, 0 dormindo, rajadas de até 30 fps).
#[derive(Debug, Default, Clone)]
pub struct Commits {
    recentes: VecDeque<u64>,
    pub total: u64,
}

impl Commits {
    pub fn contar(&mut self, agora_ms: u64) {
        self.total += 1;
        self.recentes.push_back(agora_ms);
        self.podar(agora_ms);
    }

    pub fn por_minuto(&mut self, agora_ms: u64) -> usize {
        self.podar(agora_ms);
        self.recentes.len()
    }

    fn podar(&mut self, agora_ms: u64) {
        while let Some(&t) = self.recentes.front() {
            if agora_ms.saturating_sub(t) <= JANELA_COMMITS_MS {
                break;
            }
            self.recentes.pop_front();
        }
    }
}

/// Teste de estresse em curso. Os prazos ficam no mesmo relógio inteiro do
/// animador: o laço acorda no prazo ou depois, e o passo devido sempre
/// acontece (sem laço ocupado por um prazo arredondado para baixo).
#[derive(Debug, Clone)]
pub struct Estresse {
    pub chuva: Chuva,
    pub fps: u64,
    pub inicio_ms: u64,
    /// Passos dados (o passo `n` vence em `inicio + ⌈n·1000/fps⌉`).
    pub passos: u64,
    pub fim_ms: u64,
}

impl Estresse {
    pub fn prazo_do_passo(&self, n: u64) -> u64 {
        self.inicio_ms + (n * 1000).div_ceil(self.fps)
    }

    /// Dá um passo se algum venceu. Passos atrasados não viram rajada: o
    /// confete só pula para o passo de agora.
    pub fn avancar(&mut self, agora_ms: u64) {
        let devidos = agora_ms.saturating_sub(self.inicio_ms) * self.fps / 1000;
        if devidos > self.passos {
            self.chuva.passo();
            self.passos = devidos;
        }
    }

    pub fn proximo_prazo(&self) -> u64 {
        self.prazo_do_passo(self.passos + 1).min(self.fim_ms)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::confete::Grade;

    #[test]
    fn commits_por_minuto_esquece_o_que_passou_de_60_s() {
        let mut c = Commits::default();
        for i in 0..5 {
            c.contar(i * 10_000);
        }
        assert_eq!(c.por_minuto(40_000), 5);
        assert_eq!(c.por_minuto(75_000), 3);
        assert_eq!(c.por_minuto(500_000), 0);
        assert_eq!(c.total, 5);
    }

    #[test]
    fn estresse_anda_um_passo_por_vez_e_acaba_no_prazo() {
        let grade = Grade { x: 0, y: 0, d: 5 };
        let mut e = Estresse {
            chuva: Chuva::nova(4, (1920, 1200), grade, 3, 7),
            fps: 30,
            inicio_ms: 1_000,
            passos: 0,
            fim_ms: 4_000,
        };
        assert_eq!(e.proximo_prazo(), 1_034, "⌈1000/30⌉ = 34");
        e.avancar(1_020);
        assert_eq!(e.passos, 0, "nenhum passo venceu");
        e.avancar(1_500);
        assert_eq!(e.passos, 15, "atrasado pula para o passo de agora");
        assert_eq!(e.proximo_prazo(), 1_534);
        e.avancar(3_999);
        assert_eq!(e.proximo_prazo(), 4_000, "nunca passa do fim");
    }
}
