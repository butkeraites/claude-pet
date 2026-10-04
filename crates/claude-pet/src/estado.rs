//! Estado compartilhado entre as threads: batimento do laço principal (lido
//! pelo vigia e pelo `/saude`), situação da tela e a configuração efetiva.

use std::sync::RwLock;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use pet_core::config::ConfigEfetiva;
use pet_core::geometria::Ret;
use serde::Serialize;
use serde_json::{Value, json};

/// Batimento mais velho que isto: `/saude` responde 503.
pub const LIMITE_SAUDE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tela {
    /// Sem compositor: antes do login, depois do logout, ou Hyprland caído.
    Aguardando = 0,
    /// Conectado, com personagem.
    Ativa = 1,
    /// Conectado, mas sem personagem aprovado: o pet fica escondido
    /// (decisão 0011; a skin de teste nunca substitui o personagem).
    SemPersonagem = 2,
}

impl Tela {
    fn de_u8(valor: u8) -> Self {
        match valor {
            1 => Tela::Ativa,
            2 => Tela::SemPersonagem,
            _ => Tela::Aguardando,
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Tela::Aguardando => "aguardando",
            Tela::Ativa => "ativa",
            Tela::SemPersonagem => "sem_personagem",
        }
    }
}

/// Qual skin está na tela e por quê (fixo desde a partida no M1).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct InfoSkin {
    /// A skin carregada; `None` quando não há personagem.
    pub id: Option<String>,
    /// A skin pedida (a de teste em debug, senão a configurada).
    pub pedida: String,
    pub avisos: Vec<String>,
}

/// O que a sessão Wayland publica para o `/v1/estado` (escrito só pela
/// thread principal; o ingress só lê).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Painel {
    /// Nome do monitor da camada (`wl_output` v4).
    pub monitor: Option<String>,
    /// Escala do monitor (`preferred_scale` ÷ 120).
    pub escala: Option<f64>,
    /// Pixels do monitor por pixel de arte.
    pub d: Option<i32>,
    /// Célula do sprite em pixels do monitor, relativa ao monitor e
    /// recortada a ele (a foto e a checagem de nitidez recortam por aqui).
    pub sprite_disp: Option<Ret>,
    /// Região clicável, em pixels lógicos da superfície.
    pub regiao_entrada: Option<Ret>,
    /// O pet está desenhado na tela.
    pub visivel: bool,
    /// Teste de estresse rodando.
    pub estresse: bool,
    /// Commits Wayland no último minuto (orçamento: até 120 parado).
    pub commits_por_min: usize,
    pub commits_total: u64,
    /// SHM do buffer do monitor (o Hyprland guarda uma textura do mesmo
    /// tamanho, fora do container).
    pub shm_bytes: usize,
}

pub struct Compartilhado {
    inicio: Instant,
    /// Milissegundos desde `inicio` no último batimento.
    batimento_ms: AtomicU64,
    tela: AtomicU8,
    config: RwLock<ConfigEfetiva>,
    painel: RwLock<Painel>,
    skin: RwLock<InfoSkin>,
    debug: bool,
}

impl Compartilhado {
    pub fn novo(config: ConfigEfetiva, debug: bool) -> Self {
        Compartilhado {
            inicio: Instant::now(),
            batimento_ms: AtomicU64::new(0),
            tela: AtomicU8::new(Tela::Aguardando as u8),
            config: RwLock::new(config),
            painel: RwLock::new(Painel::default()),
            skin: RwLock::new(InfoSkin::default()),
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

    pub fn definir_skin(&self, info: InfoSkin) {
        *self.skin.write().expect("lock da skin envenenado") = info;
    }

    pub fn publicar_painel(&self, painel: Painel) {
        *self.painel.write().expect("lock do painel envenenado") = painel;
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
        let painel = self.painel.read().expect("lock do painel envenenado");
        let skin = self.skin.read().expect("lock da skin envenenado");
        json!({
            "versao": pet_core::VERSAO,
            "tela": self.tela().nome(),
            "desde_s": self.inicio.elapsed().as_secs(),
            "batimento_ms": self.idade_batimento().as_millis() as u64,
            "debug": self.debug,
            "monitor": painel.monitor,
            "escala": painel.escala,
            "d": painel.d,
            "sprite_disp": painel.sprite_disp,
            "regiao_entrada": painel.regiao_entrada,
            "visivel": painel.visivel,
            "estresse": painel.estresse,
            "commits_por_min": painel.commits_por_min,
            "commits_total": painel.commits_total,
            "shm_bytes": painel.shm_bytes,
            "skin": &*skin,
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
    fn painel_aparece_no_estado() {
        let c = novo();
        c.publicar_painel(Painel {
            monitor: Some("eDP-1".into()),
            escala: Some(1.5),
            d: Some(5),
            sprite_disp: Some(Ret::novo(1706, 951, 214, 240)),
            commits_por_min: 42,
            commits_total: 7,
            shm_bytes: 9_216_000,
            ..Painel::default()
        });
        c.definir_skin(InfoSkin {
            id: None,
            pedida: "zeca".into(),
            avisos: vec!["não encontrada".into()],
        });
        let estado = c.estado_json();
        assert_eq!(estado["monitor"], "eDP-1");
        assert_eq!(estado["escala"], 1.5);
        assert_eq!(estado["sprite_disp"]["w"], 214);
        assert_eq!(estado["skin"]["pedida"], "zeca");
        assert!(estado["skin"]["id"].is_null());
        assert_eq!(estado["commits_por_min"], 42);
        assert_eq!(estado["commits_total"], 7);
        assert_eq!(estado["shm_bytes"], 9_216_000);
    }

    #[test]
    fn telas() {
        let c = novo();
        c.definir_tela(Tela::Ativa);
        assert_eq!(c.tela(), Tela::Ativa);
        c.definir_tela(Tela::SemPersonagem);
        assert_eq!(c.tela(), Tela::SemPersonagem);
        assert_eq!(c.saude_json()["tela"], "sem_personagem");
    }
}
