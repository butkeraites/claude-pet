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

use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::aprovacao::hex;
use crate::evento::{
    MAX_CONTAGEM, MAX_DURACAO_MS, MAX_FERRAMENTA, MAX_TAREFAS, eh_enum, eh_hash_de_arquivo, eh_id,
    eh_nome_de_evento, eh_origem, eh_projeto, eh_token,
};

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
/// (`CLAUDE_CODE_ENTRYPOINT`), o "não perturbe" e se é teste (`PET_TESTE=1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contexto<'a> {
    pub evento: &'a str,
    /// Relógio do host no início do hook, em ms desde 1970.
    pub ts: Option<u64>,
    pub ent: Option<&'a str>,
    pub dnd: bool,
    pub teste: bool,
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

/// O caminho editado, só no PostToolUse de uma ferramenta de edição:
/// `tool_input.file_path`, ou `notebook_path` se ele faltar (o `//` do jq).
/// Nunca sai daqui: vira [`hash_do_caminho`].
pub fn caminho_editado<'a>(evento: &str, entrada: &'a Value) -> Option<&'a str> {
    if evento != "PostToolUse" {
        return None;
    }
    let ferramenta = entrada.get("tool_name")?.as_str()?;
    if !FERRAMENTAS_DE_EDICAO.contains(&ferramenta) {
        return None;
    }
    let entrada = entrada.get("tool_input")?.as_object()?;
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

/// Só o nome da última pasta do `cwd`, nunca o caminho.
fn pasta(v: Option<&Value>) -> Option<String> {
    let cwd = v?.as_str()?;
    let ultima = cwd.split('/').rfind(|p| !p.is_empty())?;
    eh_projeto(ultima).then(|| ultima.to_owned())
}

/// Monta o corpo do fio v1 a partir do JSON do hook. `arq` é o hash do
/// caminho editado ([`hash_do_caminho`]), calculado por quem chama. Cada
/// campo só vale no evento que o tem e passa pelo validador do pet; nenhum
/// outro campo do hook é lido.
pub fn montar(entrada: &Map<String, Value>, ctx: &Contexto, arq: Option<&str>) -> Fio {
    let e = ctx.evento;
    let campo = |nome: &str| entrada.get(nome);
    let de_ferramenta = matches!(
        e,
        "PreToolUse" | "PostToolUse" | "PostToolUseFailure" | "PermissionRequest"
    );
    let tarefas: &[Value] = match campo("background_tasks") {
        Some(Value::Array(lista)) => lista,
        _ => &[],
    };
    let validas: Vec<(String, String)> = tarefas
        .iter()
        .filter(|t| t.is_object())
        .filter_map(|t| Some((tipo_de_tarefa(t.get("type"))?, texto(t.get("id"), eh_id)?)))
        .take(MAX_TAREFAS)
        .collect();
    let stop = e == "Stop";
    let lista_de_tarefas = matches!(campo("background_tasks"), Some(Value::Array(_)));
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
        bg: (stop && lista_de_tarefas)
            .then_some(tarefas.len() as u64)
            .filter(|&n| n <= MAX_CONTAGEM),
        bgt: (stop && !validas.is_empty()).then(|| validas.iter().map(|t| t.0.clone()).collect()),
        bgi: (stop && !validas.is_empty()).then(|| validas.iter().map(|t| t.1.clone()).collect()),
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
    }
}

/// O corpo inteiro, do jeito que vai no POST: o JSON do hook vira o fio v1;
/// o que não é um objeto JSON vira o [`minimo`]. `null` vale como objeto
/// vazio (como no jq).
pub fn corpo(entrada: &[u8], ctx: &Contexto) -> String {
    let valor: Value = match serde_json::from_slice(entrada) {
        Ok(valor) => valor,
        Err(_) => return minimo(ctx.evento, ctx.teste),
    };
    let vazio = Map::new();
    let objeto = match &valor {
        Value::Object(objeto) => objeto,
        Value::Null => &vazio,
        _ => return minimo(ctx.evento, ctx.teste),
    };
    let arq = caminho_editado(ctx.evento, &valor).map(hash_do_caminho);
    let fio = montar(objeto, ctx, arq.as_deref());
    serde_json::to_string(&fio).unwrap_or_else(|_| minimo(ctx.evento, ctx.teste))
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
        }
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

    #[test]
    fn hash_e_caminho_so_da_edicao() {
        assert_eq!(hash_do_caminho("/tmp/x"), {
            let h = hex(&Sha256::digest(b"/tmp/x"));
            h[..12].to_owned()
        });
        let edit = json!({"tool_name": "Edit", "tool_input": {"file_path": "/a/b.rs"}});
        assert_eq!(caminho_editado("PostToolUse", &edit), Some("/a/b.rs"));
        assert_eq!(caminho_editado("PreToolUse", &edit), None);
        let nb = json!({"tool_name": "NotebookEdit",
                        "tool_input": {"file_path": null, "notebook_path": "/n.ipynb"}});
        assert_eq!(caminho_editado("PostToolUse", &nb), Some("/n.ipynb"));
        let lido = json!({"tool_name": "Read", "tool_input": {"file_path": "/etc/x"}});
        assert_eq!(caminho_editado("PostToolUse", &lido), None);
        let ruim = json!({"tool_name": "Write", "tool_input": "SEGREDO"});
        assert_eq!(caminho_editado("PostToolUse", &ruim), None);
        let vazio = json!({"tool_name": "Write", "tool_input": {"file_path": ""}});
        assert_eq!(caminho_editado("PostToolUse", &vazio), None);
    }

    #[test]
    fn campos_so_no_evento_que_os_tem() {
        let entrada = json!({
            "session_id": "s1", "prompt_id": "p1", "tool_name": "Bash",
            "notification_type": "idle_prompt", "error": "rate_limit", "source": "user",
            "reason": "logout", "is_interrupt": true, "stop_hook_active": true,
            "duration_ms": 12.9, "background_tasks": [{"id": "t1", "type": "shell"}],
            "cwd": "/home/x/meu-projeto", "prompt": "SEGREDO"
        });
        let stop = montado("Stop", entrada.clone());
        assert_eq!(stop["sha"], true);
        assert_eq!(
            (stop["bg"].clone(), stop["bgt"].clone()),
            (json!(1), json!(["shell"]))
        );
        for ausente in ["tool", "nt", "err", "src", "reason", "intr", "dur", "arq"] {
            assert!(stop.get(ausente).is_none(), "{ausente} em {stop}");
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
