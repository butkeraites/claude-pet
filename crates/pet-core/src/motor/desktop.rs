//! O que o Motor sabe do desktop (decisão 0043): a fonte dos eventos ligada
//! ou não, o monitor em foco, a janela ativa e a presença. Só ids opacos e
//! booleanos: nada de título, classe ou nome de área de trabalho.

use serde::Serialize;

use super::janelas::{Anel, PainelTroca};
use crate::plataforma::{Alca, CapDesktop, EventoDesktop, InfoDesktop};

/// O estado do desktop como os eventos contaram.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EstadoDesktop {
    /// `None`: a fonte dos eventos nunca contou nada (sem compositor, ou um
    /// desktop sem eventos).
    pub ligado: Option<bool>,
    pub monitor_em_foco: Option<String>,
    pub janela_ativa: Option<Alca>,
    pub olhando_claude: bool,
    /// As últimas trocas de janela ativa (decisão 0050).
    pub anel: Anel,
}

impl EstadoDesktop {
    /// Aplica um evento ao que se sabe; `parede_ms` é a hora de agora (ms
    /// desde 1970), para marcar um buraco no anel quando a fonte cai.
    /// Devolve se algo mudou.
    pub fn aplicar(&mut self, evento: &EventoDesktop, parede_ms: u64) -> bool {
        let antes = self.clone();
        match evento {
            EventoDesktop::Ligado(ligado) => {
                self.ligado = Some(*ligado);
                if !ligado {
                    // O que vier depois de voltar é o que vale; o que se
                    // sabia pode ter mudado no meio.
                    self.olhando_claude = false;
                    self.anel.buraco(parede_ms);
                }
            }
            EventoDesktop::MonitorEmFoco(nome) => self.monitor_em_foco = Some(nome.clone()),
            EventoDesktop::JanelaAtiva {
                janela,
                parede_ms: quando,
            } => {
                self.janela_ativa = janela.clone();
                self.anel.ativou(janela.clone(), *quando);
            }
            EventoDesktop::JanelaInicial {
                janela,
                parede_ms: quando,
            } => {
                if self.anel.semente(janela.clone(), *quando) || self.janela_ativa.is_none() {
                    self.janela_ativa = Some(janela.clone());
                }
            }
            EventoDesktop::OlhandoClaude(olhando) => self.olhando_claude = *olhando,
            EventoDesktop::JanelaFechou(janela) => {
                if self.janela_ativa.as_ref() == Some(janela) {
                    self.janela_ativa = None;
                }
            }
            EventoDesktop::JanelaAbriu { .. } | EventoDesktop::Monitores => {}
        }
        *self != antes
    }

    /// O pedaço do painel do `/v1/estado`, com o que a ligação da conexão
    /// atual diz que sabe fazer.
    pub fn painel(&self, capacidades: CapDesktop, info: InfoDesktop) -> PainelDesktop {
        PainelDesktop {
            eventos: match self.ligado {
                None => "sem",
                Some(true) => "ligado",
                Some(false) => "caiu",
            },
            monitor_em_foco: self.monitor_em_foco.clone(),
            janela_ativa: self.janela_ativa.as_ref().map(|a| a.0.clone()),
            olhando_claude: self.olhando_claude,
            foca_janelas: capacidades.foca_janela,
            protocolos: info.protocolos,
            janelas: info.janelas,
            anel: self.anel.painel(),
        }
    }
}

/// O desktop no `/v1/estado`: só ids opacos, booleanos e contagens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PainelDesktop {
    /// A fonte dos eventos do desktop (no Hyprland, o socket2): `sem`
    /// (nunca ligou), `ligado` ou `caiu`.
    pub eventos: &'static str,
    pub monitor_em_foco: Option<String>,
    /// O endereço da janela ativa (no Hyprland, o do `activewindowv2`).
    pub janela_ativa: Option<String>,
    pub olhando_claude: bool,
    /// A conexão de agora sabe focar janelas.
    pub foca_janelas: bool,
    /// Os protocolos ligados para isso.
    pub protocolos: Vec<String>,
    /// Janelas que ela sabe focar.
    pub janelas: usize,
    /// As últimas trocas de janela ativa: o endereço e a hora, nada mais.
    pub anel: Vec<PainelTroca>,
}

impl Default for PainelDesktop {
    fn default() -> PainelDesktop {
        EstadoDesktop::default().painel(CapDesktop::default(), InfoDesktop::default())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn eventos_mudam_o_que_se_sabe_e_dizem_se_mudou() {
        let mut d = EstadoDesktop::default();
        let vazio = || (CapDesktop::default(), InfoDesktop::default());
        let (c, i) = vazio();
        assert_eq!(d.painel(c, i).eventos, "sem");
        assert!(d.aplicar(&EventoDesktop::Ligado(true), 0));
        assert!(d.aplicar(&EventoDesktop::MonitorEmFoco("eDP-1".into()), 0));
        assert!(
            !d.aplicar(&EventoDesktop::MonitorEmFoco("eDP-1".into()), 0),
            "o mesmo"
        );
        let a = Alca("5bbf4e6128f0".into());
        let ativa = EventoDesktop::JanelaAtiva {
            janela: Some(a.clone()),
            parede_ms: 10,
        };
        assert!(d.aplicar(&ativa, 10));
        assert!(!d.aplicar(&ativa, 11), "repetida: nada muda, nem o anel");
        // A semente só vale se nada mais novo chegou.
        let semente = EventoDesktop::JanelaInicial {
            janela: Alca("outra".into()),
            parede_ms: 5,
        };
        assert!(!d.aplicar(&semente, 12));
        assert!(d.aplicar(&EventoDesktop::OlhandoClaude(true), 13));
        let (c, i) = vazio();
        let p = d.painel(c, i);
        assert_eq!(p.eventos, "ligado");
        assert_eq!(p.janela_ativa.as_deref(), Some("5bbf4e6128f0"));
        assert!(p.olhando_claude);
        assert_eq!(p.anel.len(), 1);
        assert_eq!(p.anel[0].janela.as_deref(), Some("5bbf4e6128f0"));
        // A janela ativa fechou: nenhuma.
        assert!(d.aplicar(&EventoDesktop::JanelaFechou(a), 20));
        assert_eq!(d.janela_ativa, None);
        // A fonte caiu: a presença deixa de valer e o anel ganha um buraco.
        assert!(d.aplicar(&EventoDesktop::Ligado(false), 30));
        assert!(!d.olhando_claude);
        let (c, i) = vazio();
        let p = d.painel(c, i);
        assert_eq!(p.eventos, "caiu");
        assert_eq!(p.anel[0].tipo, super::super::janelas::Tipo::Buraco);
        // Num buraco, a semente entra (o foreign-toplevel diz a ativa).
        assert!(d.aplicar(&semente, 40));
        assert_eq!(d.janela_ativa, Some(Alca("outra".into())));
    }
}
