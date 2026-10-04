//! Pedidos de outras threads para o laço principal.
//!
//! Chegam por um canal do calloop (limitado: um ingress afobado recebe 503
//! em vez de crescer a memória): os eventos dos hooks (`/v1/evento`), os
//! comandos do `/v1/comando` (as reações do M3 e as aprovações de
//! personagem do M2) e as rotas de debug.

use std::sync::mpsc::SyncSender;
use std::time::Instant;

use pet_core::evento::Evento;
use pet_core::geometria::Ret;

/// Quantos comandos esperam no canal antes de o ingress responder 503
/// (folga para rajadas de PostToolUse de ferramentas em paralelo enquanto o
/// laço espera o compositor numa ida e volta com prazo).
pub const CAPACIDADE: usize = 256;

/// O sprite como deveria estar na tela: o RGBA exato, em pixels do
/// monitor, para a checagem de nitidez comparar com uma captura do grim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuadroEsperado {
    pub monitor: String,
    /// Recorte em pixels do monitor (o `sprite_disp`).
    pub area: Ret,
    /// Canto da célula: origem da grade de blocos D×D.
    pub grade: (i32, i32),
    pub d: i32,
    /// Número do commit que pôs este quadro na tela.
    pub seq: u64,
    /// Há quanto tempo foi esse commit.
    pub idade_ms: u64,
    /// RGBA direto, `area.w * area.h * 4` bytes.
    pub rgba: Vec<u8>,
}

/// Um evento de hook já validado e a hora em que o ingress o recebeu: em ms
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

/// O que um `tocar` do `/v1/comando` fez (decisão 0033).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tocou {
    /// Tocou na tela a tag `tag` da skin.
    NaTela { tag: String },
    /// A skin sabe tocar (`tag`), mas não há onde mostrar: sem compositor,
    /// ou o pet escondido.
    ForaDaTela { tag: String, motivo: &'static str },
    /// Nenhum personagem aprovado.
    SemPersonagem,
    /// A skin não tem estado, reserva nem tag com esse nome.
    Desconhecida { skin: String },
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
