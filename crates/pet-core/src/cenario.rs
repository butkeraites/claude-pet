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
//! [`BASE_PAREDE`] + `t`, monotônico `t`. Entre um passo e outro, vence os
//! prazos como o laço do daemon (o do cérebro primeiro, depois os da janela),
//! com o compositor de mentira mostrando cada quadro na hora. Um prazo que
//! vence e continua armado é um erro (o laço do daemon giraria a 100% de CPU).

use std::fmt::Write as _;
use std::rc::Rc;

use serde_json::{Map, Value};

use crate::cerebro::{Agora, ConfigCerebro};
use crate::config::ConfigEfetiva;
use crate::evento::{self, Evento};
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
    Fim { t: u64 },
}

impl Passo {
    pub fn t(&self) -> u64 {
        match self {
            Passo::Evento { t, .. }
            | Passo::Desktop { t, .. }
            | Passo::Clique { t, .. }
            | Passo::Fim { t } => *t,
        }
    }
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

fn agora(t: u64) -> Agora {
    Agora {
        parede_ms: BASE_PAREDE + t,
        mono_ms: t,
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
    let tipos: Vec<&str> = ["evento", "desktop", "clique", "fim"]
        .into_iter()
        .filter(|k| objeto.contains_key(*k))
        .collect();
    if tipos.len() != 1 || objeto.len() != 2 {
        return Err("cada linha tem o t e só um de evento, desktop, clique ou fim".into());
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
        _ => Ok(Passo::Fim { t }),
    }
}

/// O Motor de um cenário rodando: a janela de mentira e o relógio.
pub struct Execucao {
    pub motor: Motor,
    pub janela: JanelaFalsa,
    /// Agora, em ms desde o começo.
    agora: u64,
    linha_do_tempo: Vec<Intencao>,
}

impl Execucao {
    /// O Motor com `config`, o personagem `skin` (os cenários dourados usam a
    /// `_teste`; sem skin, o cérebro decide do mesmo jeito, e só as reações
    /// não animam) e a janela pronta no eDP-1 em `t = 0`.
    pub fn nova(config: ConfigCerebro, skin: Option<Rc<Skin>>) -> Execucao {
        let mut motor = Motor::novo(config);
        motor.gravar_todas_as_intencoes();
        motor.acertar_relogio(agora(0));
        motor.definir_skin(skin);
        let mut janela = JanelaFalsa::default();
        motor.conectou(0);
        motor.aplicar_visibilidade(&mut janela, 0);
        let mut execucao = Execucao {
            motor,
            janela,
            agora: 0,
            linha_do_tempo: Vec::new(),
        };
        execucao.assentar();
        execucao
    }

    fn agora(&self) -> Agora {
        agora(self.agora)
    }

    /// O compositor de mentira: mostra o quadro em voo, deixa pronta a janela
    /// recém-criada e termina de destruir a que está saindo.
    fn assentar(&mut self) {
        for _ in 0..8 {
            self.janela.mostrou();
            let t = self.agora;
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
        self.linha_do_tempo
            .extend(self.motor.tirar_intencoes_novas());
    }

    /// Vence os prazos até `t`, como o laço do daemon.
    pub fn andar_ate(&mut self, t: u64) -> Result<(), String> {
        let mut repeticoes = 0;
        let mut ultimo = None;
        while let Some(prazo) = self.motor.proximo_prazo().filter(|p| *p <= t) {
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
            self.motor.acertar_relogio(ag);
            if self.motor.prazo_do_cerebro().is_some_and(|p| p <= quando) {
                let reacoes = self.motor.tique(ag);
                for r in reacoes {
                    self.motor.reagir(Some(&mut self.janela), r.nome, quando);
                }
            }
            self.motor.vencer(&mut self.janela, quando);
            self.assentar();
        }
        self.agora = self.agora.max(t);
        Ok(())
    }

    /// Aplica um passo no instante dele (vencendo os prazos antes).
    pub fn passo(&mut self, passo: &Passo) -> Result<(), String> {
        self.andar_ate(passo.t())?;
        let t = self.agora;
        let ag = self.agora();
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
            Passo::Fim { .. } => {}
        }
        self.assentar();
        Ok(())
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
