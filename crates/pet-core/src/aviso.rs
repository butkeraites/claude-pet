//! O aviso que o hook manda ao pet: a lista branca do `bichinho avisar`
//! (decisões 0009, 0031 e 0041).
//!
//! É a mesma lista do `plugin/scripts/avisar.sh`, escrita em Rust e com os
//! validadores do próprio fio v1 ([`crate::evento`]): o que o pet
//! descartaria nem sai do host. Só METADADOS saem: nome do evento, ids
//! opacos, nome da ferramenta, enums, contagens, durações, o hash do caminho
//! editado e o nome da pasta do projeto. Prompt, código, resposta, texto de
//! erro, títulos e caminhos nunca saem: nenhum outro campo do hook é lido.
//!
//! Tudo aqui é puro (recebe o JSON do hook e o ambiente já lido); quem lê a
//! entrada padrão, o arquivo do "não perturbe" e faz o POST é o subcomando
//! `avisar` do binário.
//!
//! **Leitura** (decisão 0045): o JSON do hook é lido em fluxo, e só os
//! campos da lista branca são guardados, como texto cru, e interpretados um
//! a um ([`Lido`]). O resto (o prompt, a resposta, a saída das ferramentas)
//! é pulado sem ser interpretado: um texto com um caractere quebrado (um
//! substituto UTF-16 sozinho, bytes que não são UTF-8), um aninhamento
//! fundo ou um número enorme num campo que o hook não lê não derruba os
//! metadados, e a memória não cresce com o tamanho deles. Um campo da lista
//! que não se lê cai sozinho, como um de tipo errado.
//!
//! **A forma do prompt e os agendamentos** (decisão 0072): do `prompt` só sai
//! um enum (`orig`), `notificacao` quando o texto começa por
//! `<task-notification>` (o Claude Code acordando a sessão com o aviso de uma
//! tarefa em segundo plano) e `comum` para qualquer outro texto. O prompt
//! nunca é guardado: a entrada deixa olhar os primeiros [`JANELA_DO_PROMPT`]
//! bytes que passam logo depois da chave `prompt`, que só são comparados com a
//! etiqueta e jogados fora, e o resto do texto é pulado como antes. Do
//! `session_crons` do Stop só sai a contagem (`crn`); nada de dentro dos
//! agendamentos é lido.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::io::{self, Read};
use std::rc::Rc;

use serde::Serialize;
use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor,
};
use serde_json::value::RawValue;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::aprovacao::hex;
use crate::evento::{
    MAX_APP, MAX_CONTAGEM, MAX_DURACAO_MS, MAX_FERRAMENTA, MAX_TAREFAS, ORIG_COMUM,
    ORIG_NOTIFICACAO, Terminal, eh_enum, eh_hash_de_arquivo, eh_id, eh_nome_de_evento,
    eh_numero_de_terminal, eh_origem, eh_painel_tmux, eh_projeto, eh_token,
};

/// O começo do prompt com que o Claude Code acorda a sessão quando uma tarefa
/// em segundo plano (um agente, um shell) termina (decisão 0071).
pub const ETIQUETA_DE_NOTIFICACAO: &str = "<task-notification>";
/// Quantos bytes da entrada são olhados logo depois da chave `prompt` (os
/// dois-pontos, as aspas, uns espaços e a etiqueta cabem com folga).
pub const JANELA_DO_PROMPT: usize = 64;

/// Porta do pet quando `PET_PORTA` falta ou não serve (decisão 0008).
pub const PORTA_PADRAO: u16 = 27380;

/// Ferramentas que editam arquivo: só delas sai o hash do caminho.
pub const FERRAMENTAS_DE_EDICAO: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

/// Os tipos de tarefa em segundo plano do Claude Code 2.1.288, normalizados
/// para `[a-z_]` (decisão 0031). Qualquer outro vira `outro`, nunca o texto
/// dele.
pub const TIPOS_DE_TAREFA: [&str; 10] = [
    "shell",
    "subagent",
    "workflow",
    "monitor",
    "mcp_task",
    "teammate",
    "dream",
    "auto_mode_scan",
    "memory_import",
    "cloud_session",
];

/// O que o hook sabe fora do JSON: o evento (o argumento), a hora, a origem
/// (`CLAUDE_CODE_ENTRYPOINT`), o "não perturbe", se é teste (`PET_TESTE=1`)
/// e os ids de terminal do ambiente (decisão 0054).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contexto<'a> {
    pub evento: &'a str,
    /// Relógio do host no início do hook, em ms desde 1970.
    pub ts: Option<u64>,
    pub ent: Option<&'a str>,
    pub dnd: bool,
    pub teste: bool,
    pub terminal: IdsDoAmbiente<'a>,
    /// O `__CFBundleIdentifier` do ambiente do hook (o app que hospeda a
    /// sessão no macOS; decisão 0105). Ausente fora do macOS.
    pub app: Option<&'a str>,
}

/// Os ids de terminal como o ambiente do hook os tem (`TMUX_PANE`,
/// `KITTY_WINDOW_ID`, `WEZTERM_PANE`), antes de validar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IdsDoAmbiente<'a> {
    pub tmux: Option<&'a str>,
    pub kitty: Option<&'a str>,
    pub wezterm: Option<&'a str>,
}

/// Os eventos que levam os ids de terminal: o começo da sessão e cada
/// prompt (é quando a janela do terminal da sessão é casada; decisão 0054).
pub const EVENTOS_COM_TERMINAL: [&str; 2] = ["SessionStart", "UserPromptSubmit"];

/// Os ids de terminal válidos do ambiente (cada um pelo validador do pet;
/// um que não passa fica de fora sozinho), só nos eventos que os levam.
pub fn terminal(evento: &str, ids: &IdsDoAmbiente) -> Option<Terminal> {
    if !EVENTOS_COM_TERMINAL.contains(&evento) {
        return None;
    }
    let t = Terminal {
        tmux: ids.tmux.filter(|v| eh_painel_tmux(v)).map(str::to_owned),
        kitty: ids
            .kitty
            .filter(|v| eh_numero_de_terminal(v))
            .map(str::to_owned),
        wezterm: ids
            .wezterm
            .filter(|v| eh_numero_de_terminal(v))
            .map(str::to_owned),
    };
    (!t.vazio()).then_some(t)
}

/// O app que hospeda a sessão (o `__CFBundleIdentifier` do macOS), só nos
/// eventos que casam a janela (os mesmos do terminal) e se passa pelo
/// validador de token do pet (decisão 0105). É a `Alca` da janela no macOS.
pub fn app(evento: &str, id: Option<&str>) -> Option<String> {
    if !EVENTOS_COM_TERMINAL.contains(&evento) {
        return None;
    }
    id.filter(|v| eh_token(v, MAX_APP)).map(str::to_owned)
}

