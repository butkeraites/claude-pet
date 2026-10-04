//! Núcleo puro do claude-pet.
//!
//! Tudo aqui é determinístico e testável sem compositor: configuração,
//! cérebro (eventos do Claude Code → reações), pontuação, animador, skin e
//! raster. Este crate **nunca** depende de crates Wayland (CLAUDE.md).

pub mod animador;
pub mod cena;
pub mod confete;
pub mod config;
pub mod geometria;
pub mod raster;
pub mod skin;

/// Versão do workspace, igual para o daemon, o core e o xtask.
pub const VERSAO: &str = env!("CARGO_PKG_VERSION");
