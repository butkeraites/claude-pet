//! Formato de fio v1 (decisão 0019): o corpo que o `avisar.sh` do plugin
//! `bichinho` manda para `POST /v1/evento`.
//!
//! Só metadados: nome do evento, ids opacos, nome de ferramenta, enums,
//! contagens, durações, o hash do caminho editado e o nome da pasta do
//! projeto. Prompt, código, resposta, texto de erro, títulos e caminhos
//! completos nunca chegam aqui: o `avisar.sh` monta o corpo com uma lista
//! branca do jq, e os testes canário provam.
//!
//! Esta validação é a segunda barreira:
//! - `v` e `e` são obrigatórios; defeito neles recusa o evento (400);
//! - qualquer outro campo com tipo, tamanho ou caractere fora da regra é
//!   **descartado**: só o nome do campo fica em [`Lido::descartados`], nunca
//!   o valor, e o resto do evento vale;
//! - campos desconhecidos são ignorados, para um Claude Code mais novo não
//!   deixar o pet surdo.
//!
//! | Campo | Origem no hook | Regra |
//! |---|---|---|
//! | `v` | — | inteiro, sempre 1 |
//! | `e` | `hook_event_name` (argumento do script) | `[A-Za-z]{1,40}` |
//! | `ts` | relógio do host no início do script (ms) | inteiro de 1 a 2^53−1 |
//! | `sid`, `turno`, `aid` | `session_id`, `prompt_id`, `agent_id` | token de até 64 |
//! | `agente` | `agent_id` presente (o hook veio de dentro de um subagente) | bool |
//! | `tool` | `tool_name` | token de até 128 |
//! | `nt`, `err`, `src`, `reason` | `notification_type`, `error` do StopFailure, `source`, `reason` | `[a-z_]{1,40}` |
//! | `intr`, `sha` | `is_interrupt`, `stop_hook_active` | bool |
//! | `bg` | tamanho de `background_tasks` | inteiro até 10 000 |
//! | `bgt`, `bgi` | tipos (lista fechada no `avisar.sh`, senão `outro`) e ids de `background_tasks` | listas alinhadas de até 16 |
//! | `dur` | `duration_ms` | inteiro até 24 h |
//! | `arq` | sha256 do caminho editado, calculado no host | 12 hexadecimais minúsculos |
//! | `proj` | último componente de `cwd` | até 64 letras, dígitos, espaço, `_.-` |
//! | `ent` | `$CLAUDE_CODE_ENTRYPOINT` | `[a-z0-9_-]{1,40}` |
//! | `dnd` | `~/.local/state/omarchy/notifications.json` | bool |
//! | `teste` | `PET_TESTE=1` (evento sintético do `bin/pet testar`) | bool |
//!
//! Token = `[A-Za-z0-9_.:-]`. Toda regra é de byte ASCII, exceto o nome do
//! projeto, que aceita letras e dígitos Unicode (pastas com acento).

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Versão do fio que este pet entende.
pub const VERSAO: u64 = 1;
/// Ids opacos (`session_id`, `prompt_id`, `agent_id`, ids de tarefa).
pub const MAX_ID: usize = 64;
/// Nomes de ferramenta (os de MCP são longos: `mcp__servidor__ferramenta`).
pub const MAX_FERRAMENTA: usize = 128;
/// Enums do Claude Code (`notification_type`, `error`, `source`, `reason`).
pub const MAX_ENUM: usize = 40;
/// Tarefas em segundo plano listadas por evento (a contagem vai à parte).
pub const MAX_TAREFAS: usize = 16;
/// Caracteres do nome do projeto.
pub const MAX_PROJETO: usize = 64;
/// Maior duração de ferramenta aceita.
pub const MAX_DURACAO_MS: u64 = 24 * 60 * 60 * 1000;
/// Maior contagem de tarefas em segundo plano aceita.
pub const MAX_CONTAGEM: u64 = 10_000;
/// Maior inteiro exato num número JSON.
const MAX_INTEIRO_JSON: u64 = (1 << 53) - 1;

