//! Cenários (decisão 0077): uma sequência de eventos do Claude Code e do
//! desktop, com o tempo relativo, e a linha do tempo das intenções que o
//! Motor decide para ela.
//!
//! **Formato** (`cenarios/<nome>.jsonl`): uma linha JSON por passo, com `t`
//! (ms desde o começo do cenário):
//!
//! - `{"t": 1200, "evento": {…}}`: um evento do fio v1, como o hook manda
//!   (`v` é opcional; o `ts`, se houver, também é relativo);
//! - `{"t": 5000, "desktop": {"ocioso": true}}`: um evento do desktop
//!   (`ocioso`, `olhando_claude`, `protetor`, `compartilhando`, `ligado`,
//!   `janela_ativa` com o id da janela ou `null`, `monitor`);
//! - `{"t": 9000, "clique": "esquerdo"}` (ou `"direito"`);
//! - `{"t": 90000, "reinicio": {"parado_ms": 20000}}`: o pet para (grava a
//!   memória das sessões, como no SIGTERM; decisão 0093), fica fora do ar
//!   por `parado_ms` (um evento nesse tempo se perde, como o hook que não
//!   acha o pet) e volta num Motor novo, com o relógio do laço do zero e a
//!   parede adiante, que restaura a memória antes de achar o compositor.
//!   Opções: `"maquina": true` (a máquina reiniciou: outro boot id),
//!   `"compositor": true` (outra instância do compositor: as janelas de
//!   antes não existem mais) e `"arquivo": "corrompido"`;
//! - `{"t": 60000, "fim": true}`: o relógio anda até aqui e o cenário acaba.
//!
//! Linhas vazias e as que começam por `#` são comentários. A primeira linha
//! pode ser o cabeçalho: `{"cenario": "<nome>", "descricao": "…", "config":
//! {"celebracao.modo": "discreta"}, "padrao": {"ent": "cli", "sid": "s1"}}`
//! (o `config` são chaves do `bichinho.toml`; o `padrao`, campos que todo
//! evento leva se não tiver os seus).
//!
//! **O executor** roda o Motor com a [`JanelaFalsa`] pronta num monitor de
//! 1920x1200 (o eDP-1 do Renan), num relógio falso: parede
//! [`BASE_PAREDE`] + `t`, monotônico `t` (depois de um reinício, `t` menos o
//! instante em que o pet voltou; a linha do tempo continua em `t`). Entre um
//! passo e outro, vence os prazos como o laço do daemon (o do cérebro
//! primeiro, depois os da janela), com o compositor de mentira mostrando
//! cada quadro na hora. Um prazo que vence e continua armado é um erro (o
//! laço do daemon giraria a 100% de CPU).

use std::fmt::Write as _;
use std::rc::Rc;

use serde_json::{Map, Value};

use crate::cerebro::{Agora, ConfigCerebro};
use crate::config::ConfigEfetiva;
use crate::evento::{self, Evento};
use crate::memoria;
use crate::motor::Motor;
use crate::motor::intencoes::Intencao;
use crate::plataforma::falsa::JanelaFalsa;
use crate::plataforma::{Alca, Botao, EventoDesktop, EventoOverlay, Fase, Monitor};
use crate::skin::Skin;

/// Hora de parede do instante 0 de todo cenário (2026-09-21, a mesma dos
/// testes do cérebro).
pub const BASE_PAREDE: u64 = 1_790_000_000_000;
/// A janela da proteção de tela nos cenários.
const JANELA_DO_PROTETOR: &str = "protetor";
/// Quantas vezes seguidas um prazo pode vencer no mesmo instante antes de o
/// executor desistir (um prazo que não anda).
const MESMO_INSTANTE_MAX: u32 = 64;
/// O boot id da máquina nos cenários; o de depois de um `"maquina": true`.
const BOOT: &str = "boot-1";
const OUTRO_BOOT: &str = "boot-2";
/// A instância do compositor nos cenários; a de depois de um
/// `"compositor": true`.
const COMPOSITOR: &str = "hyprland-1";
const OUTRO_COMPOSITOR: &str = "hyprland-2";

/// Um cenário lido.
#[derive(Debug, Clone)]
pub struct Cenario {
    pub nome: String,
    pub descricao: Option<String>,
    pub config: ConfigCerebro,
    pub passos: Vec<Passo>,
}

