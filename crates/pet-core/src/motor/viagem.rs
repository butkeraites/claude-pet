//! Seguir o monitor ativo (M4, decisão 0051): quando ir e em que pé está a
//! viagem. Puro; quem mexe na janela é o Motor.
//!
//! O monitor em foco chega pelo desktop (no Hyprland, o `focusedmonv2` do
//! socket2). Só o último valor vale, depois de [`DEBOUNCE_MS`] parado (a
//! proteção de tela do Omarchy foca cada monitor em sequência). Não viaja
//! arrastando, nem com o pet escondido (quando ele voltar, nasce no monitor
//! em foco), nem a menos de [`INTERVALO_MS`] da viagem anterior (adia). Mais
//! de [`VIAGENS_ATE_RAPIDA`] viagens em [`JANELA_MS`] viram rápidas: sem o
//! poof.
//!
//! A viagem: o poof de saída, o quadro transparente e a destruição da
//! camada, a camada nova com output NULL (o compositor a põe no monitor em
//! foco), o palco na posição salva daquele monitor (ou no ponto onde o pet
//! foi solto, se veio de um arraste para fora) e o poof de chegada.

use std::collections::VecDeque;

use super::poof;

/// O monitor em foco tem de ficar parado isto antes de o pet ir.
pub const DEBOUNCE_MS: u64 = 300;
/// Intervalo mínimo entre duas viagens.
pub const INTERVALO_MS: u64 = 1_500;
/// A janela da contagem de viagens.
pub const JANELA_MS: u64 = 20_000;
/// Viagens na janela a partir das quais a próxima é rápida (sem poof).
pub const VIAGENS_ATE_RAPIDA: usize = 3;

/// Onde pousar no monitor novo, quando a viagem vem de um arraste solto
/// fora do monitor: o ponto do desktop onde o botão subiu e onde o ponteiro
/// pegou o pet (relativo à célula), em pixels lógicos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pouso {
    pub global: (f64, f64),
    pub pegada: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fase {
    /// O poof de saída, desde `inicio_ms`.
    Poof { inicio_ms: u64 },
    /// O quadro transparente foi; esperando a camada morrer.
    Saindo,
    /// A camada nova foi pedida; esperando ela ficar pronta.
    Chegando,
    /// O poof de chegada, desde `inicio_ms`.
    Entrando { inicio_ms: u64 },
}

