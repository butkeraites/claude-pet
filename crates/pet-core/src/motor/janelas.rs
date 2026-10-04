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
//! foreign-toplevel diz estar ativa) só entra com o anel vazio, num buraco
//! ou depois de outra semente — enquanto a fonte das trocas não contou nada
//! desde o buraco. A primeira semente diz que a janela estava ativa, não
//! desde quando; as seguintes são trocas que o foreign-toplevel viu.

use std::collections::{BTreeMap, VecDeque};

use serde::Serialize;

use crate::evento::Terminal;
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

    /// A janela ativa pelo foreign-toplevel: só entra com o anel vazio, num
    /// buraco ou depois de outra semente (a fonte das trocas não contou nada
    /// desde então).
    pub fn semente(&mut self, janela: Alca, parede_ms: u64) -> bool {
        match self.trocas.back() {
            None => {}
            Some(t) if t.tipo == Tipo::Buraco => {}
            Some(t) if t.tipo == Tipo::Semente => {
                if t.janela.as_ref() == Some(&janela) {
                    return false;
                }
            }
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
            (_, Some(janela)) => {
                // A primeira troca do anel (ou a primeira depois de um
                // buraco, e a primeira semente) não diz o que havia antes: a
                // dúvida só vale para uma troca entre duas janelas
                // conhecidas.
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

/// Uma sessão do Claude: (é teste, `session_id`), como o cérebro.
pub type Chave = (bool, String);

/// O quanto se sabe da janela de uma sessão (decisão 0055).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Certeza {
    /// A janela ativa na hora do prompt, sem troca perto.
    Certa,
    /// A janela trocou menos de 1 s antes do prompt.
    Duvida,
    /// O anel não cobre a hora (o pet acabou de subir, ou a fonte caiu).
    SemAnel,
    /// Nenhuma janela estava ativa na hora.
    SemJanela,
    /// A janela da sessão fechou.
    Fechou,
}

impl Certeza {
    /// Por que não dá para focar, em palavras (o balão).
    pub fn motivo(self) -> &'static str {
        match self {
            Certeza::Certa => "",
            Certeza::Duvida => "trocou de janela perto do prompt",
            Certeza::SemAnel => "não vi a janela dela",
            Certeza::SemJanela => "nenhuma janela estava ativa",
            Certeza::Fechou => "a janela dela fechou",
        }
    }
}

/// A janela de uma sessão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identidade {
    /// A janela (o terminal), quando se sabe; uma vez certa, fica até outra
    /// certa trocá-la ou ela fechar.
    pub janela: Option<Alca>,
    pub certeza: Certeza,
    /// Os ids de terminal do hook (só separam sessões dentro de um mesmo
    /// terminal).
    pub terminal: Option<Terminal>,
    /// Hora (parede) do último prompt casado.
    pub em_ms: u64,
}

/// A identidade de uma sessão no `/v1/estado.sessoes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResumoJanela {
    /// O endereço da janela (o do `activewindowv2`), se se sabe.
    pub endereco: Option<String>,
    pub certeza: Certeza,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<Terminal>,
}

/// As janelas das sessões.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identidades {
    mapa: BTreeMap<Chave, Identidade>,
}

impl Identidades {
    /// Um `SessionStart` ou `UserPromptSubmit` da sessão `chave`, na hora
    /// `ts` (o `ts` do hook), com o que o anel achou nessa hora. Uma janela
    /// certa troca a de antes; sem certeza, a de antes fica (o terminal de
    /// uma sessão não muda), e só sem nenhuma a dúvida é guardada.
    pub fn observar(&mut self, chave: Chave, achado: Achado, terminal: Option<Terminal>, ts: u64) {
        let atual = self.mapa.entry(chave).or_insert(Identidade {
            janela: None,
            certeza: Certeza::SemAnel,
            terminal: None,
            em_ms: ts,
        });
        atual.em_ms = ts;
        if terminal.is_some() {
            atual.terminal = terminal;
        }
        match achado {
            Achado::Janela(janela) => {
                atual.janela = Some(janela);
                atual.certeza = Certeza::Certa;
            }
            _ if atual.janela.is_some() => {}
            Achado::Duvida => atual.certeza = Certeza::Duvida,
            Achado::Nenhuma => atual.certeza = Certeza::SemJanela,
            Achado::Desconhecida => atual.certeza = Certeza::SemAnel,
        }
    }

