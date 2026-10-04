//! Pedidos de outras threads para o laço principal.
//!
//! Chegam pela [`pet_core::plataforma::Caixa`] (limitada: uma entrada
//! afobada recebe 503 em vez de crescer a memória), que acorda o laço de
//! cada sistema: os eventos dos hooks (`/v1/evento`), os comandos do
//! `/v1/comando` (as reações do M3, as aprovações de personagem do M2 e o
//! clique do M4) e as rotas de debug.

use std::sync::mpsc::SyncSender;
use std::time::Instant;

use pet_core::cerebro::Agora;
use pet_core::evento::Evento;
use pet_core::plataforma::Botao;

pub use pet_core::motor::{Clicou, QuadroEsperado, Tocou};

/// Quantos comandos esperam na caixa antes de a entrada responder 503
/// (folga para rajadas de PostToolUse de ferramentas em paralelo enquanto o
/// laço espera o compositor numa ida e volta com prazo).
pub const CAPACIDADE: usize = 256;

/// Um evento de hook já validado e a hora em que a entrada o recebeu: em ms
/// desde 1970 (relógio do host), que vale como hora do evento quando o `ts`
/// dele falta ou não é plausível, e no relógio monotônico, para o cérebro
/// contar os prazos a partir da chegada mesmo se o laço demorar a
/// processar (decisão 0032).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recebido {
    pub evento: Evento,
    pub recebido_ms: u64,
    pub chegada: Instant,
}

impl Recebido {
    /// O relógio do cérebro para este evento: a hora em que a entrada o
    /// recebeu (parede e monotônico desde `inicio`), não a hora do
    /// processamento. Se o laço ficou parado (o handshake do Wayland, uma
    /// troca de personagem), os eventos da caixa entram na hora em que
    /// chegaram, e o laço esvazia a caixa antes de vencer um prazo do cérebro:
    /// uma acomodação que um desses eventos cancela não vence antes dele
    /// (decisão 0032).
    pub fn agora(&self, inicio: Instant) -> Agora {
        Agora {
            parede_ms: self.recebido_ms,
            mono_ms: self.chegada.saturating_duration_since(inicio).as_millis() as u64,
        }
    }
}

#[derive(Debug)]
pub enum Comando {
    /// Evento do Claude Code (`POST /v1/evento`).
    Evento(Box<Recebido>),
    /// Toca uma reação uma vez (`/v1/comando` `tocar`); o laço responde o
    /// que fez.
    Tocar {
        reacao: String,
        resposta: SyncSender<Tocou>,
    },
    Esconder,
    Mostrar,
    /// Clica no pet como o ponteiro (`/v1/comando` `clique`, decisão 0057);
    /// o laço responde o que fez.
    Clique {
        botao: Botao,
        resposta: SyncSender<Clicou>,
    },
    /// Confete pela tela inteira, para medir o custo no compositor.
    Estresse {
        fps: u32,
        segundos: u32,
    },
    /// Pede o quadro esperado; `None` se o pet não está na tela.
    Quadro(SyncSender<Option<QuadroEsperado>>),
    /// Uma aprovação mudou: escolher o personagem de novo e trocar na tela
    /// (`/v1/comando` `aprovar_skin` e `revogar_skin`, decisão 0026). O
    /// laço avisa pelo canal quando terminou.
    RecarregarPersonagem(SyncSender<()>),
}

#[cfg(test)]
mod testes {
    use std::time::Duration;

    use super::*;

    #[test]
    fn cerebro_conta_os_prazos_da_chegada_do_evento() {
        let inicio = Instant::now();
        let recebido = |chegada: Instant| Recebido {
            evento: Evento {
                e: "Stop".into(),
                ..Default::default()
            },
            recebido_ms: 1_790_000_000_000,
            chegada,
        };
        let agora = recebido(inicio + Duration::from_millis(1_234)).agora(inicio);
        assert_eq!(
            agora,
            Agora {
                parede_ms: 1_790_000_000_000,
                mono_ms: 1_234
            },
            "a hora da chegada, mesmo processado bem depois"
        );
        // Chegou antes de o laço existir (a entrada sobe primeiro): zero.
        let cedo = inicio
            .checked_sub(Duration::from_millis(5))
            .unwrap_or(inicio);
        assert_eq!(recebido(cedo).agora(inicio).mono_ms, 0);
    }
}
