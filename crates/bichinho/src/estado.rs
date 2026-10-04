//! Estado compartilhado entre as threads: batimento do laço principal (lido
//! pelo vigia e pelo `/saude`), situação da tela, a configuração efetiva,
//! a contagem de eventos dos hooks e, em debug, os últimos eventos já
//! validados (só metadados, decisão 0019).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

use pet_core::cerebro::Resumo;
use pet_core::config::ConfigEfetiva;
use serde::Serialize;

/// O que o Motor publica para o `/v1/estado` (escrito só pelo laço; a
/// entrada HTTP só lê).
pub use pet_core::motor::Painel;
use serde_json::{Value, json};

/// Batimento mais velho que isto: `/saude` responde 503.
pub const LIMITE_SAUDE: Duration = Duration::from_secs(30);
/// Eventos guardados para o `/v1/debug/eventos` (só com `PET_DEBUG=1`).
pub const EVENTOS_DEBUG: usize = 200;

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

/// Qual skin está na tela e por quê (muda quando o Renan aprova ou revoga).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct InfoSkin {
    /// A skin carregada; `None` quando não há personagem.
    pub id: Option<String>,
    /// A skin pedida (a de teste em debug, senão a configurada).
    pub pedida: String,
    /// De onde veio o personagem aprovado: `imagem` ou `snapshot` (a cópia
    /// em `/state`).
    pub origem: Option<pet_core::aprovacao::Origem>,
    /// Impressão digital da skin na tela (decisão 0026).
    pub sha256: Option<String>,
    pub avisos: Vec<String>,
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
    /// Eventos de hook aceitos (204) e recusados (4xx) pelo ingress.
    eventos_aceitos: AtomicU64,
    eventos_recusados: AtomicU64,
    /// Milissegundos desde `inicio` no último evento aceito, mais 1 (0 =
    /// nenhum ainda).
    ultimo_evento_ms: AtomicU64,
    /// Os últimos eventos validados, só em debug.
    eventos_debug: Mutex<VecDeque<Value>>,
    /// O que o cérebro publicou (sessões, última reação, turnos).
    cerebro: RwLock<Value>,
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
            eventos_aceitos: AtomicU64::new(0),
            eventos_recusados: AtomicU64::new(0),
            ultimo_evento_ms: AtomicU64::new(0),
            eventos_debug: Mutex::new(VecDeque::new()),
            cerebro: RwLock::new(Value::Null),
        }
    }

    /// Publicado pelo laço principal depois de cada evento e prazo.
    pub fn publicar_cerebro(&self, resumo: &Resumo) {
        let valor = serde_json::to_value(resumo).unwrap_or(Value::Null);
        *self.cerebro.write().expect("lock do cérebro envenenado") = valor;
    }

    /// Um evento de hook aceito. Em debug, `validado` (só os campos que
    /// passaram na validação) vai para o `/v1/debug/eventos`.
    pub fn evento_aceito(&self, validado: impl FnOnce() -> Value) {
        self.eventos_aceitos.fetch_add(1, Ordering::Relaxed);
        let agora = self.inicio.elapsed().as_millis() as u64;
        self.ultimo_evento_ms.store(agora + 1, Ordering::Relaxed);
        if self.debug {
            let mut fila = self
                .eventos_debug
                .lock()
                .expect("lock dos eventos envenenado");
            if fila.len() == EVENTOS_DEBUG {
                fila.pop_front();
            }
            fila.push_back(validado());
        }
    }

    pub fn evento_recusado(&self) {
        self.eventos_recusados.fetch_add(1, Ordering::Relaxed);
    }

    /// Os últimos eventos validados (vazio sem debug), do mais velho ao
    /// mais novo.
    pub fn eventos_debug_json(&self) -> Value {
        let fila = self
            .eventos_debug
            .lock()
            .expect("lock dos eventos envenenado");
        json!({ "eventos": fila.iter().collect::<Vec<_>>() })
    }

    fn eventos_json(&self) -> Value {
        let ultimo = self.ultimo_evento_ms.load(Ordering::Relaxed);
        let agora = self.inicio.elapsed().as_millis() as u64;
        json!({
            "aceitos": self.eventos_aceitos.load(Ordering::Relaxed),
            "recusados": self.eventos_recusados.load(Ordering::Relaxed),
            "ultimo_ha_ms": (ultimo > 0).then(|| agora.saturating_sub(ultimo - 1)),
        })
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

    /// Config relida (a cada aprovação; decisão 0029).
    pub fn definir_config(&self, config: ConfigEfetiva) {
        *self.config.write().expect("lock da config envenenado") = config;
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
            "fonte": crate::FONTE,
            "tela": self.tela().nome(),
            "batimento_ms": self.idade_batimento().as_millis() as u64,
        })
    }

    /// Fotografia só com metadados, para `bin/pet estado`.
    pub fn estado_json(&self) -> Value {
        let config = self.config.read().expect("lock da config envenenado");
        let painel = self.painel.read().expect("lock do painel envenenado");
        let skin = self.skin.read().expect("lock da skin envenenado");
        let cerebro = self.cerebro.read().expect("lock do cérebro envenenado");
        let do_cerebro = |chave: &str, vazio: Value| cerebro.get(chave).cloned().unwrap_or(vazio);
        json!({
            "versao": pet_core::VERSAO,
            "fonte": crate::FONTE,
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
            "reacao": painel.reacao,
            "arrastando": painel.arrastando,
            "viagem": painel.viagem,
            "desktop": painel.desktop,
            "eventos": self.eventos_json(),
            "sessoes": do_cerebro("sessoes", json!([])),
            "ultima_reacao": do_cerebro("ultima_reacao", Value::Null),
            "turnos": do_cerebro("turnos", json!([])),
            "cerebro": {
                "origens": do_cerebro("origens", json!([])),
                "ignorados": do_cerebro("ignorados", json!({})),
            },
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
        use pet_core::geometria::Ret;
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
            ..InfoSkin::default()
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
    fn eventos_contados_e_guardados_so_em_debug() {
        let c = novo();
        assert!(c.estado_json()["eventos"]["ultimo_ha_ms"].is_null());
        c.evento_aceito(|| json!({"e": "Stop"}));
        c.evento_recusado();
        let estado = c.estado_json();
        assert_eq!(estado["eventos"]["aceitos"], 1);
        assert_eq!(estado["eventos"]["recusados"], 1);
        assert!(estado["eventos"]["ultimo_ha_ms"].is_u64());
        assert_eq!(c.eventos_debug_json()["eventos"], json!([]), "sem debug");

        let d = Compartilhado::novo(ConfigEfetiva::carregar(None, |_| None), true);
        for i in 0..EVENTOS_DEBUG + 5 {
            d.evento_aceito(|| json!({ "n": i }));
        }
        let lista = d.eventos_debug_json();
        let eventos = lista["eventos"].as_array().unwrap();
        assert_eq!(eventos.len(), EVENTOS_DEBUG);
        assert_eq!(eventos[0]["n"], 5, "os mais velhos saem primeiro");
    }

    #[test]
    fn cerebro_aparece_no_estado() {
        use pet_core::cerebro::{Agora, Cerebro, ConfigCerebro};
        use pet_core::evento::Evento;
        let c = novo();
        let estado = c.estado_json();
        assert_eq!(estado["sessoes"], json!([]));
        assert!(estado["ultima_reacao"].is_null());
        let mut cerebro = Cerebro::novo(ConfigCerebro::default());
        let agora = Agora {
            parede_ms: 1_790_000_000_000,
            mono_ms: 0,
        };
        let ev = Evento {
            e: "Stop".into(),
            sid: Some("0123456789abcdef".into()),
            turno: Some("p1".into()),
            ent: Some("cli".into()),
            proj: Some("claude-pet".into()),
            ..Evento::default()
        };
        cerebro.receber(&ev, agora.parede_ms, agora);
        let depois = Agora {
            parede_ms: agora.parede_ms + 900,
            mono_ms: 900,
        };
        cerebro.tique(depois);
        c.publicar_cerebro(&cerebro.resumo());
        let estado = c.estado_json();
        assert_eq!(estado["sessoes"][0]["sid8"], "01234567");
        assert_eq!(estado["sessoes"][0]["proj"], "claude-pet");
        assert_eq!(estado["ultima_reacao"]["nome"], "nod");
        assert_eq!(estado["ultima_reacao"]["sid8"], "01234567");
        assert_eq!(estado["turnos"][0]["nivel"], "T0");
        assert_eq!(estado["cerebro"]["origens"], json!(["cli"]));
        assert!(
            !estado.to_string().contains("0123456789abcdef"),
            "só o sid curto"
        );
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
