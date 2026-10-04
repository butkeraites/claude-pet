//! Log mínimo no stderr (o Docker guarda e põe data). Nível em `PET_LOG`:
//! `off`, `error`, `warn`, `info` (padrão) ou `debug`.
//!
//! Mora no core desde o T8.0 (decisão 0040) para o Motor, o daemon e os
//! backends de cada sistema registrarem do mesmo jeito. Só usa a `std`.
//!
//! Regra de ouro: nunca registrar conteúdo vindo dos hooks ou títulos de
//! janela — só nomes de evento, ids e contagens. O hook (`avisar`) nunca
//! registra nada: o binário chama [`desligar`] antes dele, sem olhar o
//! `PET_LOG` (decisão 0045).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

pub const ERRO: u8 = 0;
pub const AVISO: u8 = 1;
pub const INFO: u8 = 2;
pub const DEPURAR: u8 = 3;

static NIVEL: AtomicU8 = AtomicU8::new(INFO);
static DESLIGADO: AtomicBool = AtomicBool::new(false);

pub fn iniciar(valor: Option<&str>) {
    let nivel = match valor.map(str::trim) {
        Some("off") => {
            desligar();
            return;
        }
        Some("error") => ERRO,
        Some("warn") => AVISO,
        Some("debug") => DEPURAR,
        _ => INFO,
    };
    NIVEL.store(nivel, Ordering::Relaxed);
}

/// Nada mais vai para o log, nem erro: o hook nunca imprime.
pub fn desligar() {
    DESLIGADO.store(true, Ordering::Relaxed);
}

pub fn ativo(nivel: u8) -> bool {
    !DESLIGADO.load(Ordering::Relaxed) && nivel <= NIVEL.load(Ordering::Relaxed)
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

#[macro_export]
macro_rules! erro {
    ($($arg:tt)*) => { $crate::registro::escrever($crate::registro::ERRO, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! aviso {
    ($($arg:tt)*) => { $crate::registro::escrever($crate::registro::AVISO, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::registro::escrever($crate::registro::INFO, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! depurar {
    ($($arg:tt)*) => { $crate::registro::escrever($crate::registro::DEPURAR, format_args!($($arg)*)) };
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn desligado_nao_registra_nem_erro() {
        iniciar(Some("debug"));
        assert!(ativo(DEPURAR) && ativo(ERRO));
        iniciar(Some("off"));
        assert!(!ativo(ERRO), "off desliga tudo");
        // Depois de desligado, nenhum nível religa (o hook segue calado).
        DESLIGADO.store(false, Ordering::Relaxed);
        desligar();
        iniciar(Some("debug"));
        assert!(!ativo(ERRO));
        DESLIGADO.store(false, Ordering::Relaxed);
        iniciar(None);
        assert!(ativo(INFO) && !ativo(DEPURAR));
    }
}