/// O corpo do fio v1, na ordem do `avisar.sh`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Fio {
    pub v: u8,
    pub e: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turno: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agente: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// A forma do prompt (decisão 0072): `notificacao` ou `comum`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orig: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bgt: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bgi: Option<Vec<String>>,
    /// Quantos agendamentos (`session_crons`) o Stop listou (decisão 0072).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crn: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dur: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proj: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ent: Option<String>,
    pub dnd: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teste: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term: Option<Terminal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

/// O mínimo, quando a entrada não é um objeto JSON: `{"v":1,"e":"<Evento>"}`
/// (e `"teste":true` com `PET_TESTE=1`), byte a byte como o `avisar.sh`.
/// `evento` já passou por [`eh_nome_de_evento`] (só letras).
pub fn minimo(evento: &str, teste: bool) -> String {
    if teste {
        format!(r#"{{"v":1,"e":"{evento}","teste":true}}"#)
    } else {
        format!(r#"{{"v":1,"e":"{evento}"}}"#)
    }
}

/// O nome do evento que o hook recebeu como argumento serve?
pub fn evento_valido(evento: &str) -> bool {
    eh_nome_de_evento(evento)
}

/// `PET_PORTA`: só dígitos, até 5, e uma porta de verdade; senão a padrão.
pub fn porta(valor: Option<&str>) -> u16 {
    valor
        .filter(|v| !v.is_empty() && v.len() <= 5 && v.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|&p| p > 0)
        .unwrap_or(PORTA_PADRAO)
}

/// O "não perturbe" do Omarchy: só `{"dnd": true}` liga; qualquer outra
/// coisa (outro tipo, JSON quebrado) desliga. Do arquivo só sai o booleano.
pub fn dnd_do_omarchy(conteudo: &[u8]) -> bool {
    serde_json::from_slice::<Value>(conteudo)
        .ok()
        .and_then(|v| v.get("dnd").cloned())
        == Some(Value::Bool(true))
}

/// Os 12 primeiros hexadecimais do sha256 do caminho editado: o pet conta
/// arquivos diferentes sem saber quais são.
pub fn hash_do_caminho(caminho: &str) -> String {
    hex(&Sha256::digest(caminho.as_bytes()))[..12].to_owned()
}

/// Os campos do JSON do hook que a lista branca lê. Nada mais é guardado.
const ESCALARES: [&str; 12] = [
    "session_id",
    "prompt_id",
    "agent_id",
    "tool_name",
    "notification_type",
    "error",
    "source",
    "reason",
    "is_interrupt",
    "stop_hook_active",
    "duration_ms",
    "cwd",
];

/// O que o hook leu do JSON: só os campos da lista branca, cada um já
/// interpretado (um que não se lê fica de fora). Nenhum outro campo do hook
/// chega aqui.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Lido {
    /// Os campos simples da lista ([`ESCALARES`]).
    campos: Map<String, Value>,
    /// O `tool_input`, só com o `file_path` e o `notebook_path`; `None` se
    /// não é um objeto.
    entrada_da_ferramenta: Option<Map<String, Value>>,
    /// O `background_tasks`, se é uma lista.
    tarefas: Option<Tarefas>,
    /// A forma do `prompt` ([`ORIG_NOTIFICACAO`] ou [`ORIG_COMUM`]), se ele é
    /// um texto; o texto nunca fica aqui (decisão 0072).
    forma_do_prompt: Option<&'static str>,
    /// Quantos agendamentos o `session_crons` tem, se é uma lista.
    agendamentos: Option<u64>,
}

/// As tarefas em segundo plano: quantas a lista tem e as primeiras válidas
/// (tipo normalizado e id), sem guardar o resto delas.
#[derive(Debug, Default, Clone, PartialEq)]
struct Tarefas {
    contagem: u64,
    validas: Vec<(String, String)>,
}

/// O que a [`Espia`] viu: os primeiros bytes depois da chave `prompt`, até
/// [`JANELA_DO_PROMPT`], enquanto ligada.
#[derive(Debug, Default)]
struct Espiada {
    ligada: Cell<bool>,
    bytes: RefCell<Vec<u8>>,
}

impl Espiada {
    /// Começa a olhar: o próximo byte lido é o primeiro da janela.
    fn ligar(&self) {
        self.bytes.borrow_mut().clear();
        self.ligada.set(true);
    }

    /// Para de olhar e devolve a forma do prompt pelo que viu; a janela é
    /// esvaziada na hora.
    fn forma(&self) -> Option<&'static str> {
        self.ligada.set(false);
        let mut bytes = self.bytes.borrow_mut();
        let forma = forma_do_prompt(&bytes);
        bytes.clear();
        forma
    }
}

/// A entrada do hook, com uma janela para os primeiros bytes de um valor
/// (decisão 0072): o leitor do JSON pega um byte de cada vez, e a espiã só
/// guarda o que passa enquanto está ligada, até [`JANELA_DO_PROMPT`] bytes.
struct Espia<R> {
    dentro: R,
    espiada: Rc<Espiada>,
}

impl<R: Read> Read for Espia<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.dentro.read(buf)?;
        if self.espiada.ligada.get() {
            let mut bytes = self.espiada.bytes.borrow_mut();
            let pega = n.min(JANELA_DO_PROMPT.saturating_sub(bytes.len()));
            bytes.extend_from_slice(&buf[..pega]);
            if bytes.len() >= JANELA_DO_PROMPT {
                self.espiada.ligada.set(false);
            }
        }
        Ok(n)
    }
}

/// A forma do prompt pelos primeiros bytes crus que vieram depois da chave
/// (os dois-pontos, as aspas e o começo do texto, escapado como no JSON):
/// [`ORIG_NOTIFICACAO`] se é um texto que começa (depois de espaços) por
/// [`ETIQUETA_DE_NOTIFICACAO`], [`ORIG_COMUM`] se é outro texto, `None` se o
/// valor não é texto.
fn forma_do_prompt(janela: &[u8]) -> Option<&'static str> {
    let mut i = 0;
    while janela
        .get(i)
        .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b':'))
    {
        i += 1;
    }
    if janela.get(i) != Some(&b'"') {
        return None;
    }
    i += 1;
    // Espaços no começo do texto, crus ou escapados (`\n`, `\t`, `\r`).
    loop {
        match (janela.get(i), janela.get(i + 1)) {
            (Some(b' '), _) => i += 1,
            (Some(b'\\'), Some(b'n' | b't' | b'r')) => i += 2,
            _ => break,
        }
    }
    let resto = janela.get(i..).unwrap_or_default();
    Some(if resto.starts_with(ETIQUETA_DE_NOTIFICACAO.as_bytes()) {
        ORIG_NOTIFICACAO
    } else {
        ORIG_COMUM
    })
}

