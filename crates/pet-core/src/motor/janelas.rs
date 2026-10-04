//! O anel de ativações (M4, decisões 0039, 0043 e 0050): as últimas trocas
//! de janela ativa, só o endereço (um id opaco) e a hora de parede em que o
//! desktop contou, nunca o título.
//!
//! É por ele que o Motor descobre qual janela é o terminal de uma sessão do
//! Claude: a janela que estava ativa na hora (`ts`) em que o Renan mandou o
//! prompt ([`Anel::em`]). Quando a troca foi perto demais dessa hora, há
//! dúvida, e o clique cai no balão com a lista.
//!
//! **Buracos.** Quando a fonte das trocas cai (o socket2 do Hyprland), o que
//! aconteceu até ela voltar é desconhecido: o anel marca um buraco, e uma
//! hora dentro dele não acha janela. A semente (a janela que o
//! foreign-toplevel diz estar ativa) só entra com o anel vazio ou num
//! buraco: ela diz que a janela estava ativa, não desde quando.

use std::collections::VecDeque;

use serde::Serialize;

use crate::plataforma::Alca;

/// Trocas guardadas (só trocas de verdade: o mesmo endereço repetido não
/// conta).
pub const CAPACIDADE: usize = 128;
/// Uma troca de janela a menos disto antes da hora procurada deixa dúvida:
/// o prompt pode ter sido mandado da janela de antes.
pub const DUVIDA_MS: u64 = 1_000;
/// Trocas mostradas no `/v1/estado`.
const NO_PAINEL: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tipo {
    /// Uma troca contada pela fonte dos eventos.
    Troca,
    /// A janela que já estava ativa (o foreign-toplevel): sem a hora da
    /// troca.
    Semente,
    /// A fonte caiu: daqui até a próxima troca, nada se sabe.
    Buraco,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Troca {
    pub janela: Option<Alca>,
    pub parede_ms: u64,
    pub tipo: Tipo,
}

/// O que o anel acha numa hora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Achado {
    /// A janela ativa naquela hora, sem troca perto.
    Janela(Alca),
    /// A janela trocou menos de [`DUVIDA_MS`] antes: pode ser a de antes.
    Duvida,
    /// Nenhuma janela estava ativa (uma área de trabalho vazia).
    Nenhuma,
    /// O anel não cobre a hora (vazio, antes da troca mais velha, ou num
    /// buraco).
    Desconhecida,
}

/// Uma troca no `/v1/estado`: o endereço e a hora, nada mais.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PainelTroca {
    pub janela: Option<String>,
    /// Hora de parede da troca (ms desde 1970).
    pub em_ms: u64,
    pub tipo: Tipo,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Anel {
    trocas: VecDeque<Troca>,
}

impl Anel {
    fn guardar(&mut self, troca: Troca) {
        if self.trocas.len() == CAPACIDADE {
            self.trocas.pop_front();
        }
        self.trocas.push_back(troca);
    }

    /// A fonte contou que `janela` ficou ativa em `parede_ms`. `false` se não
    /// era troca (a mesma janela de antes).
    pub fn ativou(&mut self, janela: Option<Alca>, parede_ms: u64) -> bool {
        let ultima = self.trocas.back();
        if ultima.is_some_and(|t| t.tipo != Tipo::Buraco && t.janela == janela) {
            return false;
        }
        // Um relógio que voltou (ajuste de hora) não bagunça a ordem.
        let parede_ms = parede_ms.max(ultima.map_or(0, |t| t.parede_ms));
        self.guardar(Troca {
            janela,
            parede_ms,
            tipo: Tipo::Troca,
        });
        true
    }

    /// A janela que já estava ativa (o foreign-toplevel, na conexão nova):
    /// só entra com o anel vazio ou num buraco.
    pub fn semente(&mut self, janela: Alca, parede_ms: u64) -> bool {
        match self.trocas.back() {
            None => {}
            Some(t) if t.tipo == Tipo::Buraco => {}
            Some(_) => return false,
        }
        let parede_ms = parede_ms.max(self.trocas.back().map_or(0, |t| t.parede_ms));
        self.guardar(Troca {
            janela: Some(janela),
            parede_ms,
            tipo: Tipo::Semente,
        });
        true
    }

    /// A fonte caiu em `parede_ms`.
    pub fn buraco(&mut self, parede_ms: u64) {
        if self.trocas.back().is_some_and(|t| t.tipo == Tipo::Buraco) {
            return;
        }
        let parede_ms = parede_ms.max(self.trocas.back().map_or(0, |t| t.parede_ms));
        self.guardar(Troca {
            janela: None,
            parede_ms,
            tipo: Tipo::Buraco,
        });
    }

    /// A janela ativa agora, pelo anel (`None`: nenhuma, ou não se sabe).
    pub fn ativa(&self) -> Option<&Alca> {
        self.trocas.back().and_then(|t| t.janela.as_ref())
    }

    /// Quando a janela ativa de agora ficou ativa (para o "pronto visto"
    /// depois de uns segundos com o terminal em foco). `None` se não há
    /// janela ativa conhecida.
    pub fn ativa_desde(&self) -> Option<(&Alca, u64)> {
        let t = self.trocas.back()?;
        Some((t.janela.as_ref()?, t.parede_ms))
    }

