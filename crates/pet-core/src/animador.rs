//! Animador mínimo (M1): toca tags com as durações por quadro do Aseprite e
//! mantém o pet parado de um jeito barato.
//!
//! Cada troca de quadro vira um commit Wayland, e cada commit repinta o
//! monitor inteiro no Hyprland 0.56 (decisão 0005). Por isso o repouso não é
//! a tag `idle` em laço: é uma **pose fixa** (o primeiro quadro da primeira
//! tag do estado `idle`) e, a cada [`PAUSA_MS`], uma **rajada** que toca uma
//! das tags do estado uma vez, alternando entre elas. Com as tags da skin de
//! teste isso dá menos de 1 commit/s em média (orçamento: até 2/s).
//!
//! Tudo aqui recebe o relógio de fora (milissegundos), para os testes não
//! dependerem de tempo real.

use crate::skin::{Direcao, Skin, Tag};

/// Pausa entre rajadas no repouso.
pub const PAUSA_MS: u64 = 4000;

/// Quadros de uma tag na ordem em que tocam num ciclo (sem repetir as pontas
/// no ping-pong, como o Aseprite).
pub fn sequencia(tag: &Tag) -> Vec<usize> {
    let ida: Vec<usize> = (tag.de..=tag.ate).collect();
    let miolo = if ida.len() > 2 {
        &ida[1..ida.len() - 1]
    } else {
        &[][..]
    };
    match tag.direcao {
        Direcao::Frente => ida.clone(),
        Direcao::Tras => ida.iter().rev().copied().collect(),
        Direcao::PingPong => ida.iter().chain(miolo.iter().rev()).copied().collect(),
        Direcao::PingPongReverso => ida.iter().rev().chain(miolo.iter()).copied().collect(),
    }
}

/// Uma tag tocando a partir de um instante, uma vez ou em laço.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animacao {
    /// (quadro da folha, duração em ms).
    passos: Vec<(usize, u64)>,
    inicio_ms: u64,
    laco: bool,
    total_ms: u64,
}

impl Animacao {
    pub fn nova(skin: &Skin, tag: usize, inicio_ms: u64, laco: bool) -> Animacao {
        let passos: Vec<(usize, u64)> = sequencia(&skin.tags[tag])
            .into_iter()
            .map(|q| (q, skin.quadros[q].duracao_ms.max(1) as u64))
            .collect();
        let total_ms = passos.iter().map(|(_, d)| d).sum();
        Animacao {
            passos,
            inicio_ms,
            laco,
            total_ms,
        }
    }

    pub fn duracao_ms(&self) -> u64 {
        self.total_ms
    }

    /// Quadro em `agora_ms` e o instante da próxima troca (`None`: não troca
    /// mais — acabou, ou a tag tem um quadro só).
    pub fn em(&self, agora_ms: u64) -> (usize, Option<u64>) {
        let decorrido = agora_ms.saturating_sub(self.inicio_ms);
        let ultimo = self.passos.last().map(|p| p.0).unwrap_or(0);
        if self.passos.iter().all(|p| p.0 == ultimo) {
            return (ultimo, None);
        }
        if !self.laco && decorrido >= self.total_ms {
            return (ultimo, None);
        }
        let ciclo = decorrido / self.total_ms;
        let mut t = decorrido % self.total_ms;
        let base = self.inicio_ms + ciclo * self.total_ms;
        let mut fim = base;
        for &(quadro, duracao) in &self.passos {
            fim += duracao;
            if t < duracao {
                return (quadro, Some(fim));
            }
            t -= duracao;
        }
        (ultimo, None)
    }
}

/// O pet parado: pose fixa e rajadas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repouso {
    pose: usize,
    /// Rajadas (cada uma começa no instante 0; o repouso desloca).
    rajadas: Vec<Animacao>,
    inicio_ms: u64,
    periodo_ms: u64,
}

impl Repouso {
    /// Repouso do estado `idle` da skin (a skin garante que ele existe).
    pub fn novo(skin: &Skin, inicio_ms: u64) -> Repouso {
        let tags = skin.tags_do_estado("idle");
        let pose = tags
            .first()
            .map(|&t| sequencia(&skin.tags[t])[0])
            .unwrap_or(0);
        let rajadas: Vec<Animacao> = tags
            .iter()
            .map(|&t| Animacao::nova(skin, t, 0, false))
            .collect();
        let periodo_ms = rajadas
            .iter()
            .map(|r| PAUSA_MS + r.duracao_ms())
            .sum::<u64>();
        Repouso {
            pose,
            rajadas,
            inicio_ms,
            periodo_ms: periodo_ms.max(PAUSA_MS),
        }
    }

    pub fn pose(&self) -> usize {
        self.pose
    }