/// Um passo do cenário, no instante `t` (ms desde o começo).
#[derive(Debug, Clone, PartialEq)]
pub enum Passo {
    Evento { t: u64, evento: Box<Evento> },
    Desktop { t: u64, evento: Desktop },
    Clique { t: u64, botao: Botao },
    Reinicio { t: u64, reinicio: Reinicio },
    Fim { t: u64 },
}

impl Passo {
    pub fn t(&self) -> u64 {
        match self {
            Passo::Evento { t, .. }
            | Passo::Desktop { t, .. }
            | Passo::Clique { t, .. }
            | Passo::Reinicio { t, .. }
            | Passo::Fim { t } => *t,
        }
    }
}

/// O pet reinicia no meio do cenário (decisão 0093).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reinicio {
    /// Quanto tempo o pet fica fora do ar.
    pub parado_ms: u64,
    /// A máquina reiniciou (outro boot id).
    pub maquina: bool,
    /// Outra instância do compositor (um logout e um login).
    pub compositor: bool,
    /// A memória gravada chega corrompida.
    pub corrompido: bool,
}

/// Um evento do desktop num cenário, antes de virar o
/// [`EventoDesktop`] (que precisa da hora).
#[derive(Debug, Clone, PartialEq)]
pub enum Desktop {
    Ocioso(bool),
    OlhandoClaude(bool),
    Protetor(bool),
    Compartilhando(bool),
    Ligado(bool),
    JanelaAtiva(Option<String>),
    Monitor(String),
}

/// O monitor dos cenários: o eDP-1 do Renan.
pub fn monitor() -> Monitor {
    Monitor {
        nome: Some("eDP-1".into()),
        logico: (1280, 800),
        escala: 1.5,
        ..Monitor::default()
    }
}

/// Lê um cenário. `nome` é o do arquivo (sem `.jsonl`); o do cabeçalho, se
/// houver, tem de ser o mesmo.
pub fn ler(nome: &str, texto: &str) -> Result<Cenario, String> {
    let mut cenario = Cenario {
        nome: nome.to_owned(),
        descricao: None,
        config: ConfigCerebro::default(),
        passos: Vec::new(),
    };
    let mut padrao = Map::new();
    let mut ultimo_t = 0;
    let mut primeira = true;
    for (i, linha) in texto.lines().enumerate() {
        let n = i + 1;
        let linha = linha.trim();
        if linha.is_empty() || linha.starts_with('#') {
            continue;
        }
        let valor: Value =
            serde_json::from_str(linha).map_err(|e| format!("linha {n}: não é JSON ({e})"))?;
        let Some(objeto) = valor.as_object() else {
            return Err(format!("linha {n}: esperava um objeto"));
        };
        if objeto.contains_key("cenario") {
            if !primeira {
                return Err(format!("linha {n}: o cabeçalho é a primeira linha"));
            }
            primeira = false;
            cabecalho(objeto, &mut cenario, &mut padrao).map_err(|e| format!("linha {n}: {e}"))?;
            continue;
        }
        primeira = false;
        let passo = passo(objeto, &padrao).map_err(|e| format!("linha {n}: {e}"))?;
        if passo.t() < ultimo_t {
            return Err(format!(
                "linha {n}: t = {} volta no tempo (antes: {ultimo_t})",
                passo.t()
            ));
        }
        ultimo_t = passo.t();
        cenario.passos.push(passo);
    }
    Ok(cenario)
}

fn cabecalho(
    objeto: &Map<String, Value>,
    cenario: &mut Cenario,
    padrao: &mut Map<String, Value>,
) -> Result<(), String> {
    for (chave, valor) in objeto {
        match chave.as_str() {
            "cenario" => {
                let nome = valor.as_str().ok_or("cenario: esperava texto")?;
                if nome != cenario.nome {
                    return Err(format!(
                        "o cabeçalho diz «{nome}», o arquivo é «{}»",
                        cenario.nome
                    ));
                }
            }
            "descricao" => {
                cenario.descricao = Some(valor.as_str().ok_or("descricao: texto")?.to_owned());
            }
            "padrao" => {
                *padrao = valor
                    .as_object()
                    .ok_or("padrao: esperava um objeto")?
                    .clone();
            }
            "config" => {
                let config = valor.as_object().ok_or("config: esperava um objeto")?;
                cenario.config = config_do_cenario(config)?;
            }
            outra => return Err(format!("chave desconhecida no cabeçalho: {outra}")),
        }
    }
    Ok(())
}

