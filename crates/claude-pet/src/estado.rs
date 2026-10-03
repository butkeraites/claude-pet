//! Estado compartilhado entre as threads: batimento do laço principal (lido
//! pelo vigia e pelo `/saude`), situação da tela e a configuração efetiva.

use std::sync::RwLock;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use pet_core::config::ConfigEfetiva;
use serde_json::{Value, json};

/// Batimento mais velho que isto: `/saude` responde 503.
pub const LIMITE_SAUDE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tela {
    /// Sem compositor: antes do login, depois do logout, ou Hyprland caído.
    Aguardando = 0,
    /// Conectado ao Wayland e ao Hyprland (a partir do M1).
    Ativa = 1,
}

impl Tela {
    fn de_u8(valor: u8) -> Self {
        if valor == Tela::Ativa as u8 {
            Tela::Ativa
        } else {
            Tela::Aguardando
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Tela::Aguardando => "aguardando",
            Tela::Ativa => "ativa",
        }
    }
}

pub struct Compartilhado {
    inicio: Instant,
    /// Milissegundos desde `inicio` no último batimento.
    batimento_ms: AtomicU64,
    tela: AtomicU8,
    config: RwLock<ConfigEfetiva>,
    debug: bool,
}

impl Compartilhado {
    pub fn novo(config: ConfigEfetiva, debug: bool) -> Self {
        Compartilhado {
            inicio: Instant::now(),
            batimento_ms: AtomicU64::new(0),
            tela: AtomicU8::new(Tela::Aguardando as u8),
            config: RwLock::new(config),
            debug,
        }
    }

    /// Chamado pelo laço principal, independente de desenhar ou não.
    pub fn bater(&self) {
        let agora = self.inicio.elapsed().as_millis() as u64;
        self.batimento_ms.store(agora, Ordering::Relaxed);
    }

    pub fn idade_batimento(&self) -> Duration {
        let agora = self.inicio.elapsed().as_millis() as u64;
        Duration::from_millis(agora.saturating_sub(self.batimento_ms.load(Ordering::Relaxed)))
    }

    pub fn saudavel(&self) -> bool {
        self.idade_batimento() < LIMITE_SAUDE
    }

    pub fn tela(&self) -> Tela {
        Tela::de_u8(self.tela.load(Ordering::Relaxed))
    }

    pub fn definir_tela(&self, tela: Tela) {
        self.tela.store(tela as u8, Ordering::Relaxed);
    }

    pub fn saude_json(&self) -> Value {
        json!({
            "ok": self.saudavel(),
            "versao": pet_core::VERSAO,
            "tela": self.tela().nome(),
            "batimento_ms": self.idade_batimento().as_millis() as u64,
        })
    }

    /// Fotografia só com metadados, para `bin/pet estado`.
    pub fn estado_json(&self) -> Value {
        let config = self.config.read().expect("lock da config envenenado");
        json!({
            "versao": pet_core::VERSAO,
            "tela": self.tela().nome(),
            "desde_s": self.inicio.elapsed().as_secs(),
            "batimento_ms": self.idade_batimento().as_millis() as u64,
            "debug": self.debug,
            "config": &*config,
        })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn novo() -> Compartilhado {
        Compartilhado::novo(ConfigEfetiva::carregar(None, |_| None), false)
    }

    #[test]
    fn comeca_aguardando_e_saudavel() {
        let c = novo();
        c.bater();
        assert_eq!(c.tela(), Tela::Aguardando);
        assert!(c.saudavel());
        assert_eq!(c.saude_json()["tela"], "aguardando");
    }

    #[test]
    fn estado_mostra_config_com_origem() {
        let c = novo();
        let estado = c.estado_json();
        assert_eq!(
            estado["config"]["chaves"]["aparencia.skin"]["origem"],
            "padrao"
        );
        assert_eq!(estado["versao"], pet_core::VERSAO);
    }

    #[test]
    fn tela_ativa() {
        let c = novo();
        c.definir_tela(Tela::Ativa);
        assert_eq!(c.tela(), Tela::Ativa);
    }
}
