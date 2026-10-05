//! O registro de intenções do Motor (decisão 0077): cada decisão do M5 vira
//! uma linha normalizada, com a hora no relógio do laço.
//!
//! É o contrato com quem desenha (a segunda metade do M5 e o M6) e o que os
//! cenários dourados comparam (`pet_core::cenario`). As reações continuam
//! indo ao animador pelo caminho do M3; o resto (a base segurada, os selos,
//! os voos, o confete, os balões da festa) espera quem desenha, e está aqui
//! para os testes e o `/v1/estado` lerem.
//!
//! Só metadados: o id curto da sessão (`sid8`), o do turno, o nome da pasta
//! do projeto nos balões (como o balão do M4 já mostra) e enums. Nada de
//! título de janela, prompt ou caminho.

use std::collections::VecDeque;

use serde::Serialize;

use crate::cerebro::{Fim, Nivel, OrigemTurno, RegistroTurno, ResumoCorrente};

/// Intenções guardadas para o `/v1/estado` (as mais novas).
pub const GUARDADAS: usize = 200;
/// Intenções mostradas no `/v1/estado.intencoes`.
pub const NO_PAINEL: usize = 50;

/// Uma decisão do Motor, na hora em que foi tomada (ms no relógio do laço).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Intencao {
    #[serde(rename = "t")]
    pub t_ms: u64,
    #[serde(flatten)]
    pub tipo: Tipo,
}

/// O que foi decidido.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "i", rename_all = "snake_case")]
pub enum Tipo {
    /// Um turno fechou (o registro do `/v1/estado.turnos`, resumido).
    Turno {
        sid8: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        turno8: Option<String>,
        fim: Fim,
        #[serde(skip_serializing_if = "Option::is_none")]
        nivel: Option<Nivel>,
        /// A pontuação do trabalho (decisão 0074), num fim por Stop.
        #[serde(skip_serializing_if = "Option::is_none")]
        pontuacao: Option<f64>,
        /// O que mexeu no nível (`maquina`, `modo`, `intervalo_t3`).
        #[serde(skip_serializing_if = "Vec::is_empty")]
        teto: Vec<&'static str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reacao: Option<&'static str>,
        /// De onde veio o prompt, se não foi digitado (decisão 0073).
        #[serde(skip_serializing_if = "Option::is_none")]
        origem: Option<OrigemTurno>,
        /// A corrente de agentes do turno, se há uma.
        #[serde(skip_serializing_if = "Option::is_none")]
        corrente: Option<ResumoCorrente>,
        #[serde(skip_serializing_if = "eh_falso")]
        teste: bool,
    },
    /// Uma reação para o animador tocar uma vez (`nod`, `done_small`, `bye`,
    /// `giggle`, `yawn`, `wake`, …).
    Reacao {
        nome: String,
        motivo: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        sid8: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        nivel: Option<Nivel>,
    },
    /// Um balão com estas linhas, por este motivo.
    Balao {
        linhas: Vec<String>,
        motivo: &'static str,
    },
    /// O que um clique no pet fez (`focou`, `nao_focou`, `lista`, `soneca`,
    /// `nada`).
    Clique {
        resultado: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        sid8: Option<String>,
    },
}

fn eh_falso(b: &bool) -> bool {
    !*b
}

impl Tipo {
    /// O resumo de um turno fechado.
    pub fn do_turno(r: &RegistroTurno) -> Tipo {
        Tipo::Turno {
            sid8: r.sid8.clone(),
            turno8: r.turno8.clone(),
            fim: r.fim,
            nivel: r.nivel,
            pontuacao: r.pontuacao.map(|p| p.total),
            teto: r.teto.clone(),
            reacao: r.reacao,
            origem: r.origem.maquina().then_some(r.origem),
            corrente: r.corrente,
            teste: r.teste,
        }
    }
}

/// Uma intenção no `/v1/estado`: a idade em vez da hora do laço.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NoPainel {
    pub ha_ms: u64,
    #[serde(flatten)]
    pub tipo: Tipo,
}

