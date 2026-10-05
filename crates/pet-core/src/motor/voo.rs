//! O voo da escalada (decisão 0084): na L3 (e quando o Renan volta, decisão
//! 0075), o pet sobe da casa até o alto-centro do monitor, paira com o "!!"
//! piscando e volta. Puro: o Motor mexe na célula e desenha.
//!
//! A célula anda em múltiplos de D a partir da casa (a nitidez da pixel art,
//! decisão 0004), num passo a cada [`PASSO_MS`] enquanto sobe ou desce (até
//! 30 quadros por segundo); pairando, ela fica, e só o bater das asas e o
//! "!!" mudam. O "!!" troca a cada [`PISCA_MS`]: pisca a menos de 2 Hz. Todas
//! as durações são múltiplos do passo, a partir do começo do voo: dois
//! quadros do voo nunca saem a menos de 34 ms um do outro. A casa volta no
//! fim, e nada grava posição: a posição salva do Renan não muda.

/// Um passo do voo: até 30 quadros por segundo.
pub const PASSO_MS: u64 = 34;
/// Da casa ao alto-centro (30 passos).
pub const SUBIDA_MS: u64 = 30 * PASSO_MS;
/// O "!!" aceso e apagado: troca a cada tanto (8 passos; 1,84 Hz, abaixo
/// dos 2 Hz).
pub const PISCA_MS: u64 = 8 * PASSO_MS;
/// Pairando lá em cima, com o "!!" (8 trocas).
pub const PAIRAR_MS: u64 = 8 * PISCA_MS;
/// De volta para a casa (30 passos).
pub const DESCIDA_MS: u64 = 30 * PASSO_MS;

/// Em que pé está o voo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fase {
    Subindo,
    Pairando,
    Descendo,
}

impl Fase {
    pub fn nome(self) -> &'static str {
        match self {
            Fase::Subindo => "subindo",
            Fase::Pairando => "pairando",
            Fase::Descendo => "descendo",
        }
    }
}

/// Um voo até o alto-centro e de volta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voo {
    pub inicio_ms: u64,
    /// A célula na casa (onde o pet estava e para onde volta).
    pub casa: (i32, i32),
    /// A célula lá em cima: a casa mais um múltiplo de D em cada eixo.
    pub alvo: (i32, i32),
    pub d: i32,
    /// `escalada` ou `voltou`, como a intenção.
    pub motivo: &'static str,
    /// Mandado de volta antes da hora: de onde e desde quando.
    volta: Option<((i32, i32), u64)>,
}

