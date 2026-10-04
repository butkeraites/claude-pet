//! O que o Motor sabe do desktop (decisão 0043): a fonte dos eventos ligada
//! ou não, o monitor em foco, a janela ativa e a presença. Só ids opacos e
//! booleanos: nada de título, classe ou nome de área de trabalho.

use serde::Serialize;

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
}

impl EstadoDesktop {
    /// Aplica um evento ao que se sabe. Devolve se algo mudou.
    pub fn aplicar(&mut self, evento: &EventoDesktop) -> bool {
        let antes = self.clone();
        match evento {
            EventoDesktop::Ligado(ligado) => {
                self.ligado = Some(*ligado);
                if !ligado {
                    // O que vier depois de voltar é o que vale; o que se
                    // sabia pode ter mudado no meio.
                    self.olhando_claude = false;
                }
            }
            EventoDesktop::MonitorEmFoco(nome) => self.monitor_em_foco = Some(nome.clone()),
            EventoDesktop::JanelaAtiva { janela, .. } => self.janela_ativa = janela.clone(),
            EventoDesktop::JanelaInicial { janela, .. } => {
                if self.janela_ativa.is_none() {
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
        assert_eq!(
            d.painel(CapDesktop::default(), InfoDesktop::default())
                .eventos,
            "sem"
        );
        assert!(d.aplicar(&EventoDesktop::Ligado(true)));
        assert!(d.aplicar(&EventoDesktop::MonitorEmFoco("eDP-1".into())));
        assert!(
            !d.aplicar(&EventoDesktop::MonitorEmFoco("eDP-1".into())),
            "o mesmo"
        );
        let a = Alca("5bbf4e6128f0".into());
        assert!(d.aplicar(&EventoDesktop::JanelaAtiva {
            janela: Some(a.clone()),
            parede_ms: 10,
        }));
        // A semente só vale se nada mais novo chegou.
        assert!(!d.aplicar(&EventoDesktop::JanelaInicial {
            janela: Alca("outra".into()),
            parede_ms: 5,
        }));
        assert!(d.aplicar(&EventoDesktop::OlhandoClaude(true)));
        let p = d.painel(CapDesktop::default(), InfoDesktop::default());
        assert_eq!(p.eventos, "ligado");
        assert_eq!(p.janela_ativa.as_deref(), Some("5bbf4e6128f0"));
        assert!(p.olhando_claude);
        // A janela ativa fechou: nenhuma.
        assert!(d.aplicar(&EventoDesktop::JanelaFechou(a)));
        assert_eq!(d.janela_ativa, None);
        // A fonte caiu: a presença deixa de valer.
        assert!(d.aplicar(&EventoDesktop::Ligado(false)));
        assert!(!d.olhando_claude);
        assert_eq!(
            d.painel(CapDesktop::default(), InfoDesktop::default())
                .eventos,
            "caiu"
        );
    }
}