/// As intenções: as últimas [`GUARDADAS`] e, quando alguém pede (o executor
/// dos cenários), todas as novas desde a última vez.
#[derive(Debug, Clone, Default)]
pub struct Registro {
    ultimas: VecDeque<Intencao>,
    /// Só com [`Registro::gravar_tudo`]: o daemon nunca as tira, e elas não
    /// podem crescer sem fim.
    novas: Option<Vec<Intencao>>,
}

impl Registro {
    /// Guarda também cada intenção nova até [`Registro::tirar_novas`] (o
    /// executor dos cenários).
    pub fn gravar_tudo(&mut self) {
        self.novas.get_or_insert_with(Vec::new);
    }

    pub fn anotar(&mut self, t_ms: u64, tipo: Tipo) {
        let intencao = Intencao { t_ms, tipo };
        if let Some(novas) = self.novas.as_mut() {
            novas.push(intencao.clone());
        }
        if self.ultimas.len() == GUARDADAS {
            self.ultimas.pop_front();
        }
        self.ultimas.push_back(intencao);
    }

    /// As intenções desde a última vez (vazio sem [`Registro::gravar_tudo`]).
    pub fn tirar_novas(&mut self) -> Vec<Intencao> {
        self.novas.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// As últimas [`NO_PAINEL`], da mais nova para a mais velha, com a idade.
    pub fn painel(&self, agora_ms: u64) -> Vec<NoPainel> {
        self.ultimas
            .iter()
            .rev()
            .take(NO_PAINEL)
            .map(|i| NoPainel {
                ha_ms: agora_ms.saturating_sub(i.t_ms),
                tipo: i.tipo.clone(),
            })
            .collect()
    }

    /// Todas as guardadas, da mais velha para a mais nova.
    pub fn ultimas(&self) -> impl Iterator<Item = &Intencao> {
        self.ultimas.iter()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn reacao(nome: &str) -> Tipo {
        Tipo::Reacao {
            nome: nome.into(),
            motivo: "fim",
            sid8: Some("aaaaaaaa".into()),
            nivel: Some(Nivel::T0),
        }
    }

    #[test]
    fn linha_normalizada_com_a_hora_e_o_tipo() {
        let i = Intencao {
            t_ms: 30_800,
            tipo: reacao("nod"),
        };
        assert_eq!(
            serde_json::to_string(&i).unwrap(),
            r#"{"t":30800,"i":"reacao","nome":"nod","motivo":"fim","sid8":"aaaaaaaa","nivel":"T0"}"#
        );
        let c = Intencao {
            t_ms: 5,
            tipo: Tipo::Clique {
                resultado: "lista",
                sid8: None,
            },
        };
        assert_eq!(
            serde_json::to_string(&c).unwrap(),
            r#"{"t":5,"i":"clique","resultado":"lista"}"#
        );
    }

    #[test]
    fn guarda_as_ultimas_e_so_grava_tudo_quando_pedem() {
        let mut r = Registro::default();
        for t in 0..(GUARDADAS as u64 + 10) {
            r.anotar(t, reacao("nod"));
        }
        assert!(r.tirar_novas().is_empty(), "o daemon nunca acumula");
        assert_eq!(r.ultimas().count(), GUARDADAS);
        assert_eq!(r.ultimas().next().unwrap().t_ms, 10);
        let painel = r.painel(GUARDADAS as u64 + 20);
        assert_eq!(painel.len(), NO_PAINEL);
        assert_eq!(painel[0].ha_ms, 11, "a mais nova primeiro");
        r.gravar_tudo();
        r.anotar(500, reacao("bye"));
        r.anotar(501, reacao("nod"));
        let novas = r.tirar_novas();
        assert_eq!(
            novas.iter().map(|i| i.t_ms).collect::<Vec<_>>(),
            vec![500, 501]
        );
        assert!(r.tirar_novas().is_empty());
    }
}