/// Suave no começo e no fim (smoothstep), de 0 a 1.
fn suave(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// De `de` a `ate` (células com a mesma grade de D), na fração `f`, em
/// múltiplos de D a partir de `de`.
fn entre(de: (i32, i32), ate: (i32, i32), d: i32, f: f64) -> (i32, i32) {
    let passo = |a: i32, b: i32| {
        let k = (b - a) / d;
        a + (k as f64 * f).round() as i32 * d
    };
    (passo(de.0, ate.0), passo(de.1, ate.1))
}

impl Voo {
    /// O voo de `casa` até `alvo`, a partir de `inicio_ms`. O `alvo` é
    /// trazido para a grade de D da casa (para perto dela).
    pub fn novo(
        inicio_ms: u64,
        casa: (i32, i32),
        alvo: (i32, i32),
        d: i32,
        motivo: &'static str,
    ) -> Voo {
        let d = d.max(1);
        let alvo = (
            casa.0 + (alvo.0 - casa.0) / d * d,
            casa.1 + (alvo.1 - casa.1) / d * d,
        );
        Voo {
            inicio_ms,
            casa,
            alvo,
            d,
            motivo,
            volta: None,
        }
    }

    /// Quando o voo acaba (o pet de volta na casa).
    pub fn fim_ms(&self) -> u64 {
        match self.volta {
            Some((_, desde)) => desde + DESCIDA_MS,
            None => self.inicio_ms + SUBIDA_MS + PAIRAR_MS + DESCIDA_MS,
        }
    }

    /// A fase em `agora_ms`; `None`: acabou.
    pub fn fase(&self, agora_ms: u64) -> Option<Fase> {
        if agora_ms >= self.fim_ms() {
            return None;
        }
        if self.volta.is_some() {
            return Some(Fase::Descendo);
        }
        let e = agora_ms.saturating_sub(self.inicio_ms);
        Some(if e < SUBIDA_MS {
            Fase::Subindo
        } else if e < SUBIDA_MS + PAIRAR_MS {
            Fase::Pairando
        } else {
            Fase::Descendo
        })
    }

    /// A célula em `agora_ms`, no passo de [`PASSO_MS`] (sempre a casa mais
    /// um múltiplo de D).
    pub fn posicao(&self, agora_ms: u64) -> (i32, i32) {
        if agora_ms >= self.fim_ms() {
            return self.casa;
        }
        // A fração andada no último passo de um trecho que começa em `ini` e
        // dura `dura`.
        let fracao = |ini: u64, dura: u64| {
            let passos = agora_ms.saturating_sub(ini) / PASSO_MS;
            suave((passos * PASSO_MS) as f64 / dura as f64)
        };
        if let Some((de, desde)) = self.volta {
            return entre(de, self.casa, self.d, fracao(desde, DESCIDA_MS));
        }
        match self.fase(agora_ms) {
            None => self.casa,
            Some(Fase::Subindo) => entre(
                self.casa,
                self.alvo,
                self.d,
                fracao(self.inicio_ms, SUBIDA_MS),
            ),
            Some(Fase::Pairando) => self.alvo,
            Some(Fase::Descendo) => entre(
                self.alvo,
                self.casa,
                self.d,
                fracao(self.inicio_ms + SUBIDA_MS + PAIRAR_MS, DESCIDA_MS),
            ),
        }
    }

    /// Manda de volta já (a resposta chegou, a soneca, o "não perturbe"): da
    /// célula de agora até a casa, em [`DESCIDA_MS`].
    pub fn voltar(&mut self, agora_ms: u64) {
        if self.volta.is_some() || self.fase(agora_ms).is_none() {
            return;
        }
        let de = self.posicao(agora_ms);
        self.volta = Some((de, agora_ms));
    }

    /// `t` levado para a frente, até a grade de [`PASSO_MS`] do trecho de
    /// agora (a do começo do voo, ou a da volta mandada antes da hora).
    pub fn na_grade(&self, t: u64) -> u64 {
        let base = self.volta.map_or(self.inicio_ms, |(_, desde)| desde);
        if t <= base {
            return base;
        }
        base + (t - base).div_ceil(PASSO_MS) * PASSO_MS
    }

    /// O "!!" está aceso em `agora_ms` (troca a cada [`PISCA_MS`]).
    pub fn exclamacoes_acesas(&self, agora_ms: u64) -> bool {
        (agora_ms.saturating_sub(self.inicio_ms) / PISCA_MS) % 2 == 0
    }

    /// A próxima mudança do desenho do voo: o próximo passo subindo ou
    /// descendo; pairando, a próxima troca do "!!" ou o fim de pairar; `None`
    /// depois do fim.
    pub fn proxima(&self, agora_ms: u64) -> Option<u64> {
        let fase = self.fase(agora_ms)?;
        // O próximo ponto da grade de `passo` que começa em `ini`.
        let grade = |ini: u64, passo: u64| ini + (agora_ms.saturating_sub(ini) / passo + 1) * passo;
        let proxima = match (fase, self.volta) {
            (_, Some((_, desde))) => grade(desde, PASSO_MS),
            (Fase::Pairando, None) => {
                grade(self.inicio_ms, PISCA_MS).min(self.inicio_ms + SUBIDA_MS + PAIRAR_MS)
            }
            (Fase::Subindo | Fase::Descendo, None) => grade(self.inicio_ms, PASSO_MS),
        };
        Some(proxima.min(self.fim_ms()))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn voo() -> Voo {
        // Do canto do eDP-1 (D = 4) ao alto-centro.
        Voo::novo(1_000, (1_700, 976), (868, 80), 4, "escalada")
    }

    #[test]
    fn sobe_paira_e_volta_em_multiplos_de_d_da_casa() {
        let v = voo();
        assert_eq!(v.alvo, (868, 80), "o alvo já na grade da casa");
        let torto = Voo::novo(0, (1_700, 976), (873, 83), 4, "escalada");
        assert_eq!(torto.alvo, (876, 84), "para perto da casa");
        assert_eq!(v.posicao(1_000), v.casa);
        assert_eq!(v.fase(1_000), Some(Fase::Subindo));
        let mut anterior = v.casa;
        let mut t = 1_000;
        while t < v.fim_ms() {
            let p = v.posicao(t);
            assert_eq!(
                ((p.0 - v.casa.0) % 4, (p.1 - v.casa.1) % 4),
                (0, 0),
                "{t}: {p:?}"
            );
            if t < 1_000 + SUBIDA_MS {
                assert!(p.1 <= anterior.1, "sobe sem voltar: {t}");
            }
            anterior = p;
            t += 1;
        }
        assert_eq!(v.posicao(1_000 + SUBIDA_MS), v.alvo);
        assert_eq!(v.fase(1_000 + SUBIDA_MS + 10), Some(Fase::Pairando));
        assert_eq!(v.posicao(1_000 + SUBIDA_MS + PAIRAR_MS - 1), v.alvo);
        assert_eq!(v.fase(1_000 + SUBIDA_MS + PAIRAR_MS), Some(Fase::Descendo));
        assert_eq!(v.fim_ms(), 1_000 + SUBIDA_MS + PAIRAR_MS + DESCIDA_MS);
        assert_eq!(v.fase(v.fim_ms()), None);
        assert_eq!(v.posicao(v.fim_ms()), v.casa, "de volta");
    }

    #[test]
    fn os_passos_vao_ate_30_por_segundo_e_o_pisca_e_de_2_hz() {
        let v = voo();
        let mut t = v.inicio_ms;
        let mut horas = vec![t];
        while let Some(p) = v.proxima(t) {
            assert!(p > t);
            horas.push(p);
            t = p;
        }
        assert_eq!(t, v.fim_ms());
        assert!(
            horas.windows(2).all(|j| j[1] - j[0] >= PASSO_MS),
            "{horas:?}"
        );
        // Pairando, só o "!!" muda.
        let pairando: Vec<u64> = horas
            .iter()
            .copied()
            .filter(|&h| v.fase(h) == Some(Fase::Pairando))
            .collect();
        assert!(
            pairando.windows(2).all(|j| j[1] - j[0] <= PISCA_MS),
            "{pairando:?}"
        );
        assert!(v.exclamacoes_acesas(1_000) && !v.exclamacoes_acesas(1_000 + PISCA_MS));
        assert!(v.exclamacoes_acesas(1_000 + 2 * PISCA_MS));
        // No máximo 2 Hz: um ciclo aceso e apagado leva mais de meio segundo.
        const { assert!(2 * PISCA_MS >= 500) };
    }

    #[test]
    fn mandado_de_volta_desce_de_onde_esta() {
        let mut v = voo();
        let meio = 1_000 + SUBIDA_MS / 2;
        let la = v.posicao(meio);
        assert_ne!(la, v.casa);
        v.voltar(meio);
        assert_eq!(v.fase(meio), Some(Fase::Descendo));
        assert_eq!(v.posicao(meio), la, "começa onde estava");
        assert_eq!(v.fim_ms(), meio + DESCIDA_MS);
        assert_eq!(v.posicao(meio + DESCIDA_MS), v.casa);
        let p = v.posicao(meio + DESCIDA_MS / 2);
        assert_eq!(((p.0 - v.casa.0) % 4, (p.1 - v.casa.1) % 4), (0, 0));
        // Voltar de novo não muda nada; depois do fim, também não.
        v.voltar(meio + 10);
        assert_eq!(v.fim_ms(), meio + DESCIDA_MS);
        // A grade da volta é a dela.
        assert_eq!(v.na_grade(meio + 1), meio + PASSO_MS);
        assert_eq!(v.na_grade(meio), meio);
        assert_eq!(voo().na_grade(1_001), 1_034);
        assert_eq!(voo().na_grade(1_034), 1_034);
        assert_eq!(voo().na_grade(5), 1_000);
    }
}