/// Lê o JSON do hook de `leitor`, em fluxo. Só o primeiro valor conta: com
/// ele fechado, o hook não espera o fim da entrada. `null` vale como objeto
/// vazio (como no jq); outra coisa que não um objeto é erro.
pub fn ler_de(leitor: impl Read) -> Result<Lido, serde_json::Error> {
    let espiada = Rc::new(Espiada::default());
    let entrada = Espia {
        dentro: leitor,
        espiada: Rc::clone(&espiada),
    };
    LeitorDoHook { espiada }.deserialize(&mut serde_json::Deserializer::from_reader(entrada))
}

/// [`ler_de`] de bytes já na memória (o mesmo caminho, em fluxo).
pub fn ler(entrada: &[u8]) -> Result<Lido, serde_json::Error> {
    ler_de(entrada)
}

/// O texto cru de um campo vira valor; o que não se lê (um substituto
/// sozinho, um número fora do `f64`, aninhamento demais) fica de fora.
fn interpretar(cru: &RawValue) -> Option<Value> {
    serde_json::from_str(cru.get()).ok()
}

/// Guarda o campo `nome` (o último vale, como no jq); se ele não se lê, o
/// campo cai.
fn guardar(campos: &mut Map<String, Value>, nome: &str, cru: &RawValue) {
    match interpretar(cru) {
        Some(valor) => {
            campos.insert(nome.to_owned(), valor);
        }
        None => {
            campos.remove(nome);
        }
    }
}

/// Uma chave do JSON do hook, lida como bytes: o texto de uma chave que a
/// lista não usa nunca é validado nem guardado.
enum Chave {
    Escalar(&'static str),
    EntradaDaFerramenta,
    Tarefas,
    /// O `prompt`: só a forma dele, pela janela da espiã.
    Prompt,
    /// O `session_crons`: só a contagem.
    Agendamentos,
    Outra,
}

impl Chave {
    fn de(nome: &[u8]) -> Chave {
        match nome {
            b"tool_input" => Chave::EntradaDaFerramenta,
            b"background_tasks" => Chave::Tarefas,
            b"prompt" => Chave::Prompt,
            b"session_crons" => Chave::Agendamentos,
            _ => ESCALARES
                .iter()
                .find(|e| e.as_bytes() == nome)
                .map_or(Chave::Outra, |e| Chave::Escalar(e)),
        }
    }
}

/// O visitante das chaves: aceita a chave em bytes ou em texto.
struct VisitanteDeChave<F>(F);

impl<'de, T, F: FnOnce(&[u8]) -> T> Visitor<'de> for VisitanteDeChave<F> {
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("uma chave")
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<T, E> {
        Ok((self.0)(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<T, E> {
        Ok((self.0)(v.as_bytes()))
    }
}

impl<'de> Deserialize<'de> for Chave {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Chave, D::Error> {
        d.deserialize_bytes(VisitanteDeChave(Chave::de))
    }
}

/// Uma chave de dentro do `tool_input`: só o caminho interessa.
struct ChaveDoCaminho(Option<&'static str>);

impl<'de> Deserialize<'de> for ChaveDoCaminho {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<ChaveDoCaminho, D::Error> {
        d.deserialize_bytes(VisitanteDeChave(|nome: &[u8]| {
            ChaveDoCaminho(match nome {
                b"file_path" => Some("file_path"),
                b"notebook_path" => Some("notebook_path"),
                _ => None,
            })
        }))
    }
}

/// Uma chave de uma tarefa em segundo plano: só o tipo e o id interessam.
struct ChaveDaTarefa(Option<&'static str>);

impl<'de> Deserialize<'de> for ChaveDaTarefa {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<ChaveDaTarefa, D::Error> {
        d.deserialize_bytes(VisitanteDeChave(|nome: &[u8]| {
            ChaveDaTarefa(match nome {
                b"type" => Some("type"),
                b"id" => Some("id"),
                _ => None,
            })
        }))
    }
}

/// Num visitante que espera um tipo, qualquer valor simples de outro tipo
/// vale `$nada` (o campo cai, como um de tipo errado).
macro_rules! simples_valem {
    ($nada:expr) => {
        fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok($nada)
        }
        fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
            Ok($nada)
        }
        fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
            Ok($nada)
        }
        fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
            Ok($nada)
        }
        fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
            Ok($nada)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok($nada)
        }
    };
}

/// Pula uma lista inteira sem interpretar os itens.
fn pular_lista<'de, A: SeqAccess<'de>>(mut lista: A) -> Result<(), A::Error> {
    while lista.next_element::<IgnoredAny>()?.is_some() {}
    Ok(())
}

/// Pula um objeto inteiro sem interpretar as chaves nem os valores.
fn pular_objeto<'de, A: MapAccess<'de>>(mut objeto: A) -> Result<(), A::Error> {
    while objeto.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
    Ok(())
}

/// Lê o [`Lido`] com a janela da espiã para o `prompt`.
struct LeitorDoHook {
    espiada: Rc<Espiada>,
}

impl<'de> DeserializeSeed<'de> for LeitorDoHook {
    type Value = Lido;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Lido, D::Error> {
        struct V(Rc<Espiada>);

        impl<'de> Visitor<'de> for V {
            type Value = Lido;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("o objeto JSON do hook")
            }

            fn visit_unit<E: de::Error>(self) -> Result<Lido, E> {
                Ok(Lido::default())
            }

            fn visit_map<A: MapAccess<'de>>(self, mut objeto: A) -> Result<Lido, A::Error> {
                let mut lido = Lido::default();
                while let Some(chave) = objeto.next_key::<Chave>()? {
                    match chave {
                        Chave::Escalar(nome) => {
                            let cru: Box<RawValue> = objeto.next_value()?;
                            guardar(&mut lido.campos, nome, &cru);
                        }
                        Chave::EntradaDaFerramenta => {
                            lido.entrada_da_ferramenta =
                                objeto.next_value::<EntradaDaFerramenta>()?.0;
                        }
                        Chave::Tarefas => {
                            lido.tarefas = objeto.next_value::<ListaDeTarefas>()?.0;
                        }
                        Chave::Prompt => {
                            // A janela começa logo depois da chave: os
                            // dois-pontos, as aspas e o começo do texto. O
                            // texto passa sem ser guardado (o `IgnoredAny`
                            // não junta os bytes dele).
                            self.0.ligar();
                            let pulou = objeto.next_value::<IgnoredAny>();
                            let forma = self.0.forma();
                            pulou?;
                            lido.forma_do_prompt = forma;
                        }
                        Chave::Agendamentos => {
                            lido.agendamentos = objeto.next_value::<Contagem>()?.0;
                        }
                        Chave::Outra => {
                            objeto.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                Ok(lido)
            }
        }

        d.deserialize_any(V(self.espiada))
    }
}

/// Quantos itens uma lista tem, sem olhar nenhum; `None` se não é lista.
struct Contagem(Option<u64>);

impl<'de> Deserialize<'de> for Contagem {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Contagem, D::Error> {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = Contagem;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("uma lista")
            }

            simples_valem!(Contagem(None));