    /// A janela fechou: as sessões nela ficam sem janela.
    pub fn fechou(&mut self, janela: &Alca) {
        for identidade in self.mapa.values_mut() {
            if identidade.janela.as_ref() == Some(janela) {
                identidade.janela = None;
                identidade.certeza = Certeza::Fechou;
            }
        }
    }

    /// Só as sessões que ainda existem.
    pub fn manter(&mut self, existe: impl Fn(&Chave) -> bool) {
        self.mapa.retain(|chave, _| existe(chave));
    }

    pub fn de(&self, chave: &Chave) -> Option<&Identidade> {
        self.mapa.get(chave)
    }

    pub fn resumo(&self, chave: &Chave) -> Option<ResumoJanela> {
        self.mapa.get(chave).map(|i| ResumoJanela {
            endereco: i.janela.as_ref().map(|a| a.0.clone()),
            certeza: i.certeza,
            terminal: i.terminal.clone(),
        })
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
            !anel.semente(a("foot1"), T0 + 5),
            "a mesma janela não repete"
        );
        // Sem o socket2, o foreign-toplevel conta as trocas: com dúvida perto.
        assert!(anel.semente(a("foot2"), T0 + 5_000));
        assert_eq!(anel.em(T0 + 5_500), Achado::Duvida);
        assert_eq!(anel.em(T0 + 6_000), Achado::Janela(a("foot2")));
        // A fonte das trocas contou uma: a semente não entra mais.
        assert!(anel.ativou(Some(a("foot3")), T0 + 10_000));
        assert!(!anel.semente(a("foot4"), T0 + 11_000));
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
    fn identidade_certa_fica_e_a_duvida_nao_apaga() {
        let mut ids = Identidades::default();
        let s1 = (false, "s1".to_owned());
        let tmux = Terminal {
            tmux: Some("%3".into()),
            ..Terminal::default()
        };
        ids.observar(s1.clone(), Achado::Duvida, None, T0);
        assert_eq!(ids.de(&s1).unwrap().certeza, Certeza::Duvida);
        assert_eq!(ids.de(&s1).unwrap().janela, None);
        ids.observar(
            s1.clone(),
            Achado::Janela(a("foot1")),
            Some(tmux.clone()),
            T0 + 10,
        );
        assert_eq!(ids.de(&s1).unwrap().janela, Some(a("foot1")));
        // Um prompt com dúvida não apaga a janela certa de antes.
        ids.observar(s1.clone(), Achado::Duvida, None, T0 + 20);
        let i = ids.de(&s1).unwrap();
        assert_eq!(
            (i.janela.clone(), i.certeza),
            (Some(a("foot1")), Certeza::Certa)
        );
        assert_eq!(i.terminal, Some(tmux), "os ids de terminal ficam");
        // Uma certa nova troca (o `--resume` em outro terminal).
        ids.observar(s1.clone(), Achado::Janela(a("foot2")), None, T0 + 30);
        assert_eq!(ids.de(&s1).unwrap().janela, Some(a("foot2")));
        // A janela fechou.
        ids.fechou(&a("foot2"));
        let i = ids.de(&s1).unwrap();
        assert_eq!((i.janela.clone(), i.certeza), (None, Certeza::Fechou));
        assert_eq!(
            ids.resumo(&s1).unwrap(),
            ResumoJanela {
                endereco: None,
                certeza: Certeza::Fechou,
                terminal: Some(Terminal {
                    tmux: Some("%3".into()),
                    ..Terminal::default()
                })
            }
        );
        // A sessão acabou.
        ids.manter(|_| false);
        assert!(ids.de(&s1).is_none());
        let mut outras = Identidades::default();
        outras.observar((true, "t".into()), Achado::Nenhuma, None, T0);
        assert_eq!(
            outras.de(&(true, "t".into())).unwrap().certeza,
            Certeza::SemJanela
        );
        outras.observar((true, "u".into()), Achado::Desconhecida, None, T0);
        assert_eq!(
            outras.de(&(true, "u".into())).unwrap().certeza,
            Certeza::SemAnel
        );
        assert!(!Certeza::Fechou.motivo().is_empty());
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