/// Um evento validado. Só tem o que passou nas regras.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Evento {
    /// Nome do evento do Claude Code (`Stop`, `PostToolUse`, …).
    pub e: String,
    /// Relógio do host quando o hook rodou, em ms desde 1970.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    /// `prompt_id`: um turno vai de um prompt ao próximo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turno: Option<String>,
    /// O hook rodou dentro de um subagente.
    #[serde(skip_serializing_if = "eh_falso")]
    pub agente: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "eh_falso")]
    pub intr: bool,
    #[serde(skip_serializing_if = "eh_falso")]
    pub sha: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bgt: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bgi: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dur: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proj: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ent: Option<String>,
    #[serde(skip_serializing_if = "eh_falso")]
    pub dnd: bool,
    #[serde(skip_serializing_if = "eh_falso")]
    pub teste: bool,
}

fn eh_falso(b: &bool) -> bool {
    !*b
}

/// Um corpo lido: o evento e os campos jogados fora (só os nomes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lido {
    pub evento: Evento,
    pub descartados: Vec<&'static str>,
}

/// Por que um corpo foi recusado. As mensagens nunca citam o corpo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroEvento {
    /// Não é um objeto JSON (ou tem chave repetida).
    Json,
    SemVersao,
    Versao,
    SemNome,
    NomeInvalido,
}

impl fmt::Display for ErroEvento {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ErroEvento::Json => "o corpo precisa ser um objeto JSON válido",
            ErroEvento::SemVersao => "falta o campo v",
            ErroEvento::Versao => "v precisa ser 1",
            ErroEvento::SemNome => "falta o campo e",
            ErroEvento::NomeInvalido => "e precisa ser um nome de evento ([A-Za-z], até 40)",
        })
    }
}

impl std::error::Error for ErroEvento {}

/// Tudo como `Value`: tipo errado num campo opcional descarta só o campo.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Bruto {
    v: Option<Value>,
    e: Option<Value>,
    ts: Option<Value>,
    sid: Option<Value>,
    turno: Option<Value>,
    agente: Option<Value>,
    aid: Option<Value>,
    tool: Option<Value>,
    nt: Option<Value>,
    err: Option<Value>,
    src: Option<Value>,
    reason: Option<Value>,
    intr: Option<Value>,
    sha: Option<Value>,
    bg: Option<Value>,
    bgt: Option<Value>,
    bgi: Option<Value>,
    dur: Option<Value>,
    arq: Option<Value>,
    proj: Option<Value>,
    ent: Option<Value>,
    dnd: Option<Value>,
    teste: Option<Value>,
}

