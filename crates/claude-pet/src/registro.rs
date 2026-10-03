//! Log mínimo no stderr (o Docker guarda e põe data). Nível em `PET_LOG`:
//! `error`, `warn`, `info` (padrão) ou `debug`.
//!
//! Regra de ouro: nunca registrar conteúdo vindo dos hooks ou títulos de
//! janela — só nomes de evento, ids e contagens.

use std::sync::atomic::{AtomicU8, Ordering};

pub const ERRO: u8 = 0;
pub const AVISO: u8 = 1;
pub const INFO: u8 = 2;
pub const DEPURAR: u8 = 3;

static NIVEL: AtomicU8 = AtomicU8::new(INFO);

pub fn iniciar(valor: Option<&str>) {
    let nivel = match valor.map(str::trim) {
        Some("error") => ERRO,
        Some("warn") => AVISO,
        Some("debug") => DEPURAR,
        _ => INFO,
    };
    NIVEL.store(nivel, Ordering::Relaxed);
}

pub fn ativo(nivel: u8) -> bool {
    nivel <= NIVEL.load(Ordering::Relaxed)
}

pub fn escrever(nivel: u8, mensagem: std::fmt::Arguments<'_>) {
    if ativo(nivel) {
        let rotulo = match nivel {
            ERRO => "ERRO",
            AVISO => "AVISO",
            INFO => "info",
            _ => "debug",
        };
        eprintln!("{rotulo} {mensagem}");
    }
}

macro_rules! erro {
    ($($arg:tt)*) => { $crate::registro::escrever($crate::registro::ERRO, format_args!($($arg)*)) };
}

macro_rules! aviso {
    ($($arg:tt)*) => { $crate::registro::escrever($crate::registro::AVISO, format_args!($($arg)*)) };
}

macro_rules! info {
    ($($arg:tt)*) => { $crate::registro::escrever($crate::registro::INFO, format_args!($($arg)*)) };
}

macro_rules! depurar {
    ($($arg:tt)*) => { $crate::registro::escrever($crate::registro::DEPURAR, format_args!($($arg)*)) };
}
