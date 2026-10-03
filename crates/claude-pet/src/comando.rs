//! Pedidos de outras threads para o laço principal.
//!
//! Chegam por um canal do calloop (limitado: um ingress afobado recebe 503
//! em vez de crescer a memória). No M1 só as rotas de debug mandam
//! comandos; o `/v1/evento` dos hooks chega no M3.

use std::sync::mpsc::SyncSender;

use pet_core::geometria::Ret;

/// Quantos comandos esperam no canal antes de o ingress responder 503.
pub const CAPACIDADE: usize = 64;

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

#[derive(Debug)]
pub enum Comando {
    Esconder,
    Mostrar,
    /// Confete pela tela inteira, para medir o custo no compositor.
    Estresse {
        fps: u32,
        segundos: u32,
    },
    /// Pede o quadro esperado; `None` se o pet não está na tela.
    Quadro(SyncSender<Option<QuadroEsperado>>),
}
