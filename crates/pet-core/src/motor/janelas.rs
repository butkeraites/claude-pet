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

use serde::{Deserialize, Serialize};

use crate::evento::Terminal;
use crate::memoria::JanelaGuardada;
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

/// De que evento do Claude vem a hora casada com o anel (decisão 0060).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origem {
    /// O prompt que o Renan mandou do teclado: a janela ativa é o terminal
    /// da sessão, e ela troca a de antes (um `--resume` em outro terminal).
    Prompt,
    /// O começo da sessão (`startup`, `resume`, `clear`, `fork`): o hook roda
    /// depois de o Claude Code subir, e o Renan pode já estar noutra janela.
    /// Só preenche uma janela que ainda não é certa.
    Comeco,
}

/// Se o evento `e` (com o `source` dele, `src`) casa a janela da sessão, e
/// como. O `SessionStart` de uma compactação (`compact`) chega no meio de um
/// turno longo, com o Renan em qualquer janela: nunca casa. Um prompt que não
/// veio do teclado (`src` que não é `user`), também não. Uma origem
/// desconhecida não casa.
pub fn origem(e: &str, src: Option<&str>) -> Option<Origem> {
    match e {
        "UserPromptSubmit" if src.is_none_or(|s| s == "user") => Some(Origem::Prompt),
        "SessionStart"
            if src.is_none_or(|s| matches!(s, "startup" | "resume" | "clear" | "fork")) =>
        {
            Some(Origem::Comeco)
        }
        _ => None,
    }
}

/// O quanto se sabe da janela de uma sessão (decisão 0055).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// A instância do compositor em que a janela foi vista (no Hyprland, a
    /// assinatura; decisão 0093): noutra instância, o mesmo endereço não é a
    /// mesma janela.
    pub compositor: Option<String>,
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
    /// A instância do compositor de agora, se se sabe (decisão 0093).
    compositor: Option<String>,
}