/// As chaves do `bichinho.toml` do cabeçalho, pela mesma validação do
/// daemon: um aviso (chave errada, valor fora da faixa) é erro aqui.
fn config_do_cenario(chaves: &Map<String, Value>) -> Result<ConfigCerebro, String> {
    let mut secoes: std::collections::BTreeMap<&str, Vec<(&str, &Value)>> = Default::default();
    for (caminho, valor) in chaves {
        let (secao, chave) = caminho
            .split_once('.')
            .ok_or_else(|| format!("config: «{caminho}» não é secao.chave"))?;
        secoes.entry(secao).or_default().push((chave, valor));
    }
    let mut toml = String::new();
    for (secao, itens) in secoes {
        let _ = writeln!(toml, "[{secao}]");
        for (chave, valor) in itens {
            // Texto, número e lista de textos: o JSON deles é TOML válido.
            let _ = writeln!(toml, "{chave} = {valor}");
        }
    }
    let efetiva = ConfigEfetiva::carregar(Some(&toml), |_| None);
    if !efetiva.avisos.is_empty() {
        return Err(format!("config: {}", efetiva.avisos.join("; ")));
    }
    Ok(ConfigCerebro::de(&efetiva.config()))
}

fn tempo(objeto: &Map<String, Value>) -> Result<u64, String> {
    objeto
        .get("t")
        .and_then(Value::as_u64)
        .ok_or_else(|| "falta o t (ms, inteiro)".to_owned())
}

fn passo(objeto: &Map<String, Value>, padrao: &Map<String, Value>) -> Result<Passo, String> {
    let t = tempo(objeto)?;
    let tipos: Vec<&str> = ["evento", "desktop", "clique", "reinicio", "fim"]
        .into_iter()
        .filter(|k| objeto.contains_key(*k))
        .collect();
    if tipos.len() != 1 || objeto.len() != 2 {
        return Err(
            "cada linha tem o t e só um de evento, desktop, clique, reinicio ou fim".into(),
        );
    }
    match tipos[0] {
        "evento" => {
            let mut corpo = padrao.clone();
            let campos = objeto["evento"]
                .as_object()
                .ok_or("evento: esperava um objeto")?;
            for (k, v) in campos {
                corpo.insert(k.clone(), v.clone());
            }
            corpo.entry("v").or_insert(Value::from(1));
            if let Some(ts) = corpo.get("ts").and_then(Value::as_u64) {
                corpo.insert("ts".into(), Value::from(BASE_PAREDE + ts));
            }
            let texto = Value::Object(corpo).to_string();
            let lido = evento::ler(texto.as_bytes()).map_err(|e| format!("evento: {e}"))?;
            if !lido.descartados.is_empty() {
                return Err(format!(
                    "evento: campos que o pet descartaria: {}",
                    lido.descartados.join(", ")
                ));
            }
            Ok(Passo::Evento {
                t,
                evento: Box::new(lido.evento),
            })
        }
        "desktop" => {
            let d = objeto["desktop"]
                .as_object()
                .ok_or("desktop: esperava um objeto")?;
            if d.len() != 1 {
                return Err("desktop: uma coisa por linha".into());
            }
            let (chave, valor) = d.iter().next().expect("um item");
            let booleano = || {
                valor
                    .as_bool()
                    .ok_or_else(|| format!("desktop.{chave}: esperava true ou false"))
            };
            let evento = match chave.as_str() {
                "ocioso" => Desktop::Ocioso(booleano()?),
                "olhando_claude" => Desktop::OlhandoClaude(booleano()?),
                "protetor" => Desktop::Protetor(booleano()?),
                "compartilhando" => Desktop::Compartilhando(booleano()?),
                "ligado" => Desktop::Ligado(booleano()?),
                "janela_ativa" => Desktop::JanelaAtiva(match valor {
                    Value::Null => None,
                    Value::String(janela) => Some(janela.clone()),
                    _ => return Err("desktop.janela_ativa: um id ou null".into()),
                }),
                "monitor" => Desktop::Monitor(
                    valor
                        .as_str()
                        .ok_or("desktop.monitor: esperava texto")?
                        .to_owned(),
                ),
                outro => return Err(format!("desktop: não conheço «{outro}»")),
            };
            Ok(Passo::Desktop { t, evento })
        }
        "clique" => {
            let botao = match objeto["clique"].as_str() {
                Some("esquerdo") => Botao::Esquerdo,
                Some("direito") => Botao::Direito,
                _ => return Err("clique: esquerdo ou direito".into()),
            };
            Ok(Passo::Clique { t, botao })
        }
        "reinicio" => {
            let r = objeto["reinicio"]
                .as_object()
                .ok_or("reinicio: esperava um objeto")?;
            let mut reinicio = Reinicio {
                parado_ms: 0,
                maquina: false,
                compositor: false,
                corrompido: false,
            };
            let mut parado = None;
            for (chave, valor) in r {
                let booleano = || {
                    valor
                        .as_bool()
                        .ok_or_else(|| format!("reinicio.{chave}: esperava true ou false"))
                };
                match chave.as_str() {
                    "parado_ms" => {
                        parado = Some(valor.as_u64().ok_or("reinicio.parado_ms: inteiro")?);
                    }
                    "maquina" => reinicio.maquina = booleano()?,
                    "compositor" => reinicio.compositor = booleano()?,
                    "arquivo" => match valor.as_str() {
                        Some("corrompido") => reinicio.corrompido = true,
                        _ => return Err("reinicio.arquivo: só \"corrompido\"".into()),
                    },
                    outro => return Err(format!("reinicio: não conheço «{outro}»")),
                }
            }
            reinicio.parado_ms = parado.ok_or("reinicio: falta o parado_ms")?;
            Ok(Passo::Reinicio { t, reinicio })
        }
        _ => Ok(Passo::Fim { t }),
    }
}