impl Fase {
    pub fn nome(self) -> &'static str {
        match self {
            Fase::Poof { .. } => "poof",
            Fase::Saindo => "saindo",
            Fase::Chegando => "chegando",
            Fase::Entrando { .. } => "entrando",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viagem {
    pub fase: Fase,
    /// Sem poof (muitas viagens seguidas, ou o pet nem estava desenhado).
    pub rapida: bool,
    pub pouso: Option<Pouso>,
}

/// O que fazer quando o prazo vence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decisao {
    Nada,
    Viajar,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Seguir {
    /// O último monitor em foco que o desktop contou.
    pub alvo: Option<String>,
    /// Quando conferir se é hora de ir (o debounce, ou o intervalo).
    prazo: Option<u64>,
    /// O início das viagens recentes.
    viagens: VecDeque<u64>,
    pub viagem: Option<Viagem>,
}

impl Seguir {
    /// O monitor em foco mudou: confere depois do debounce (só o último
    /// valor conta).
    pub fn foco(&mut self, nome: String, agora_ms: u64) {
        self.alvo = Some(nome);
        self.prazo = Some(agora_ms + DEBOUNCE_MS);
    }

    /// Confere de novo em `quando` (depois de soltar o pet, do fim de uma
    /// viagem, de a camada ficar pronta).
    pub fn conferir_em(&mut self, quando: u64) {
        if self.alvo.is_some() {
            self.prazo = Some(self.prazo.map_or(quando, |p| p.min(quando)));
        }
    }

    /// O próximo prazo: conferir o foco, ou o fim de um poof.
    pub fn prazo(&self) -> Option<u64> {
        let do_poof = match self.viagem.map(|v| v.fase) {
            Some(Fase::Poof { inicio_ms } | Fase::Entrando { inicio_ms }) => {
                Some(inicio_ms + poof::DURACAO_MS)
            }
            _ => None,
        };
        [self.prazo, do_poof].into_iter().flatten().min()
    }

    /// O prazo de conferir venceu: viajar? `monitor` é onde a janela do pet
    /// está pronta agora; `livre`, se o pet está na tela e solto (sem
    /// arraste). Perto demais da viagem anterior, adia.
    pub fn decidir(&mut self, agora_ms: u64, monitor: Option<&str>, livre: bool) -> Decisao {
        if self.prazo.is_none_or(|p| p > agora_ms) {
            return Decisao::Nada;
        }
        self.prazo = None;
        if self.viagem.is_some() || !livre {
            return Decisao::Nada;
        }
        let (Some(alvo), Some(monitor)) = (self.alvo.as_deref(), monitor) else {
            return Decisao::Nada;
        };
        if alvo == monitor {
            return Decisao::Nada;
        }
        if let Some(&ultima) = self.viagens.back()
            && agora_ms < ultima + INTERVALO_MS
        {
            self.prazo = Some(ultima + INTERVALO_MS);
            return Decisao::Nada;
        }
        Decisao::Viajar
    }

    /// Começa uma viagem; `com_poof`: o pet está desenhado (senão nem há o
    /// que esfumaçar). Devolve se ela é rápida.
    pub fn comecar(&mut self, agora_ms: u64, pouso: Option<Pouso>, com_poof: bool) -> bool {
        while self
            .viagens
            .front()
            .is_some_and(|&t| agora_ms.saturating_sub(t) >= JANELA_MS)
        {
            self.viagens.pop_front();
        }
        let rapida = !com_poof || self.viagens.len() >= VIAGENS_ATE_RAPIDA;
        self.viagens.push_back(agora_ms);
        self.prazo = None;
        self.viagem = Some(Viagem {
            fase: if rapida {
                Fase::Saindo
            } else {
                Fase::Poof {
                    inicio_ms: agora_ms,
                }
            },
            rapida,
            pouso,
        });
        rapida
    }

    pub fn em_viagem(&self) -> bool {
        self.viagem.is_some()
    }

    /// A fase da viagem, se houver.
    pub fn fase(&self) -> Option<Fase> {
        self.viagem.map(|v| v.fase)
    }

    pub fn mudar_fase(&mut self, fase: Fase) {
        if let Some(viagem) = self.viagem.as_mut() {
            viagem.fase = fase;
        }
    }

    /// Acabou (chegou, ou desistiu porque o pet foi escondido).
    pub fn terminar(&mut self) -> Option<Viagem> {
        self.viagem.take()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_o_ultimo_foco_depois_do_debounce() {
        let mut s = Seguir::default();
        s.foco("HDMI-A-1".into(), 0);
        s.foco("eDP-1".into(), 100);
        assert_eq!(s.prazo(), Some(400));
        assert_eq!(s.decidir(399, Some("eDP-1"), true), Decisao::Nada);
        assert_eq!(s.decidir(400, Some("eDP-1"), true), Decisao::Nada, "voltou");
        assert_eq!(s.prazo(), None);
        s.foco("HDMI-A-1".into(), 1_000);
        assert_eq!(s.decidir(1_300, Some("eDP-1"), true), Decisao::Viajar);
    }

    #[test]
    fn nao_viaja_arrastando_escondido_ou_sem_janela_pronta() {
        let mut s = Seguir::default();
        s.foco("HDMI-A-1".into(), 0);
        assert_eq!(s.decidir(300, Some("eDP-1"), false), Decisao::Nada);
        s.conferir_em(500);
        assert_eq!(
            s.decidir(500, None, true),
            Decisao::Nada,
            "sem janela pronta"
        );
        s.conferir_em(600);
        assert_eq!(s.decidir(600, Some("eDP-1"), true), Decisao::Viajar);
        // Sem foco conhecido, conferir não faz nada.
        let mut vazio = Seguir::default();
        vazio.conferir_em(10);
        assert_eq!(vazio.prazo(), None);
    }

    #[test]
    fn intervalo_entre_viagens_e_a_rapida() {
        let mut s = Seguir::default();
        s.foco("HDMI-A-1".into(), 0);
        assert_eq!(s.decidir(300, Some("eDP-1"), true), Decisao::Viajar);
        assert!(!s.comecar(300, None, true), "a primeira tem poof");
        assert_eq!(s.fase(), Some(Fase::Poof { inicio_ms: 300 }));
        assert_eq!(s.prazo(), Some(300 + poof::DURACAO_MS));
        s.terminar();
        // Volta 500 ms depois: adia até 1,5 s da anterior.
        s.foco("eDP-1".into(), 500);
        assert_eq!(s.decidir(800, Some("HDMI-A-1"), true), Decisao::Nada);
        assert_eq!(s.prazo(), Some(1_800));
        assert_eq!(s.decidir(1_800, Some("HDMI-A-1"), true), Decisao::Viajar);
        assert!(!s.comecar(1_800, None, true));
        s.terminar();
        assert!(!s.comecar(3_300, None, true), "a terceira ainda tem poof");
        s.terminar();
        assert!(s.comecar(4_800, None, true), "a quarta em 20 s é rápida");
        assert_eq!(s.fase(), Some(Fase::Saindo));
        s.terminar();
        // Passados 20 s, volta a ter poof.
        assert!(!s.comecar(30_000, None, true));
        s.terminar();
        assert!(
            s.comecar(31_000, None, false),
            "sem pet desenhado, sem poof"
        );
    }

    #[test]
    fn em_viagem_nao_decide_e_as_fases_mudam() {
        let mut s = Seguir::default();
        s.comecar(0, None, true);
        s.foco("HDMI-A-1".into(), 10);
        assert_eq!(
            s.decidir(310, Some("eDP-1"), true),
            Decisao::Nada,
            "em viagem"
        );
        s.mudar_fase(Fase::Chegando);
        assert_eq!(s.fase().map(Fase::nome), Some("chegando"));
        assert_eq!(s.prazo(), None, "esperando a janela");
        s.mudar_fase(Fase::Entrando { inicio_ms: 1_000 });
        assert_eq!(s.prazo(), Some(1_000 + poof::DURACAO_MS));
        assert!(s.terminar().is_some());
        assert!(!s.em_viagem());
    }
}