impl Identidades {
    /// Um `SessionStart` ou `UserPromptSubmit` da sessão `chave`, na hora
    /// `ts` (o `ts` do hook), com o que o anel achou nessa hora. A janela
    /// certa de um prompt troca a de antes; a do começo da sessão só
    /// preenche uma que ainda não é certa (decisão 0060). Sem certeza, a de
    /// antes fica (o terminal de uma sessão não muda), e só sem nenhuma a
    /// dúvida é guardada. Um hook atrasado (os hooks são assíncronos e chegam
    /// fora de ordem), com a hora mais velha que a do último casado, não
    /// desfaz o que o mais novo decidiu.
    pub fn observar(
        &mut self,
        chave: Chave,
        achado: Achado,
        terminal: Option<Terminal>,
        ts: u64,
        origem: Origem,
    ) {
        let compositor = self.compositor.clone();
        let atual = self.mapa.entry(chave).or_insert(Identidade {
            janela: None,
            certeza: Certeza::SemAnel,
            terminal: None,
            em_ms: ts,
            compositor: None,
        });
        if ts < atual.em_ms {
            return;
        }
        atual.em_ms = ts;
        if terminal.is_some() {
            atual.terminal = terminal;
        }
        match achado {
            Achado::Janela(janela)
                if origem == Origem::Prompt || atual.certeza != Certeza::Certa =>
            {
                atual.janela = Some(janela);
                atual.certeza = Certeza::Certa;
                atual.compositor = compositor;
            }
            // O começo da sessão não troca uma janela certa.
            Achado::Janela(_) => {}
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

    /// A instância do compositor de agora (decisão 0093): as janelas vistas
    /// noutra (um logout e um login sem reiniciar a máquina; a memória das
    /// sessões de antes) saem, e a sessão fica sem janela até o próximo
    /// prompt casar de novo (as sessões do tmux sobrevivem ao logout). Sem
    /// instância (o compositor caiu), nada muda. Devolve quantas saíram.
    pub fn definir_compositor(&mut self, instancia: Option<String>) -> usize {
        let Some(agora) = instancia else {
            return 0;
        };
        let mut sairam = 0;
        for identidade in self.mapa.values_mut() {
            if identidade.janela.is_some() && identidade.compositor.as_ref() != Some(&agora) {
                identidade.janela = None;
                identidade.certeza = Certeza::Fechou;
                sairam += 1;
            }
        }
        self.compositor = Some(agora);
        sairam
    }

    /// A instância do compositor de agora, se se sabe.
    pub fn compositor(&self) -> Option<&str> {
        self.compositor.as_deref()
    }

    /// A janela da sessão para a memória das sessões (decisão 0093).
    pub fn guardada(&self, chave: &Chave) -> Option<JanelaGuardada> {
        self.mapa.get(chave).map(|i| JanelaGuardada {
            endereco: i.janela.as_ref().map(|a| a.0.clone()),
            compositor: i.compositor.clone(),
            certeza: i.certeza,
            em_ms: i.em_ms,
            terminal: i.terminal.clone(),
        })
    }

    /// A janela de uma sessão que a memória restaurou. Uma janela de outra
    /// instância do compositor (já conhecida) sai na hora; com o compositor
    /// ainda por achar (a partida da máquina), ela espera a instância dele.
    pub fn restaurar(&mut self, chave: Chave, guardada: &JanelaGuardada) {
        let mut identidade = Identidade {
            janela: guardada.endereco.clone().map(Alca),
            certeza: guardada.certeza,
            terminal: guardada.terminal.clone(),
            em_ms: guardada.em_ms,
            compositor: guardada.compositor.clone(),
        };
        if let Some(agora) = &self.compositor
            && identidade.janela.is_some()
            && identidade.compositor.as_ref() != Some(agora)
        {
            identidade.janela = None;
            identidade.certeza = Certeza::Fechou;
        }
        self.mapa.insert(chave, identidade);
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
        ids.observar(s1.clone(), Achado::Duvida, None, T0, Origem::Prompt);
        assert_eq!(ids.de(&s1).unwrap().certeza, Certeza::Duvida);
        assert_eq!(ids.de(&s1).unwrap().janela, None);
        ids.observar(
            s1.clone(),
            Achado::Janela(a("foot1")),
            Some(tmux.clone()),
            T0 + 10,
            Origem::Prompt,
        );
        assert_eq!(ids.de(&s1).unwrap().janela, Some(a("foot1")));
        // Um prompt com dúvida não apaga a janela certa de antes.
        ids.observar(s1.clone(), Achado::Duvida, None, T0 + 20, Origem::Prompt);
        let i = ids.de(&s1).unwrap();
        assert_eq!(
            (i.janela.clone(), i.certeza),
            (Some(a("foot1")), Certeza::Certa)
        );
        assert_eq!(i.terminal, Some(tmux), "os ids de terminal ficam");
        // Uma certa nova troca (o `--resume` em outro terminal).
        ids.observar(
            s1.clone(),
            Achado::Janela(a("foot2")),
            None,
            T0 + 30,
            Origem::Prompt,
        );
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
        outras.observar(
            (true, "t".into()),
            Achado::Nenhuma,
            None,
            T0,
            Origem::Prompt,
        );
        assert_eq!(
            outras.de(&(true, "t".into())).unwrap().certeza,
            Certeza::SemJanela
        );
        outras.observar(
            (true, "u".into()),
            Achado::Desconhecida,
            None,
            T0,
            Origem::Prompt,
        );
        assert_eq!(
            outras.de(&(true, "u".into())).unwrap().certeza,
            Certeza::SemAnel
        );
        assert!(!Certeza::Fechou.motivo().is_empty());
    }

    #[test]
    fn so_o_prompt_do_teclado_e_o_comeco_casam_e_a_compactacao_nunca() {
        assert_eq!(origem("UserPromptSubmit", None), Some(Origem::Prompt));
        assert_eq!(
            origem("UserPromptSubmit", Some("user")),
            Some(Origem::Prompt)
        );
        assert_eq!(origem("UserPromptSubmit", Some("system")), None);
        for src in [
            None,
            Some("startup"),
            Some("resume"),
            Some("clear"),
            Some("fork"),
        ] {
            assert_eq!(origem("SessionStart", src), Some(Origem::Comeco), "{src:?}");
        }
        assert_eq!(origem("SessionStart", Some("compact")), None);
        assert_eq!(origem("SessionStart", Some("novo_no_futuro")), None);
        assert_eq!(origem("Stop", None), None);
    }

    #[test]
    fn o_comeco_so_preenche_e_o_hook_atrasado_nao_desfaz() {
        let mut ids = Identidades::default();
        let s = (false, "s".to_owned());
        // O começo sem janela certa preenche.
        ids.observar(
            s.clone(),
            Achado::Janela(a("foot1")),
            None,
            T0,
            Origem::Comeco,
        );
        assert_eq!(ids.de(&s).unwrap().janela, Some(a("foot1")));
        // Um começo depois (o `resume` com o Renan noutra janela) não troca a
        // certa; o prompt troca.
        ids.observar(
            s.clone(),
            Achado::Janela(a("navegador")),
            None,
            T0 + 1_000,
            Origem::Comeco,
        );
        assert_eq!(ids.de(&s).unwrap().janela, Some(a("foot1")));
        ids.observar(
            s.clone(),
            Achado::Janela(a("foot2")),
            None,
            T0 + 20_000,
            Origem::Prompt,
        );
        assert_eq!(ids.de(&s).unwrap().janela, Some(a("foot2")));
        // O prompt de antes chega atrasado (os hooks são assíncronos): o mais
        // novo fica, até com os ids de terminal.
        let tmux = Terminal {
            tmux: Some("%9".into()),
            ..Terminal::default()
        };
        ids.observar(
            s.clone(),
            Achado::Janela(a("foot1")),
            Some(tmux),
            T0 + 10_000,
            Origem::Prompt,
        );
        let i = ids.de(&s).unwrap();
        assert_eq!((i.janela.clone(), i.em_ms), (Some(a("foot2")), T0 + 20_000));
        assert_eq!(i.terminal, None);
        // Com a janela fechada, o começo preenche de novo.
        ids.fechou(&a("foot2"));
        ids.observar(
            s.clone(),
            Achado::Janela(a("foot3")),
            None,
            T0 + 30_000,
            Origem::Comeco,
        );
        assert_eq!(ids.de(&s).unwrap().janela, Some(a("foot3")));
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