/// O Motor de um cenário rodando: a janela de mentira e o relógio.
pub struct Execucao {
    pub motor: Motor,
    pub janela: JanelaFalsa,
    /// Agora, em ms desde o começo.
    agora: u64,
    /// O instante do cenário em que o relógio do laço do Motor de agora
    /// estava em 0 (muda a cada reinício).
    origem: u64,
    /// O pet esteve fora do ar até aqui (o último reinício): um evento antes
    /// disso se perdeu.
    fora_ate: u64,
    /// O boot id da máquina e a instância do compositor de agora.
    boot: &'static str,
    compositor: &'static str,
    config: ConfigCerebro,
    linha_do_tempo: Vec<Intencao>,
}

impl Execucao {
    /// O Motor com `config`, o personagem `skin` (os cenários dourados usam a
    /// `_teste`; sem skin, o cérebro decide do mesmo jeito, e só as reações
    /// não animam) e a janela pronta no eDP-1 em `t = 0`.
    pub fn nova(config: ConfigCerebro, skin: Option<Rc<Skin>>) -> Execucao {
        let janela = JanelaFalsa::default();
        let mut execucao = Execucao {
            motor: Motor::novo(config.clone()),
            janela,
            agora: 0,
            origem: 0,
            fora_ate: 0,
            boot: BOOT,
            compositor: COMPOSITOR,
            config,
            linha_do_tempo: Vec::new(),
        };
        execucao.partir(skin, None);
        execucao
    }

    /// Um Motor novo agora, com o relógio do laço do zero: restaura a
    /// `memoria` gravada (se houver), acha o compositor e conecta, como o
    /// daemon na partida (a memória vem antes do compositor; decisão 0093).
    fn partir(&mut self, skin: Option<Rc<Skin>>, memoria: Option<&str>) {
        self.origem = self.agora;
        let ag = self.agora();
        let mut motor = Motor::novo(self.config.clone());
        motor.gravar_todas_as_intencoes();
        motor.acertar_relogio(ag);
        motor.definir_skin(skin);
        if let Some(texto) = memoria {
            match memoria::ler(texto) {
                Ok(lida) => {
                    let _ = motor.restaurar(&lida, Some(self.boot), ag);
                }
                Err(recusa) => motor.recusar_memoria(recusa, 0),
            }
        }
        motor.definir_compositor(Some(self.compositor.to_owned()));
        motor.conectou(0);
        motor.aplicar_visibilidade(&mut self.janela, 0);
        self.motor = motor;
        self.assentar();
    }

    fn agora(&self) -> Agora {
        Agora {
            parede_ms: BASE_PAREDE + self.agora,
            mono_ms: self.agora - self.origem,
        }
    }

    /// O instante `t` do cenário no relógio do laço do Motor de agora.
    fn no_laco(&self, t: u64) -> u64 {
        t.saturating_sub(self.origem)
    }