    /// Quadro em `agora_ms` e o instante da próxima troca.
    pub fn em(&self, agora_ms: u64) -> (usize, u64) {
        let decorrido = agora_ms.saturating_sub(self.inicio_ms);
        let base = self.inicio_ms + decorrido / self.periodo_ms * self.periodo_ms;
        let mut t = decorrido % self.periodo_ms;
        let mut inicio_segmento = base;
        for rajada in &self.rajadas {
            if t < PAUSA_MS {
                return (self.pose, inicio_segmento + PAUSA_MS);
            }
            t -= PAUSA_MS;
            inicio_segmento += PAUSA_MS;
            if t < rajada.duracao_ms() {
                let (quadro, proxima) = rajada.em(t);
                let fim = inicio_segmento + rajada.duracao_ms();
                let proxima = proxima.map_or(fim, |p| inicio_segmento + p);
                return (quadro, proxima.min(fim));
            }
            t -= rajada.duracao_ms();
            inicio_segmento += rajada.duracao_ms();
        }
        (self.pose, base + self.periodo_ms)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::skin::testes::skin_minima;

    fn tag(de: usize, ate: usize, direcao: Direcao) -> Tag {
        Tag {
            nome: "t".into(),
            de,
            ate,
            direcao,
        }
    }

    #[test]
    fn sequencias_das_quatro_direcoes() {
        assert_eq!(sequencia(&tag(2, 5, Direcao::Frente)), vec![2, 3, 4, 5]);
        assert_eq!(sequencia(&tag(2, 5, Direcao::Tras)), vec![5, 4, 3, 2]);
        assert_eq!(
            sequencia(&tag(2, 5, Direcao::PingPong)),
            vec![2, 3, 4, 5, 4, 3]
        );
        assert_eq!(
            sequencia(&tag(2, 5, Direcao::PingPongReverso)),
            vec![5, 4, 3, 2, 3, 4]
        );
        assert_eq!(sequencia(&tag(0, 1, Direcao::PingPong)), vec![0, 1]);
        assert_eq!(sequencia(&tag(3, 3, Direcao::PingPong)), vec![3]);
    }

    #[test]
    fn animacao_em_laco_e_uma_vez() {
        let skin = skin_minima(); // idle: quadros 0 (100 ms) e 1 (300 ms)
        let laco = Animacao::nova(&skin, 0, 1000, true);
        assert_eq!(laco.duracao_ms(), 400);
        assert_eq!(laco.em(1000), (0, Some(1100)));
        assert_eq!(laco.em(1099), (0, Some(1100)));
        assert_eq!(laco.em(1100), (1, Some(1400)));
        assert_eq!(laco.em(1400), (0, Some(1500)), "volta ao começo");
        assert_eq!(laco.em(500), (0, Some(1100)), "antes do início");
        let uma = Animacao::nova(&skin, 0, 0, false);
        assert_eq!(uma.em(150), (1, Some(400)));
        assert_eq!(uma.em(400), (1, None));
        // Ping-pong de 2 quadros: 0,1,0,1…
        let pp = Animacao::nova(&skin, 1, 0, true);
        assert_eq!(pp.em(100), (1, Some(400)));
        assert_eq!(pp.em(400), (0, Some(500)));
    }

    #[test]
    fn repouso_pose_e_rajada() {
        let skin = skin_minima();
        let r = Repouso::novo(&skin, 0);
        assert_eq!(r.pose(), 0);
        assert_eq!(r.em(0), (0, PAUSA_MS));
        assert_eq!(r.em(PAUSA_MS - 1), (0, PAUSA_MS));
        assert_eq!(r.em(PAUSA_MS), (0, PAUSA_MS + 100));
        assert_eq!(r.em(PAUSA_MS + 100), (1, PAUSA_MS + 400));
        assert_eq!(
            r.em(PAUSA_MS + 400),
            (0, 2 * PAUSA_MS + 400),
            "volta à pose"
        );
    }

    /// Simula seguindo os prazos e conta trocas de quadro (= commits).
    fn commits_por_segundo(r: &Repouso, segundos: u64) -> f64 {
        let fim = segundos * 1000;
        let (mut atual, mut t) = r.em(0);
        let mut commits = 1; // o primeiro quadro
        while t < fim {
            let (q, proxima) = r.em(t);
            if q != atual {
                commits += 1;
                atual = q;
            }
            assert!(proxima > t, "prazo não avança em {t}");
            t = proxima;
        }
        commits as f64 / segundos as f64
    }

    #[test]
    fn repouso_cabe_no_orcamento_de_commits() {
        let skin = skin_minima();
        let media = commits_por_segundo(&Repouso::novo(&skin, 0), 600);
        assert!(media <= 2.0, "{media} commits/s");
        assert!(media > 0.0);
    }
}