            fn visit_map<A: MapAccess<'de>>(self, objeto: A) -> Result<Self::Value, A::Error> {
                pular_objeto(objeto)?;
                Ok(Contagem(None))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut lista: A) -> Result<Self::Value, A::Error> {
                let mut n: u64 = 0;
                while lista.next_element::<IgnoredAny>()?.is_some() {
                    n = n.saturating_add(1);
                }
                Ok(Contagem(Some(n)))
            }
        }

        d.deserialize_any(V)
    }
}

/// O `tool_input`: só o caminho, se for um objeto.
struct EntradaDaFerramenta(Option<Map<String, Value>>);

impl<'de> Deserialize<'de> for EntradaDaFerramenta {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<EntradaDaFerramenta, D::Error> {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = EntradaDaFerramenta;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("o tool_input")
            }

            simples_valem!(EntradaDaFerramenta(None));

            fn visit_seq<A: SeqAccess<'de>>(self, lista: A) -> Result<Self::Value, A::Error> {
                pular_lista(lista)?;
                Ok(EntradaDaFerramenta(None))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut objeto: A) -> Result<Self::Value, A::Error> {
                let mut campos = Map::new();
                while let Some(ChaveDoCaminho(nome)) = objeto.next_key()? {
                    match nome {
                        Some(nome) => {
                            let cru: Box<RawValue> = objeto.next_value()?;
                            guardar(&mut campos, nome, &cru);
                        }
                        None => {
                            objeto.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                Ok(EntradaDaFerramenta(Some(campos)))
            }
        }

        d.deserialize_any(V)
    }
}

/// O `background_tasks`: a contagem e as primeiras válidas, se for uma
/// lista.
struct ListaDeTarefas(Option<Tarefas>);

impl<'de> Deserialize<'de> for ListaDeTarefas {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<ListaDeTarefas, D::Error> {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = ListaDeTarefas;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("o background_tasks")
            }

            simples_valem!(ListaDeTarefas(None));

            fn visit_map<A: MapAccess<'de>>(self, objeto: A) -> Result<Self::Value, A::Error> {
                pular_objeto(objeto)?;
                Ok(ListaDeTarefas(None))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut lista: A) -> Result<Self::Value, A::Error> {
                let mut tarefas = Tarefas::default();
                while let Some(Tarefa(tarefa)) = lista.next_element()? {
                    tarefas.contagem += 1;
                    if let Some(par) = tarefa
                        && tarefas.validas.len() < MAX_TAREFAS
                    {
                        tarefas.validas.push(par);
                    }
                }
                Ok(ListaDeTarefas(Some(tarefas)))
            }
        }

        d.deserialize_any(V)
    }
}

/// Uma tarefa em segundo plano: `Some((tipo, id))` se é um objeto com tipo e
/// id válidos.
struct Tarefa(Option<(String, String)>);

impl<'de> Deserialize<'de> for Tarefa {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Tarefa, D::Error> {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = Tarefa;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("uma tarefa")
            }

            simples_valem!(Tarefa(None));

            fn visit_seq<A: SeqAccess<'de>>(self, lista: A) -> Result<Self::Value, A::Error> {
                pular_lista(lista)?;
                Ok(Tarefa(None))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut objeto: A) -> Result<Self::Value, A::Error> {
                let mut campos = Map::new();
                while let Some(ChaveDaTarefa(nome)) = objeto.next_key()? {
                    match nome {
                        Some(nome) => {
                            let cru: Box<RawValue> = objeto.next_value()?;
                            guardar(&mut campos, nome, &cru);
                        }
                        None => {
                            objeto.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                let par = tipo_de_tarefa(campos.get("type")).zip(texto(campos.get("id"), eh_id));
                Ok(Tarefa(par))
            }
        }

        d.deserialize_any(V)
    }
}

/// O caminho editado, só no PostToolUse de uma ferramenta de edição:
/// `tool_input.file_path`, ou `notebook_path` se ele faltar (o `//` do jq).
/// Nunca sai daqui: vira [`hash_do_caminho`].
pub fn caminho_editado<'a>(evento: &str, lido: &'a Lido) -> Option<&'a str> {
    if evento != "PostToolUse" {
        return None;
    }
    let ferramenta = lido.campos.get("tool_name")?.as_str()?;
    if !FERRAMENTAS_DE_EDICAO.contains(&ferramenta) {
        return None;
    }
    let entrada = lido.entrada_da_ferramenta.as_ref()?;
    let alvo = match entrada.get("file_path") {
        Some(v) if !matches!(v, Value::Null | Value::Bool(false)) => v,
        _ => entrada.get("notebook_path")?,
    };
    alvo.as_str().filter(|c| !c.is_empty())
}

fn texto(v: Option<&Value>, regra: impl Fn(&str) -> bool) -> Option<String> {
    v?.as_str().filter(|s| regra(s)).map(str::to_owned)
}

fn booleano(v: Option<&Value>) -> Option<bool> {
    v?.as_bool()
}

/// Número de 0 a `max`, arredondado para baixo (o `floor` do jq).
fn natural(v: Option<&Value>, max: u64) -> Option<u64> {
    let n = v?.as_f64()?;
    (n >= 0.0 && n <= max as f64).then(|| n.floor() as u64)
}

/// O tipo de uma tarefa em segundo plano: minúsculas, cada trecho fora de
/// `[a-z_]` vira um `_` (o `gsub` do jq) e, fora da lista fechada, `outro`.
fn tipo_de_tarefa(v: Option<&Value>) -> Option<String> {
    let bruto = v?.as_str()?;
    let mut normal = String::with_capacity(bruto.len());
    let mut em_trecho = false;
    for c in bruto.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c == '_' {
            normal.push(c);
            em_trecho = false;
        } else if !em_trecho {
            normal.push('_');
            em_trecho = true;
        }
    }
    Some(if TIPOS_DE_TAREFA.contains(&normal.as_str()) {
        normal
    } else {
        "outro".into()
    })
}

/// Só o nome da última pasta do `cwd`, nunca o caminho. Corta nas duas
/// barras: no Windows o `cwd` vem como `C:\Users\x\projeto` (uma barra
/// invertida nunca passa no validador do pet, então no Linux nada muda).
fn pasta(v: Option<&Value>) -> Option<String> {
    let cwd = v?.as_str()?;
    let ultima = cwd.rsplit(['/', '\\']).find(|p| !p.is_empty())?;
    eh_projeto(ultima).then(|| ultima.to_owned())
}