    /// O compositor de mentira: mostra o quadro em voo, deixa pronta a janela
    /// recém-criada e termina de destruir a que está saindo.
    fn assentar(&mut self) {
        for _ in 0..8 {
            self.janela.mostrou();
            let t = self.no_laco(self.agora);
            match self.janela.fase_atual() {
                Fase::Saindo => {
                    self.janela.fase = None;
                    self.motor
                        .evento_overlay(&mut self.janela, EventoOverlay::Saiu, t);
                }
                Fase::Viva { .. }
                    if self.janela.pedidos.last().map(String::as_str) == Some("criar") =>
                {
                    self.janela.pronta = Some(monitor());
                    self.janela.pedidos.push("pronta".into());
                    self.motor
                        .evento_overlay(&mut self.janela, EventoOverlay::Pronta, t);
                }
                _ => break,
            }
        }
        let origem = self.origem;
        self.linha_do_tempo
            .extend(self.motor.tirar_intencoes_novas().into_iter().map(|mut i| {
                i.t_ms += origem;
                i
            }));
    }

    /// Vence os prazos até `t`, como o laço do daemon.
    pub fn andar_ate(&mut self, t: u64) -> Result<(), String> {
        let mut repeticoes = 0;
        let mut ultimo = None;
        while let Some(prazo) = self
            .motor
            .proximo_prazo()
            .map(|p| p + self.origem)
            .filter(|p| *p <= t)
        {
            let quando = prazo.max(self.agora);
            if ultimo == Some(quando) {
                repeticoes += 1;
                if repeticoes > MESMO_INSTANTE_MAX {
                    return Err(format!("um prazo venceu em {quando} ms e continua armado"));
                }
            } else {
                repeticoes = 0;
                ultimo = Some(quando);
            }
            self.agora = quando;
            let ag = self.agora();
            let no_laco = ag.mono_ms;
            self.motor.acertar_relogio(ag);
            if self.motor.prazo_do_cerebro().is_some_and(|p| p <= no_laco) {
                let reacoes = self.motor.tique(ag);
                for r in reacoes {
                    self.motor.reagir(Some(&mut self.janela), r.nome, no_laco);
                }
            }
            self.motor.vencer(&mut self.janela, no_laco);
            self.assentar();
        }
        self.agora = self.agora.max(t);
        Ok(())
    }

    /// Aplica um passo no instante dele (vencendo os prazos antes).
    pub fn passo(&mut self, passo: &Passo) -> Result<(), String> {
        if passo.t() < self.fora_ate {
            // O pet estava fora do ar: o hook não o achou, e o evento se
            // perdeu. O resto (o desktop, o clique) não cabe aí.
            return match passo {
                Passo::Evento { .. } => Ok(()),
                _ => Err(format!(
                    "t = {}: o pet está fora do ar até {} (só um evento se perde aí)",
                    passo.t(),
                    self.fora_ate
                )),
            };
        }
        self.andar_ate(passo.t())?;
        let ag = self.agora();
        let t = ag.mono_ms;
        match passo {
            Passo::Evento { evento, .. } => {
                let reacoes = self.motor.evento(evento, ag.parede_ms, ag);
                for r in reacoes {
                    self.motor.reagir(Some(&mut self.janela), r.nome, t);
                }
            }
            Passo::Desktop { evento, .. } => {
                let evento = match evento {
                    Desktop::Ocioso(v) => EventoDesktop::Ocioso(*v),
                    Desktop::OlhandoClaude(v) => EventoDesktop::OlhandoClaude(*v),
                    Desktop::Protetor(true) => EventoDesktop::JanelaAbriu {
                        janela: Alca(JANELA_DO_PROTETOR.into()),
                        protetor: true,
                    },
                    Desktop::Protetor(false) => {
                        EventoDesktop::JanelaFechou(Alca(JANELA_DO_PROTETOR.into()))
                    }
                    Desktop::Compartilhando(v) => EventoDesktop::Compartilhando(*v),
                    Desktop::Ligado(v) => EventoDesktop::Ligado(*v),
                    Desktop::JanelaAtiva(janela) => {
                        let janela = janela.clone().map(Alca);
                        if let Some(j) = &janela
                            && !self.janela.desktop.janelas.contains(j)
                        {
                            self.janela.desktop.janelas.push(j.clone());
                        }
                        self.janela.desktop.ativa = janela.clone();
                        EventoDesktop::JanelaAtiva {
                            janela,
                            parede_ms: ag.parede_ms,
                        }
                    }
                    Desktop::Monitor(nome) => EventoDesktop::MonitorEmFoco(nome.clone()),
                };
                self.motor
                    .evento_desktop(Some(&mut self.janela), &evento, ag);
            }
            Passo::Clique { botao, .. } => {
                self.motor.clicar(&mut self.janela, *botao, t);
            }
            Passo::Reinicio { reinicio, .. } => self.reiniciar(*reinicio),
            Passo::Fim { .. } => {}
        }
        self.assentar();
        Ok(())
    }