/// Lê e valida um corpo de `POST /v1/evento`.
pub fn ler(corpo: &[u8]) -> Result<Lido, ErroEvento> {
    // Só objeto: um array também desserializaria no struct, por posição.
    let primeiro = corpo.iter().find(|b| !b.is_ascii_whitespace());
    if primeiro != Some(&b'{') {
        return Err(ErroEvento::Json);
    }
    // O erro do serde_json pode citar o valor: nunca é repassado.
    let bruto: Bruto = serde_json::from_slice(corpo).map_err(|_| ErroEvento::Json)?;

    match bruto.v.as_ref() {
        None => return Err(ErroEvento::SemVersao),
        Some(v) if v.as_u64() != Some(VERSAO) => return Err(ErroEvento::Versao),
        Some(_) => {}
    }
    let e = match bruto.e.as_ref() {
        None => return Err(ErroEvento::SemNome),
        Some(Value::String(e)) if eh_nome_de_evento(e) => e.clone(),
        Some(_) => return Err(ErroEvento::NomeInvalido),
    };

    let mut d = Vec::new();
    let ts = validar("ts", bruto.ts, &mut d, |v| inteiro(v, 1, MAX_INTEIRO_JSON));
    let sid = validar("sid", bruto.sid, &mut d, |v| texto(v, eh_id));
    let turno = validar("turno", bruto.turno, &mut d, |v| texto(v, eh_id));
    let agente = validar("agente", bruto.agente, &mut d, Value::as_bool);
    let aid = validar("aid", bruto.aid, &mut d, |v| texto(v, eh_id));
    let tool = validar("tool", bruto.tool, &mut d, |v| {
        texto(v, |s| eh_token(s, MAX_FERRAMENTA))
    });
    let nt = validar("nt", bruto.nt, &mut d, |v| texto(v, eh_enum));
    let err = validar("err", bruto.err, &mut d, |v| texto(v, eh_enum));
    let src = validar("src", bruto.src, &mut d, |v| texto(v, eh_enum));
    let reason = validar("reason", bruto.reason, &mut d, |v| texto(v, eh_enum));
    let intr = validar("intr", bruto.intr, &mut d, Value::as_bool);
    let sha = validar("sha", bruto.sha, &mut d, Value::as_bool);
    let bg = validar("bg", bruto.bg, &mut d, |v| inteiro(v, 0, MAX_CONTAGEM));
    let bgt = validar("bgt", bruto.bgt, &mut d, |v| lista(v, eh_enum));
    let bgi = validar("bgi", bruto.bgi, &mut d, |v| lista(v, eh_id));
    let dur = validar("dur", bruto.dur, &mut d, |v| inteiro(v, 0, MAX_DURACAO_MS));
    let arq = validar("arq", bruto.arq, &mut d, |v| texto(v, eh_hash_de_arquivo));
    let proj = validar("proj", bruto.proj, &mut d, |v| texto(v, eh_projeto));
    let ent = validar("ent", bruto.ent, &mut d, |v| texto(v, eh_origem));
    let dnd = validar("dnd", bruto.dnd, &mut d, Value::as_bool);
    let teste = validar("teste", bruto.teste, &mut d, Value::as_bool);

    Ok(Lido {
        evento: Evento {
            e,
            ts,
            sid,
            turno,
            agente: agente.unwrap_or(false),
            aid,
            tool,
            nt,
            err,
            src,
            reason,
            intr: intr.unwrap_or(false),
            sha: sha.unwrap_or(false),
            bg,
            bgt: bgt.unwrap_or_default(),
            bgi: bgi.unwrap_or_default(),
            dur,
            arq,
            proj,
            ent,
            dnd: dnd.unwrap_or(false),
            teste: teste.unwrap_or(false),
        },
        descartados: d,
    })
}

/// Valida um campo opcional presente; se não passa, só o nome fica em
/// `descartados`.
fn validar<T>(
    nome: &'static str,
    valor: Option<Value>,
    descartados: &mut Vec<&'static str>,
    regra: impl FnOnce(&Value) -> Option<T>,
) -> Option<T> {
    let valor = valor?;
    let aceito = regra(&valor);
    if aceito.is_none() {
        descartados.push(nome);
    }
    aceito
}

fn texto(v: &Value, regra: impl Fn(&str) -> bool) -> Option<String> {
    v.as_str().filter(|s| regra(s)).map(str::to_owned)
}

fn inteiro(v: &Value, min: u64, max: u64) -> Option<u64> {
    v.as_u64().filter(|n| (min..=max).contains(n))
}

/// Lista de até [`MAX_TAREFAS`] textos, todos válidos (senão o campo cai
/// inteiro: `bgt` e `bgi` andam alinhados).
fn lista(v: &Value, regra: impl Fn(&str) -> bool) -> Option<Vec<String>> {
    let itens = v.as_array()?;
    if itens.len() > MAX_TAREFAS {
        return None;
    }
    itens.iter().map(|item| texto(item, &regra)).collect()
}