/// Monta o corpo do fio v1 a partir do que o hook leu. `arq` é o hash do
/// caminho editado ([`hash_do_caminho`]), calculado por quem chama. Cada
/// campo só vale no evento que o tem e passa pelo validador do pet; nenhum
/// outro campo do hook é lido.
pub fn montar(lido: &Lido, ctx: &Contexto, arq: Option<&str>) -> Fio {
    let e = ctx.evento;
    let campo = |nome: &str| lido.campos.get(nome);
    let de_ferramenta = matches!(
        e,
        "PreToolUse" | "PostToolUse" | "PostToolUseFailure" | "PermissionRequest"
    );
    let validas: &[(String, String)] = lido.tarefas.as_ref().map_or(&[], |t| &t.validas);
    let stop = e == "Stop";
    Fio {
        v: 1,
        e: e.to_owned(),
        ts: ctx.ts.filter(|&t| t > 0 && t < 1 << 53),
        sid: texto(campo("session_id"), eh_id),
        turno: texto(campo("prompt_id"), eh_id),
        agente: campo("agent_id").and_then(Value::as_str).map(|_| true),
        aid: texto(campo("agent_id"), eh_id),
        tool: if de_ferramenta {
            texto(campo("tool_name"), |s| eh_token(s, MAX_FERRAMENTA))
        } else {
            None
        },
        nt: if e == "Notification" {
            texto(campo("notification_type"), eh_enum)
        } else {
            None
        },
        err: if e == "StopFailure" {
            texto(campo("error"), eh_enum)
        } else {
            None
        },
        src: if e == "UserPromptSubmit" || e == "SessionStart" {
            texto(campo("source"), eh_enum)
        } else {
            None
        },
        orig: if e == "UserPromptSubmit" {
            lido.forma_do_prompt
                .filter(|f| eh_enum(f))
                .map(str::to_owned)
        } else {
            None
        },
        reason: if e == "SessionEnd" {
            texto(campo("reason"), eh_enum)
        } else {
            None
        },
        intr: if e == "PostToolUseFailure" {
            booleano(campo("is_interrupt"))
        } else {
            None
        },
        sha: if stop {
            booleano(campo("stop_hook_active"))
        } else {
            None
        },
        bg: lido
            .tarefas
            .as_ref()
            .filter(|_| stop)
            .map(|t| t.contagem)
            .filter(|&n| n <= MAX_CONTAGEM),
        bgt: (stop && !validas.is_empty()).then(|| validas.iter().map(|t| t.0.clone()).collect()),
        bgi: (stop && !validas.is_empty()).then(|| validas.iter().map(|t| t.1.clone()).collect()),
        crn: lido
            .agendamentos
            .filter(|_| stop)
            .filter(|&n| n <= MAX_CONTAGEM),
        dur: if e == "PostToolUse" || e == "PostToolUseFailure" {
            natural(campo("duration_ms"), MAX_DURACAO_MS)
        } else {
            None
        },
        arq: arq.filter(|a| eh_hash_de_arquivo(a)).map(str::to_owned),
        proj: pasta(campo("cwd")),
        ent: ctx.ent.filter(|o| eh_origem(o)).map(str::to_owned),
        dnd: ctx.dnd,
        teste: ctx.teste.then_some(true),
        term: terminal(e, &ctx.terminal),
        app: app(e, ctx.app),
    }
}

/// O corpo inteiro, do jeito que vai no POST, a partir do que se leu do
/// JSON do hook; o que não é um objeto JSON (ou não fecha) vira o
/// [`minimo`]. `null` vale como objeto vazio (como no jq).
pub fn corpo_do_lido(lido: Result<Lido, serde_json::Error>, ctx: &Contexto) -> String {
    let Ok(lido) = lido else {
        return minimo(ctx.evento, ctx.teste);
    };
    let arq = caminho_editado(ctx.evento, &lido).map(hash_do_caminho);
    let fio = montar(&lido, ctx, arq.as_deref());
    serde_json::to_string(&fio).unwrap_or_else(|_| minimo(ctx.evento, ctx.teste))
}

/// [`corpo_do_lido`] de bytes já na memória.
pub fn corpo(entrada: &[u8], ctx: &Contexto) -> String {
    corpo_do_lido(ler(entrada), ctx)
}

/// [`corpo_do_lido`] lendo em fluxo (a entrada padrão do hook).
pub fn corpo_de(leitor: impl Read, ctx: &Contexto) -> String {
    corpo_do_lido(ler_de(leitor), ctx)
}

#[cfg(test)]
mod testes {
    use serde_json::json;

    use super::*;
    use crate::evento;