    /// O pet para (o SIGTERM grava a memória das sessões), fica fora do ar e
    /// volta num Motor novo, na mesma máquina e no mesmo compositor ou não
    /// (decisão 0093). A janela velha some com o processo; as janelas do
    /// compositor ficam, se ele é o mesmo.
    fn reiniciar(&mut self, r: Reinicio) {
        let texto = if r.corrompido {
            r#"{"versao": 1, "gravada_ms": 1, "sessoes": [{"sid": "#.to_owned()
        } else {
            self.motor
                .memoria(self.agora(), Some(self.boot.to_owned()))
                .texto()
        };
        let skin = self.motor.skin().cloned();
        self.agora += r.parado_ms;
        self.fora_ate = self.agora;
        if r.maquina {
            self.boot = if self.boot == BOOT { OUTRO_BOOT } else { BOOT };
        }
        let mut desktop = std::mem::take(&mut self.janela.desktop);
        desktop.eventos.clear();
        if r.compositor || r.maquina {
            self.compositor = if self.compositor == COMPOSITOR {
                OUTRO_COMPOSITOR
            } else {
                COMPOSITOR
            };
            desktop.janelas.clear();
            desktop.ativa = None;
        }
        self.janela = JanelaFalsa {
            desktop,
            ..JanelaFalsa::default()
        };
        self.partir(skin, Some(&texto));
    }

    /// A linha do tempo até agora.
    pub fn linha_do_tempo(&self) -> &[Intencao] {
        &self.linha_do_tempo
    }
}

/// Roda o cenário inteiro e devolve a linha do tempo das intenções.
pub fn rodar(cenario: &Cenario, skin: Option<Rc<Skin>>) -> Result<Vec<Intencao>, String> {
    let mut execucao = Execucao::nova(cenario.config.clone(), skin);
    for passo in &cenario.passos {
        execucao
            .passo(passo)
            .map_err(|e| format!("{}: {e}", cenario.nome))?;
    }
    Ok(execucao.linha_do_tempo)
}

/// Os campos do fio v1 que um cenário gravado leva (os outros, como `term`,
/// `v`, `recebido_ms` e `descartados`, ficam de fora).
const CAMPOS_GRAVADOS: [&str; 24] = [
    "e", "ts", "sid", "turno", "agente", "aid", "tool", "nt", "err", "src", "orig", "reason",
    "intr", "sha", "bg", "bgt", "bgi", "crn", "dur", "arq", "proj", "ent", "dnd", "teste",
];

/// Os pseudônimos de um cenário gravado: cada id de verdade vira o próximo
/// da sua família, sempre o mesmo para o mesmo id.
#[derive(Default)]
struct Pseudonimos {
    mapas: std::collections::BTreeMap<&'static str, Vec<String>>,
}

impl Pseudonimos {
    fn de(&mut self, familia: &'static str, real: &str) -> usize {
        let lista = self.mapas.entry(familia).or_default();
        match lista.iter().position(|r| r == real) {
            Some(i) => i + 1,
            None => {
                lista.push(real.to_owned());
                lista.len()
            }
        }
    }
}