/// Token `[A-Za-z0-9_.:-]{1,max}`.
pub fn eh_token(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
}

/// Id opaco: token de até [`MAX_ID`].
pub fn eh_id(s: &str) -> bool {
    eh_token(s, MAX_ID)
}

/// Enum do Claude Code: `[a-z_]{1,40}`.
pub fn eh_enum(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_ENUM && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}

/// Ponto de entrada do Claude Code (`cli`, `sdk-cli`, `claude-vscode`, …):
/// `[a-z0-9_-]{1,40}`.
pub fn eh_origem(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_ENUM
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Nome de evento de hook: `[A-Za-z]{1,40}`.
pub fn eh_nome_de_evento(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_ENUM && s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// Hash do caminho editado: os 12 primeiros hexadecimais do sha256.
pub fn eh_hash_de_arquivo(s: &str) -> bool {
    s.len() == 12
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Nome da pasta do projeto: até 64 caracteres entre letras e dígitos
/// (Unicode, com acentos compostos ou decompostos), espaço, `_`, `.` e `-`.
/// Nunca barra, controle, aspas ou só pontos.
pub fn eh_projeto(s: &str) -> bool {
    let quantos = s.chars().count();
    (1..=MAX_PROJETO).contains(&quantos)
        && s.len() <= MAX_PROJETO * 4
        && !s.chars().all(|c| c == '.')
        && s.chars().all(|c| {
            c.is_alphanumeric() || matches!(c, ' ' | '_' | '.' | '-') || eh_acento_combinante(c)
        })
}

/// Marcas combinantes (o "´" de um "é" decomposto, como o macOS grava).
fn eh_acento_combinante(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
}

/// Os 8 primeiros caracteres de um id, para mostrar sem expor o id inteiro.
pub fn curto(id: &str) -> String {
    id.chars().take(8).collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ok(corpo: &str) -> Lido {
        ler(corpo.as_bytes()).unwrap_or_else(|e| panic!("{corpo}: {e}"))
    }

    #[test]
    fn evento_completo() {
        let lido = ok(
            r#"{"v":1,"e":"Stop","ts":1790020208123,"sid":"0b9e7c1d-aaaa-4bbb-8ccc-123456789abc",
            "turno":"p-1","agente":true,"aid":"a1","tool":"Edit","nt":"permission_prompt",
            "err":"rate_limit","src":"user","reason":"logout","intr":true,"sha":true,"bg":2,
            "bgt":["subagent","shell"],"bgi":["t1","t2"],"dur":1830,"arq":"3fa2b19c04de",
            "proj":"agenda-presidencial","ent":"cli","dnd":false,"teste":true}"#,
        );
        assert!(lido.descartados.is_empty(), "{:?}", lido.descartados);
        let e = lido.evento;
        assert_eq!(e.e, "Stop");
        assert_eq!(e.ts, Some(1_790_020_208_123));
        assert_eq!(
            e.sid.as_deref(),
            Some("0b9e7c1d-aaaa-4bbb-8ccc-123456789abc")
        );
        assert_eq!(e.turno.as_deref(), Some("p-1"));
        assert!(e.agente && e.intr && e.sha && e.teste && !e.dnd);
        assert_eq!(e.bgt, vec!["subagent", "shell"]);
        assert_eq!(e.bgi, vec!["t1", "t2"]);
        assert_eq!((e.bg, e.dur), (Some(2), Some(1830)));
        assert_eq!(e.arq.as_deref(), Some("3fa2b19c04de"));
        assert_eq!(e.proj.as_deref(), Some("agenda-presidencial"));
        assert_eq!(e.ent.as_deref(), Some("cli"));
    }

    #[test]
    fn so_v_e_e_bastam() {
        let lido = ok(r#"{"v":1,"e":"Stop"}"#);
        assert_eq!(
            lido.evento,
            Evento {
                e: "Stop".into(),
                ..Evento::default()
            }
        );
        assert!(lido.descartados.is_empty());
        assert_eq!(
            serde_json::to_string(&lido.evento).unwrap(),
            r#"{"e":"Stop"}"#
        );
    }

    #[test]
    fn campos_desconhecidos_sao_ignorados_sem_rastro() {
        let lido = ok(
            r#"{"v":1,"e":"UserPromptSubmit","prompt":"SEGREDO-1","tool_input":{"x":"SEGREDO-2"}}"#,
        );
        assert!(lido.descartados.is_empty());
        let texto = format!(
            "{:?} {}",
            lido,
            serde_json::to_string(&lido.evento).unwrap()
        );
        assert!(!texto.contains("SEGREDO"), "{texto}");
    }

    #[test]
    fn campo_invalido_cai_sozinho_e_so_o_nome_fica() {
        let lido = ok(
            r#"{"v":1,"e":"PostToolUse","sid":"ok-1","tool":"Bash; SEGREDO",
            "nt":"SEGREDO-3","proj":"/home/SEGREDO-4/x","dur":"SEGREDO-5","arq":"3FA2B19C04DE",
            "bgt":["shell","SEGREDO-6"],"ent":"CLI","agente":"sim","ts":-5}"#,
        );
        assert_eq!(lido.evento.sid.as_deref(), Some("ok-1"));
        let mut nomes = lido.descartados.clone();
        nomes.sort_unstable();
        assert_eq!(
            nomes,
            vec![
                "agente", "arq", "bgt", "dur", "ent", "nt", "proj", "tool", "ts"
            ]
        );
        let texto = format!(
            "{:?} {}",
            lido,
            serde_json::to_string(&lido.evento).unwrap()
        );
        assert!(!texto.contains("SEGREDO"), "{texto}");
    }

    #[test]
    fn nulo_e_o_mesmo_que_ausente() {
        let lido = ok(r#"{"v":1,"e":"Stop","sid":null,"bg":null}"#);
        assert!(lido.descartados.is_empty());
        assert_eq!(lido.evento.sid, None);
    }

    #[test]
    fn recusas() {
        for (corpo, erro) in [
            ("", ErroEvento::Json),
            ("lixo", ErroEvento::Json),
            ("[1,\"Stop\"]", ErroEvento::Json),
            ("{\"v\":1,\"e\":\"Stop\"", ErroEvento::Json),
            ("{\"v\":1,\"v\":1,\"e\":\"Stop\"}", ErroEvento::Json),
            ("{\"e\":\"Stop\"}", ErroEvento::SemVersao),
            ("{\"v\":2,\"e\":\"Stop\"}", ErroEvento::Versao),
            ("{\"v\":\"1\",\"e\":\"Stop\"}", ErroEvento::Versao),
            ("{\"v\":1.0,\"e\":\"Stop\"}", ErroEvento::Versao),
            ("{\"v\":1}", ErroEvento::SemNome),
            ("{\"v\":1,\"e\":\"Stop!\"}", ErroEvento::NomeInvalido),
            ("{\"v\":1,\"e\":\"\"}", ErroEvento::NomeInvalido),
            ("{\"v\":1,\"e\":3}", ErroEvento::NomeInvalido),
        ] {
            assert_eq!(ler(corpo.as_bytes()), Err(erro), "{corpo}");
        }
        let longo = format!("{{\"v\":1,\"e\":\"{}\"}}", "A".repeat(41));
        assert_eq!(ler(longo.as_bytes()), Err(ErroEvento::NomeInvalido));
    }

    #[test]
    fn mensagens_de_erro_nunca_citam_o_corpo() {
        let r = ler(br#"{"v":"SEGREDO-7","e":"Stop"}"#).unwrap_err();
        assert!(!r.to_string().contains("SEGREDO"));
        let r = ler(br#"{"v":1,"e":"Stop","x":"SEGREDO-8""#).unwrap_err();
        assert!(!r.to_string().contains("SEGREDO"));
    }

    #[test]
    fn limites_de_tamanho() {
        let id64 = "a".repeat(64);
        let lido = ok(&format!(
            r#"{{"v":1,"e":"Stop","sid":"{id64}","turno":"{id64}b"}}"#
        ));
        assert_eq!(lido.evento.sid.as_deref(), Some(id64.as_str()));
        assert_eq!(lido.descartados, vec!["turno"]);

        let mcp = format!("mcp__{}", "x".repeat(123));
        let lido = ok(&format!(r#"{{"v":1,"e":"PostToolUse","tool":"{mcp}"}}"#));
        assert_eq!(lido.evento.tool.as_deref(), Some(mcp.as_str()));
        let lido = ok(&format!(r#"{{"v":1,"e":"PostToolUse","tool":"{mcp}y"}}"#));
        assert_eq!(lido.descartados, vec!["tool"]);

        let dezesseis: Vec<String> = (0..16).map(|i| format!("\"t{i}\"")).collect();
        let lido = ok(&format!(
            r#"{{"v":1,"e":"Stop","bgi":[{}]}}"#,
            dezesseis.join(",")
        ));
        assert_eq!(lido.evento.bgi.len(), 16);
        let lido = ok(&format!(
            r#"{{"v":1,"e":"Stop","bgi":[{},"t16"]}}"#,
            dezesseis.join(",")
        ));
        assert_eq!(lido.descartados, vec!["bgi"]);

        let lido = ok(&format!(
            r#"{{"v":1,"e":"PostToolUse","dur":{},"bg":{}}}"#,
            MAX_DURACAO_MS + 1,
            MAX_CONTAGEM + 1
        ));
        assert_eq!(lido.descartados, vec!["bg", "dur"]);
        let lido = ok(r#"{"v":1,"e":"PostToolUse","dur":12.5}"#);
        assert_eq!(lido.descartados, vec!["dur"]);
    }

    #[test]
    fn regras_de_caracteres() {
        assert!(eh_id("0b9e7c1d-aaaa-4bbb-8ccc-123456789abc"));
        assert!(eh_id("toolu_01:x.y"));
        assert!(!eh_id("tem espaço"));
        assert!(!eh_id(""));
        assert!(eh_enum("auto_mode_scan"));
        assert!(!eh_enum("auto-mode scan"));
        assert!(!eh_enum("Rate_limit"));
        assert!(eh_origem("sdk-cli") && eh_origem("cli") && eh_origem("claude-vscode"));
        assert!(!eh_origem("CLI") && !eh_origem("sdk cli"));
        assert!(eh_nome_de_evento("PostToolUseFailure"));
        assert!(!eh_nome_de_evento("Post_Tool"));
        assert!(eh_hash_de_arquivo("0123456789ab"));
        assert!(!eh_hash_de_arquivo("0123456789AB") && !eh_hash_de_arquivo("0123456789a"));
    }

    #[test]
    fn nomes_de_projeto() {
        for bom in [
            "agenda-presidencial",
            "claude-pet",
            "Meu Projeto 2",
            "ação.v2",
            "acao\u{0301}", // decomposto
            "Documents",
            "日本",
        ] {
            assert!(eh_projeto(bom), "{bom}");
        }
        for ruim in [
            "",
            "a/b",
            "..",
            ".",
            "tab\tela",
            "aspas\"",
            "nova\nlinha",
            "barra\\invertida",
            &"x".repeat(65),
        ] {
            assert!(!eh_projeto(ruim), "{ruim:?}");
        }
        assert!(eh_projeto(&"ç".repeat(64)));
    }

    #[test]
    fn id_curto() {
        assert_eq!(curto("0b9e7c1d-aaaa-4bbb"), "0b9e7c1d");
        assert_eq!(curto("abc"), "abc");
    }
}