    fn ctx(e: &str) -> Contexto<'_> {
        Contexto {
            evento: e,
            ts: Some(1_790_000_000_000),
            ent: Some("cli"),
            dnd: false,
            teste: false,
            terminal: IdsDoAmbiente::default(),
            app: None,
        }
    }

    #[test]
    fn app_do_macos_so_no_inicio_e_no_prompt_e_so_valido() {
        let com = |e: &str, app: Option<&'static str>| {
            let c = Contexto { app, ..ctx(e) };
            let texto = corpo(br#"{"session_id":"s1"}"#, &c);
            serde_json::from_str::<Value>(&texto).unwrap()
        };
        for e in EVENTOS_COM_TERMINAL {
            assert_eq!(
                com(e, Some("com.googlecode.iterm2"))["app"],
                json!("com.googlecode.iterm2"),
                "{e}"
            );
        }
        // Fora dos eventos de casar janela, não sai.
        assert!(
            com("Stop", Some("com.googlecode.iterm2"))
                .get("app")
                .is_none()
        );
        // Um app que não é token não sai.
        assert!(
            com("UserPromptSubmit", Some("não vale"))
                .get("app")
                .is_none()
        );
    }

    #[test]
    fn ids_de_terminal_so_no_inicio_e_no_prompt_e_so_os_validos() {
        let ids = IdsDoAmbiente {
            tmux: Some("%3"),
            kitty: Some("12"),
            wezterm: Some("4"),
        };
        let com = |e: &str, ids: IdsDoAmbiente<'static>| {
            let c = Contexto {
                terminal: ids,
                ..ctx(e)
            };
            let texto = corpo(br#"{"session_id":"s1"}"#, &c);
            let lido = evento::ler(texto.as_bytes()).expect("o pet aceita");
            assert!(lido.descartados.is_empty(), "{texto}");
            serde_json::from_str::<Value>(&texto).unwrap()
        };
        for e in EVENTOS_COM_TERMINAL {
            assert_eq!(
                com(e, ids)["term"],
                json!({"tmux": "%3", "kitty": "12", "wezterm": "4"}),
                "{e}"
            );
        }
        for e in ["Stop", "PostToolUse", "Notification", "SessionEnd"] {
            assert!(
                com(e, ids).get("term").is_none(),
                "{e}: só no começo e no prompt"
            );
        }
        let ruins = IdsDoAmbiente {
            tmux: Some("%3; SEGREDO"),
            kitty: Some("SEGREDO-janela"),
            wezterm: Some("4"),
        };
        let v = com("UserPromptSubmit", ruins);
        assert_eq!(v["term"], json!({"wezterm": "4"}), "o ruim cai sozinho");
        assert!(!v.to_string().to_lowercase().contains("segredo"));
        let nenhum = IdsDoAmbiente {
            tmux: Some(""),
            ..IdsDoAmbiente::default()
        };
        assert!(com("SessionStart", nenhum).get("term").is_none());
    }

    fn montado(e: &str, entrada: Value) -> Value {
        let texto = corpo(entrada.to_string().as_bytes(), &ctx(e));
        let lido = evento::ler(texto.as_bytes()).expect("o pet aceita");
        assert!(lido.descartados.is_empty(), "{:?}", lido.descartados);
        serde_json::from_str(&texto).unwrap()
    }

    #[test]
    fn minimo_byte_a_byte() {
        assert_eq!(minimo("Stop", false), r#"{"v":1,"e":"Stop"}"#);
        assert_eq!(minimo("Stop", true), r#"{"v":1,"e":"Stop","teste":true}"#);
        for entrada in [&b""[..], b"[1]", b"\"x\"", b"3", b"{", b"\xff"] {
            assert_eq!(corpo(entrada, &ctx("Stop")), r#"{"v":1,"e":"Stop"}"#);
        }
    }

    #[test]
    fn null_vale_como_objeto_vazio() {
        let v: Value = serde_json::from_str(&corpo(b"null", &ctx("Stop"))).unwrap();
        assert_eq!(
            v,
            json!({"v": 1, "e": "Stop", "ts": 1_790_000_000_000u64, "ent": "cli", "dnd": false})
        );
    }

    #[test]
    fn porta_so_com_digitos() {
        assert_eq!(porta(Some("28001")), 28001);
        for ruim in [
            None,
            Some(""),
            Some("abc"),
            Some("123456"),
            Some("1:2@evil.com"),
            Some("0"),
            Some("99999"),
        ] {
            assert_eq!(porta(ruim), PORTA_PADRAO, "{ruim:?}");
        }
    }

    #[test]
    fn dnd_so_com_true_de_verdade() {
        assert!(dnd_do_omarchy(
            br#"{"dnd":true,"past":[{"body":"SEGREDO"}]}"#
        ));
        for ruim in [
            &br#"{"dnd":false}"#[..],
            br#"{"dnd":"true"}"#,
            b"nao e json",
            b"[true]",
            b"",
        ] {
            assert!(!dnd_do_omarchy(ruim), "{:?}", String::from_utf8_lossy(ruim));
        }
    }

    #[test]
    fn tipos_de_tarefa_por_lista_fechada() {
        let t = |s: &str| tipo_de_tarefa(Some(&json!(s)));
        assert_eq!(t("MCP task").as_deref(), Some("mcp_task"));
        assert_eq!(t("auto-mode scan").as_deref(), Some("auto_mode_scan"));
        assert_eq!(t("shell").as_deref(), Some("shell"));
        assert_eq!(t("SEGREDO Tarefa").as_deref(), Some("outro"));
        assert_eq!(t("shell\n").as_deref(), Some("outro"), "shell_ não é shell");
        assert_eq!(tipo_de_tarefa(Some(&json!(3))), None);
    }

    fn lido(v: Value) -> Lido {
        ler(v.to_string().as_bytes()).expect("objeto")
    }

    #[test]
    fn hash_e_caminho_so_da_edicao() {
        assert_eq!(hash_do_caminho("/tmp/x"), {
            let h = hex(&Sha256::digest(b"/tmp/x"));
            h[..12].to_owned()
        });
        let edit = lido(json!({"tool_name": "Edit", "tool_input": {"file_path": "/a/b.rs"}}));
        assert_eq!(caminho_editado("PostToolUse", &edit), Some("/a/b.rs"));
        assert_eq!(caminho_editado("PreToolUse", &edit), None);
        let nb = lido(json!({"tool_name": "NotebookEdit",
                        "tool_input": {"file_path": null, "notebook_path": "/n.ipynb"}}));
        assert_eq!(caminho_editado("PostToolUse", &nb), Some("/n.ipynb"));
        let leitura = lido(json!({"tool_name": "Read", "tool_input": {"file_path": "/etc/x"}}));
        assert_eq!(caminho_editado("PostToolUse", &leitura), None);
        let ruim = lido(json!({"tool_name": "Write", "tool_input": "SEGREDO"}));
        assert_eq!(caminho_editado("PostToolUse", &ruim), None);
        let lista = lido(json!({"tool_name": "Write", "tool_input": [{"file_path": "/x"}]}));
        assert_eq!(caminho_editado("PostToolUse", &lista), None);
        let vazio = lido(json!({"tool_name": "Write", "tool_input": {"file_path": ""}}));
        assert_eq!(caminho_editado("PostToolUse", &vazio), None);
    }

    #[test]
    fn so_a_lista_branca_e_guardada() {
        // O resto do JSON nem vira valor: só os campos da lista ficam.
        let l = lido(json!({
            "session_id": "s1", "prompt": "SEGREDO", "last_assistant_message": "SEGREDO",
            "tool_input": {"file_path": "/a", "content": "SEGREDO", "old_string": "SEGREDO"},
            "background_tasks": [{"id": "t1", "type": "shell", "description": "SEGREDO"}],
            "tool_response": {"stdout": "SEGREDO"}
        }));
        let guardado = format!("{l:?}");
        assert!(!guardado.contains("SEGREDO"), "{guardado}");
        assert_eq!(l.campos.len(), 1);
        assert_eq!(
            l.tarefas,
            Some(Tarefas {
                contagem: 1,
                validas: vec![("shell".into(), "t1".into())]
            })
        );
    }

    /// O mesmo hook, com `ruim` (um texto JSON) num campo que a lista não
    /// lê: os metadados ficam.
    fn com_lixo_em(campo: &str, ruim: &str) -> Value {
        let texto = format!(
            r#"{{"session_id":"s1","prompt_id":"p1","stop_hook_active":true,"{campo}":{ruim},"cwd":"/home/x/proj"}}"#
        );
        let corpo = corpo(texto.as_bytes(), &ctx("Stop"));
        serde_json::from_str(&corpo).unwrap()
    }

    #[test]
    fn lixo_fora_da_lista_nao_derruba_os_metadados() {
        let fundo = format!("{}{}", "[".repeat(200), "]".repeat(200));
        let objetos = format!("{}1{}", "{\"a\":".repeat(200), "}".repeat(200));
        for ruim in [
            r#""fim \udc00""#, // substituto baixo sozinho
            r#""fim \ud83d""#, // substituto alto sozinho (emoji cortado)
            r#""\ud83d\ud83d""#,
            "1e400", // número fora do f64
            "-1e999",
            &fundo,   // 200 níveis de lista
            &objetos, // 200 níveis de objeto
        ] {
            for campo in ["prompt", "last_assistant_message", "tool_response", "x"] {
                let v = com_lixo_em(campo, ruim);
                assert_eq!(
                    (&v["sid"], &v["turno"], &v["ent"], &v["sha"], &v["proj"]),
                    (
                        &json!("s1"),
                        &json!("p1"),
                        &json!("cli"),
                        &json!(true),
                        &json!("proj")
                    ),
                    "{campo}: {ruim:.40}"
                );
            }
        }
        // Bytes que não são UTF-8, num campo que o hook não lê.
        let mut bruto = br#"{"session_id":"s1","last_assistant_message":"a"#.to_vec();
        bruto.extend_from_slice(b"\xff\xfe");
        bruto.extend_from_slice(br#"b","prompt_id":"p1"}"#);
        let v: Value = serde_json::from_str(&corpo(&bruto, &ctx("Stop"))).unwrap();
        assert_eq!((&v["sid"], &v["turno"]), (&json!("s1"), &json!("p1")));
        // Na descrição de uma tarefa.
        let texto = r#"{"session_id":"s1","background_tasks":[{"id":"t1","type":"shell","description":"x \udc00"}]}"#;
        let v: Value = serde_json::from_str(&corpo(texto.as_bytes(), &ctx("Stop"))).unwrap();
        assert_eq!((&v["sid"], &v["bgi"]), (&json!("s1"), &json!(["t1"])));
    }

    #[test]
    fn campo_da_lista_que_nao_se_le_cai_sozinho() {
        // O substituto sozinho no próprio session_id: ele cai, o resto fica.
        let texto = r#"{"session_id":"s\udc00","prompt_id":"p1","duration_ms":1e400}"#;
        let v: Value = serde_json::from_str(&corpo(texto.as_bytes(), &ctx("PostToolUse"))).unwrap();
        assert!(v.get("sid").is_none() && v.get("dur").is_none(), "{v}");
        assert_eq!(v["turno"], "p1");
        // Chave repetida: vale a última (como no jq), e uma última ruim cai.
        let texto = r#"{"session_id":"a","session_id":"b","prompt_id":"p","prompt_id":"\udc00"}"#;
        let v: Value = serde_json::from_str(&corpo(texto.as_bytes(), &ctx("Stop"))).unwrap();
        assert_eq!(v["sid"], "b");
        assert!(v.get("turno").is_none(), "{v}");
        // Uma chave estranha (bytes quebrados) é só pulada.
        let mut bruto = b"{\"\xff\":1,\"session_id\":\"s1\"}".to_vec();
        bruto.truncate(bruto.len());
        let v: Value = serde_json::from_str(&corpo(&bruto, &ctx("Stop"))).unwrap();
        assert_eq!(v["sid"], "s1");
    }

    #[test]
    fn lista_de_tarefas_enorme_conta_sem_guardar() {
        let mut texto = String::from(r#"{"background_tasks":["#);
        for i in 0..50_000 {
            if i > 0 {
                texto.push(',');
            }
            texto.push_str(&format!(
                r#"{{"id":"t{i}","type":"shell","description":"SEGREDO"}}"#
            ));
        }
        texto.push_str("]}");
        let l = ler(texto.as_bytes()).unwrap();
        let t = l.tarefas.unwrap();
        assert_eq!(t.contagem, 50_000);
        assert_eq!(t.validas.len(), MAX_TAREFAS, "só as primeiras");
    }

    #[test]
    fn em_fluxo_nao_espera_o_fim_da_entrada() {
        // Um leitor que entrega o objeto e depois diria que há mais para
        // ler (falharia se fosse lido): o hook para no fim do objeto.
        struct Leitor(Vec<u8>, usize);
        impl Read for Leitor {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.1 >= self.0.len() {
                    return Err(std::io::Error::other("a entrada não fecha"));
                }
                let n = buf.len().min(self.0.len() - self.1);
                buf[..n].copy_from_slice(&self.0[self.1..self.1 + n]);
                self.1 += n;
                Ok(n)
            }
        }
        let leitor = Leitor(br#"{"session_id":"s1"}"#.to_vec(), 0);
        let v: Value = serde_json::from_str(&corpo_de(leitor, &ctx("Stop"))).unwrap();
        assert_eq!(v["sid"], "s1");
    }

    #[test]
    fn pasta_do_windows_e_do_linux() {
        let proj = |cwd: &str| pasta(Some(&json!(cwd)));
        assert_eq!(proj("/home/x/meu-projeto").as_deref(), Some("meu-projeto"));
        assert_eq!(proj("/home/x/meu-projeto/").as_deref(), Some("meu-projeto"));
        assert_eq!(
            proj(r"C:\Users\x\meu-projeto").as_deref(),
            Some("meu-projeto")
        );
        assert_eq!(
            proj(r"C:\Users\x\meu-projeto\").as_deref(),
            Some("meu-projeto")
        );
        assert_eq!(proj(r"\\servidor\pasta\proj").as_deref(), Some("proj"));
        assert_eq!(proj(r"C:\"), None, "a raiz do disco não é projeto");
        assert_eq!(proj("/"), None);
    }

    #[test]
    fn campos_so_no_evento_que_os_tem() {
        let entrada = json!({
            "session_id": "s1", "prompt_id": "p1", "tool_name": "Bash",
            "notification_type": "idle_prompt", "error": "rate_limit", "source": "user",
            "reason": "logout", "is_interrupt": true, "stop_hook_active": true,
            "duration_ms": 12.9, "background_tasks": [{"id": "t1", "type": "shell"}],
            "cwd": "/home/x/meu-projeto", "prompt": "SEGREDO",
            "session_crons": [{"id": "c1", "schedule": "* * * * *", "recurring": true,
                               "prompt": "SEGREDO-agendado"}]
        });
        let stop = montado("Stop", entrada.clone());
        assert_eq!(stop["sha"], true);
        assert_eq!(
            (stop["bg"].clone(), stop["bgt"].clone(), stop["crn"].clone()),
            (json!(1), json!(["shell"]), json!(1))
        );
        for ausente in [
            "tool", "nt", "err", "src", "orig", "reason", "intr", "dur", "arq",
        ] {
            assert!(stop.get(ausente).is_none(), "{ausente} em {stop}");
        }
        let prompt = montado("UserPromptSubmit", entrada.clone());
        assert_eq!(prompt["orig"], "comum");
        assert!(prompt.get("crn").is_none(), "{prompt}");
        for e in ["SessionStart", "PostToolUse", "Notification", "SessionEnd"] {
            let v = montado(e, entrada.clone());
            assert!(
                v.get("orig").is_none() && v.get("crn").is_none(),
                "{e}: {v}"
            );
        }
        let falha = montado("PostToolUseFailure", entrada.clone());
        assert_eq!(
            (falha["tool"].clone(), falha["intr"].clone()),
            (json!("Bash"), json!(true))
        );
        assert_eq!(falha["dur"], 12, "arredondado para baixo");
        assert!(
            falha.get("err").is_none(),
            "texto de erro de ferramenta nunca vira err"
        );
        assert_eq!(montado("StopFailure", entrada.clone())["err"], "rate_limit");
        assert_eq!(montado("SessionEnd", entrada.clone())["reason"], "logout");
        assert_eq!(
            montado("Notification", entrada.clone())["nt"],
            "idle_prompt"
        );
        assert_eq!(montado("UserPromptSubmit", entrada.clone())["src"], "user");
        assert_eq!(stop["proj"], "meu-projeto");
        assert!(!stop.to_string().to_lowercase().contains("segredo"));
    }

    /// A forma do prompt de um JSON de hook (o `orig` do `UserPromptSubmit`).
    fn orig(texto: &str) -> Option<String> {
        let corpo = corpo(texto.as_bytes(), &ctx("UserPromptSubmit"));
        let v: Value = serde_json::from_str(&corpo).unwrap();
        assert!(!corpo.to_lowercase().contains("segredo"), "vazou: {corpo}");
        v.get("orig").and_then(Value::as_str).map(str::to_owned)
    }

    #[test]
    fn a_forma_do_prompt_so_pelo_comeco() {
        let notificacao = Some("notificacao".to_owned());
        let comum = Some("comum".to_owned());
        // O começo de uma notificação de verdade (decisão 0071), com o
        // resultado da tarefa (conteúdo) logo depois.
        let real = json!({
            "session_id": "s1",
            "prompt": "<task-notification>\n<task-id>a15a73f9648bd8474</task-id>\n\
                       <status>completed</status>\n<result>SEGREDO-resultado</result>\n\
                       </task-notification>",
        });
        assert_eq!(orig(&real.to_string()), notificacao);
        for (texto, esperado) in [
            (r#"{"prompt":"<task-notification>"}"#, &notificacao),
            (
                r#"{"prompt" : "  \n\t<task-notification>SEGREDO"}"#,
                &notificacao,
            ),
            (r#"{"prompt":"SEGREDO <task-notification>"}"#, &comum),
            (r#"{"prompt":"<task-notif"}"#, &comum),
            (r#"{"prompt":"\u003ctask-notification>"}"#, &comum),
            (r#"{"prompt":""}"#, &comum),
            (
                r#"{"prompt":"SEGREDO","session_id":"<task-notification>"}"#,
                &comum,
            ),
            (r#"{"prompt":null}"#, &None),
            (r#"{"prompt":["<task-notification>"]}"#, &None),
            (r#"{"prompt":{"texto":"<task-notification>"}}"#, &None),
            (r#"{"session_id":"s1"}"#, &None),
            // Só a chave de cima: um `prompt` de dentro de outro campo não conta.
            (
                r#"{"tool_input":{"prompt":"<task-notification>"},"x":{"prompt":"SEGREDO"}}"#,
                &None,
            ),
            // Chave repetida: vale a última, como no jq.
            (
                r#"{"prompt":"<task-notification>","prompt":"SEGREDO"}"#,
                &comum,
            ),
        ] {
            assert_eq!(&orig(texto), esperado, "{texto}");
        }
        // Espaços demais antes da etiqueta passam da janela: comum.
        let longe = format!(
            r#"{{"prompt":"{}<task-notification>"}}"#,
            " ".repeat(JANELA_DO_PROMPT)
        );
        assert_eq!(orig(&longe), comum);
        // Só no UserPromptSubmit.
        let v: Value =
            serde_json::from_str(&corpo(br#"{"prompt":"<task-notification>"}"#, &ctx("Stop")))
                .unwrap();
        assert!(v.get("orig").is_none(), "{v}");
    }

    #[test]
    fn o_prompt_nunca_fica_guardado() {
        // O que foi lido não tem nada do prompt, só a forma.
        let texto = json!({"prompt": "<task-notification>SEGREDO-1 SEGREDO-2"}).to_string();
        let l = ler(texto.as_bytes()).unwrap();
        let guardado = format!("{l:?}");
        assert!(!guardado.contains("SEGREDO"), "{guardado}");
        assert_eq!(l.forma_do_prompt, Some(ORIG_NOTIFICACAO));
        // A janela da espiã nunca passa de JANELA_DO_PROMPT bytes e sai vazia,
        // lendo de um em um (como o leitor do JSON) ou em blocos.
        let grande = vec![b'a'; 100_000];
        for bloco in [1usize, 7, 4096] {
            let espiada = Rc::new(Espiada::default());
            let mut espia = Espia {
                dentro: &grande[..],
                espiada: Rc::clone(&espiada),
            };
            espiada.ligar();
            let mut buf = vec![0u8; bloco];
            while espia.read(&mut buf).unwrap() > 0 {
                assert!(espiada.bytes.borrow().len() <= JANELA_DO_PROMPT);
            }
            assert_eq!(espiada.bytes.borrow().len(), JANELA_DO_PROMPT, "{bloco}");
            assert!(!espiada.ligada.get(), "desliga sozinha com a janela cheia");
            assert_eq!(espiada.forma(), None, "não é texto");
            assert!(espiada.bytes.borrow().is_empty(), "esvaziada");
        }
    }

    #[test]
    fn agendamentos_contados_sem_ler_nada_deles() {
        let crn = |agendamentos: Value| {
            let entrada = json!({"session_id": "s1", "session_crons": agendamentos});
            let corpo = corpo(entrada.to_string().as_bytes(), &ctx("Stop"));
            assert!(!corpo.to_lowercase().contains("segredo"), "{corpo}");
            serde_json::from_str::<Value>(&corpo).unwrap()["crn"].clone()
        };
        let um = json!({"id": "SEGREDO-id", "schedule": "* * * * *", "recurring": true,
                        "prompt": "SEGREDO-prompt-agendado"});
        assert_eq!(crn(json!([um.clone(), um.clone(), 3, null])), json!(4));
        assert_eq!(crn(json!([])), json!(0), "sem agendamento: 0");
        assert_eq!(crn(json!("SEGREDO")), Value::Null, "não é lista");
        assert_eq!(crn(json!({"a": um})), Value::Null);
        assert_eq!(
            crn(json!(vec![json!(1); 10_001])),
            Value::Null,
            "teto do pet"
        );
    }

    #[test]
    fn contagem_e_lista_com_os_tetos_do_pet() {
        let muitas: Vec<Value> = (0..20)
            .map(|i| json!({"id": format!("t{i}"), "type": "shell"}))
            .collect();
        let stop = montado("Stop", json!({"background_tasks": muitas}));
        assert_eq!(stop["bg"], 20);
        assert_eq!(stop["bgi"].as_array().unwrap().len(), 16, "até 16");
        let absurdas = vec![json!(1); 10_001];
        let stop = montado("Stop", json!({"background_tasks": absurdas}));
        assert!(stop.get("bg").is_none(), "contagem que o pet descartaria");
    }

    #[test]
    fn agente_com_qualquer_texto_e_aid_so_token() {
        let sub = montado("PostToolUse", json!({"agent_id": "tem espaço"}));
        assert_eq!(sub["agente"], true);
        assert!(sub.get("aid").is_none());
    }
}