/// A letra de um número (1 → a, 26 → z, 27 → aa).
fn letras(mut n: usize) -> String {
    let mut s = Vec::new();
    while n > 0 {
        n -= 1;
        s.push(b'a' + (n % 26) as u8);
        n /= 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// Um cenário a partir do JSON do `/v1/debug/eventos` (decisão 0078): os
/// eventos na ordem em que chegaram, com o tempo relativo ao primeiro
/// instante (a chegada `t` e o `ts`), e só metadados com pseudônimos: as
/// sessões viram `s1`, os turnos `p1`, os agentes e as tarefas em segundo
/// plano `a1` (o mesmo pseudônimo quando o `aid` e o `bgi` são o mesmo id),
/// as pastas `projeto-a`, os hashes de arquivo `000000000001` e as
/// ferramentas MCP `mcp__servidor_a__ferramenta_1`; sem o `term`, os
/// descartados e a hora de verdade. Devolve o texto do `.jsonl`, conferido
/// pelo mesmo leitor dos cenários.
pub fn de_eventos(nome: &str, json: &str) -> Result<String, String> {
    let valor: Value = serde_json::from_str(json)
        .map_err(|e| format!("não é o JSON do /v1/debug/eventos ({e})"))?;
    let eventos = valor
        .get("eventos")
        .and_then(Value::as_array)
        .ok_or("sem a lista «eventos» (é a saída do /v1/debug/eventos?)")?;
    if eventos.is_empty() {
        return Err("nenhum evento (o pet de debug recebeu algum?)".into());
    }
    let numero = |e: &Value, k: &str| e.get(k).and_then(Value::as_u64);
    // O zero do cenário: o instante mais cedo (a chegada ou o `ts`).
    let zero = eventos
        .iter()
        .flat_map(|e| [numero(e, "recebido_ms"), numero(e, "ts")])
        .flatten()
        .min()
        .ok_or("nenhum evento com a hora de chegada")?;
    let mut nomes = Pseudonimos::default();
    let mut texto = String::new();
    let cabecalho = serde_json::json!({
        "cenario": nome,
        "descricao": "gravado do /v1/debug/eventos, com pseudônimos (decisão 0078)",
    });
    let _ = writeln!(texto, "{cabecalho}");
    let mut ultimo_t = 0;
    for (i, e) in eventos.iter().enumerate() {
        let objeto = e
            .as_object()
            .ok_or_else(|| format!("evento {}: esperava um objeto", i + 1))?;
        let t = numero(e, "recebido_ms")
            .ok_or_else(|| format!("evento {}: sem recebido_ms", i + 1))?
            .saturating_sub(zero)
            .max(ultimo_t);
        ultimo_t = t;
        let mut saida = Map::new();
        for campo in CAMPOS_GRAVADOS {
            let Some(v) = objeto.get(campo) else {
                continue;
            };
            let texto_de = |v: &Value| v.as_str().map(str::to_owned);
            let novo = match campo {
                "ts" => v.as_u64().map(|ts| Value::from(ts.saturating_sub(zero))),
                "sid" => texto_de(v).map(|r| Value::from(format!("s{}", nomes.de("sid", &r)))),
                "turno" => texto_de(v).map(|r| Value::from(format!("p{}", nomes.de("turno", &r)))),
                "aid" => texto_de(v).map(|r| Value::from(format!("a{}", nomes.de("agente", &r)))),
                "bgi" => v.as_array().map(|ids| {
                    Value::from(
                        ids.iter()
                            .filter_map(Value::as_str)
                            .map(|r| format!("a{}", nomes.de("agente", r)))
                            .collect::<Vec<_>>(),
                    )
                }),
                "proj" => texto_de(v)
                    .map(|r| Value::from(format!("projeto-{}", letras(nomes.de("proj", &r))))),
                "arq" => texto_de(v).map(|r| Value::from(format!("{:012x}", nomes.de("arq", &r)))),
                "tool" => texto_de(v).map(|r| match r.strip_prefix("mcp__") {
                    Some(resto) => {
                        let servidor = resto.split("__").next().unwrap_or_default().to_owned();
                        let s = letras(nomes.de("mcp", &servidor));
                        let f = nomes.de("mcp_ferramenta", &r);
                        Value::from(format!("mcp__servidor_{s}__ferramenta_{f}"))
                    }
                    None => Value::from(r),
                }),
                _ => Some(v.clone()),
            };
            if let Some(novo) = novo {
                saida.insert(campo.to_owned(), novo);
            }
        }
        let _ = writeln!(texto, r#"{{"t":{t},"evento":{}}}"#, Value::Object(saida));
    }
    let _ = writeln!(texto, r#"{{"t":{},"fim":true}}"#, ultimo_t + 10_000);
    // O mesmo leitor dos cenários: um campo que o pet descartaria é erro.
    ler(nome, &texto)?;
    Ok(texto)
}

/// Uma intenção por linha, em JSON (o formato do `.esperado.jsonl`).
pub fn linhas(intencoes: &[Intencao]) -> String {
    let mut texto = String::new();
    for i in intencoes {
        texto.push_str(&serde_json::to_string(i).unwrap_or_default());
        texto.push('\n');
    }
    texto
}

#[cfg(test)]
mod testes;