    /// A janela ativa na hora `ts` (ms desde 1970).
    pub fn em(&self, ts: u64) -> Achado {
        let Some(indice) = self.trocas.iter().rposition(|t| t.parede_ms <= ts) else {
            return Achado::Desconhecida;
        };
        let troca = &self.trocas[indice];
        match (&troca.tipo, &troca.janela) {
            (Tipo::Buraco, _) => Achado::Desconhecida,
            (_, None) => Achado::Nenhuma,
            (Tipo::Semente, Some(janela)) => Achado::Janela(janela.clone()),
            (Tipo::Troca, Some(janela)) => {
                // A primeira troca do anel (ou a primeira depois de um
                // buraco) não diz o que havia antes: a dúvida só vale para
                // uma troca de verdade entre duas janelas conhecidas.
                let houve_antes = indice > 0 && self.trocas[indice - 1].tipo != Tipo::Buraco;
                if houve_antes && ts.saturating_sub(troca.parede_ms) < DUVIDA_MS {
                    Achado::Duvida
                } else {
                    Achado::Janela(janela.clone())
                }
            }
        }
    }

    /// As últimas trocas, da mais nova para a mais velha, para o
    /// `/v1/estado`.
    pub fn painel(&self) -> Vec<PainelTroca> {
        self.trocas
            .iter()
            .rev()
            .take(NO_PAINEL)
            .map(|t| PainelTroca {
                janela: t.janela.as_ref().map(|a| a.0.clone()),
                em_ms: t.parede_ms,
                tipo: t.tipo,
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const T0: u64 = 1_790_000_000_000;

    fn a(s: &str) -> Alca {
        Alca(s.into())
    }

    #[test]
    fn vazio_nao_sabe_e_a_semente_preenche() {
        let mut anel = Anel::default();
        assert_eq!(anel.em(T0), Achado::Desconhecida);
        assert!(anel.semente(a("foot1"), T0));
        assert_eq!(
            anel.em(T0 + 10),
            Achado::Janela(a("foot1")),
            "semente não tem dúvida"
        );
        assert_eq!(anel.em(T0 - 1), Achado::Desconhecida, "antes da semente");
        assert!(
            !anel.semente(a("foot2"), T0 + 5),
            "com trocas, a semente não entra"
        );
    }

    #[test]
    fn a_janela_da_hora_com_duvida_perto_da_troca() {
        let mut anel = Anel::default();
        assert!(anel.ativou(Some(a("foot1")), T0));
        assert!(
            !anel.ativou(Some(a("foot1")), T0 + 500),
            "repetida não é troca"
        );
        assert!(anel.ativou(Some(a("foot2")), T0 + 10_000));
        assert!(anel.ativou(None, T0 + 20_000));
        assert!(anel.ativou(Some(a("foot1")), T0 + 30_000));
        // A primeira troca do anel não tem "antes": sem dúvida.
        assert_eq!(anel.em(T0 + 100), Achado::Janela(a("foot1")));
        assert_eq!(anel.em(T0 + 9_999), Achado::Janela(a("foot1")));
        // Logo depois de trocar para foot2: dúvida (o prompt pode ser do 1).
        assert_eq!(anel.em(T0 + 10_000), Achado::Duvida);
        assert_eq!(anel.em(T0 + 10_999), Achado::Duvida);
        assert_eq!(anel.em(T0 + 11_000), Achado::Janela(a("foot2")));
        assert_eq!(anel.em(T0 + 25_000), Achado::Nenhuma, "área vazia");
        assert_eq!(anel.em(T0 + 40_000), Achado::Janela(a("foot1")));
        assert_eq!(anel.ativa(), Some(&a("foot1")));
        assert_eq!(anel.ativa_desde(), Some((&a("foot1"), T0 + 30_000)));
    }

    #[test]
    fn buraco_nao_sabe_ate_a_proxima_troca() {
        let mut anel = Anel::default();
        anel.ativou(Some(a("foot1")), T0);
        anel.buraco(T0 + 5_000);
        anel.buraco(T0 + 6_000);
        assert_eq!(anel.em(T0 + 4_000), Achado::Janela(a("foot1")));
        assert_eq!(anel.em(T0 + 7_000), Achado::Desconhecida);
        assert_eq!(anel.ativa(), None, "não se sabe a ativa");
        // A semente preenche o buraco; a primeira troca depois dele não tem
        // dúvida (não se sabe o que havia antes).
        assert!(anel.semente(a("foot2"), T0 + 8_000));
        assert_eq!(anel.em(T0 + 8_100), Achado::Janela(a("foot2")));
        let mut outro = Anel::default();
        outro.ativou(Some(a("foot1")), T0);
        outro.buraco(T0 + 1_000);
        outro.ativou(Some(a("foot2")), T0 + 2_000);
        assert_eq!(outro.em(T0 + 2_100), Achado::Janela(a("foot2")));
        // A mesma janela de antes do buraco conta como troca.
        let mut terceiro = Anel::default();
        terceiro.ativou(Some(a("foot1")), T0);
        terceiro.buraco(T0 + 1_000);
        assert!(terceiro.ativou(Some(a("foot1")), T0 + 2_000));
    }

    #[test]
    fn capacidade_e_relogio_que_volta() {
        let mut anel = Anel::default();
        for i in 0..CAPACIDADE as u64 + 10 {
            anel.ativou(Some(a(&format!("{i:x}"))), T0 + i * 10_000);
        }
        assert_eq!(anel.trocas.len(), CAPACIDADE);
        assert_eq!(anel.em(T0), Achado::Desconhecida, "a mais velha saiu");
        assert_eq!(anel.painel().len(), NO_PAINEL);
        let ultima = &anel.painel()[0];
        assert_eq!(ultima.tipo, Tipo::Troca);
        assert_eq!(ultima.janela.as_deref(), Some("89"), "137 em hexadecimal");
        // Uma hora que voltou fica na hora da troca anterior.
        let mut volta = Anel::default();
        volta.ativou(Some(a("x")), T0 + 5_000);
        volta.ativou(Some(a("y")), T0);
        assert_eq!(volta.trocas[1].parede_ms, T0 + 5_000);
    }
}
