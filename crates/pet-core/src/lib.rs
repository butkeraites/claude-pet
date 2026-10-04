//! Núcleo puro do bichinho.
//!
//! Tudo aqui é determinístico e testável sem compositor: configuração,
//! cérebro (eventos do Claude Code → reações), pontuação, animador, skin,
//! raster e, desde o T8.0, o Motor (`motor`) e os contratos de cada sistema
//! (`plataforma`). Este crate **nunca** depende de crates Wayland nem de
//! nenhum sistema operacional (CLAUDE.md, decisão 0040).

#[macro_use]
pub mod registro;

pub mod animador;
pub mod aprovacao;
pub mod aviso;
pub mod cena;
pub mod cerebro;
pub mod confete;
pub mod config;
pub mod estados;
pub mod evento;
pub mod geometria;
pub mod motor;
pub mod plataforma;
pub mod raster;
pub mod skin;

/// Versão do workspace, igual para o daemon, o core e o xtask.
pub const VERSAO: &str = env!("CARGO_PKG_VERSION");
