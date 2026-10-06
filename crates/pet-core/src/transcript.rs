//! Classificador puro de linhas de transcript do Claude Code para o
//! observador do daemon (decisão 0108): dá ao Zeca os eventos de uma sessão
//! que o hook ainda não cobre (aberta antes do plugin), lendo
//! `~/.claude/projects/*/*.jsonl` em vez do hook.
//!
//! Aqui é só a classificação **pura**: uma linha já parseada (`serde_json::
//! Value`) vira, se for o caso, um [`Evento`] do fio v1. A descoberta e o
//! seguimento dos arquivos são do daemon (IO).
//!
//! **Privacidade:** só os metadados do fio saem — `sessionId`, `promptId`, a
//! última pasta do `cwd`, o `entrypoint` e a forma do turno. O texto do prompt
//! e da resposta (`message.content`) nunca é lido para fora: este módulo olha
//! o `type` da linha e, no máximo, o `type` do primeiro bloco do conteúdo
//! (para separar um prompt de um `tool_result`), nunca o texto.

use serde_json::Value;

use crate::evento::{Evento, ORIG_COMUM, ORIG_NOTIFICACAO, eh_projeto};

/// A última pasta do `cwd` (nunca o caminho), como o fio (decisão 0019); corta
/// nas duas barras (no Windows o `cwd` vem com `\`).
pub fn proj_do_cwd(cwd: &str) -> Option<String> {
    let ultima = cwd.rsplit(['/', '\\']).find(|p| !p.is_empty())?;
    eh_projeto(ultima).then(|| ultima.to_owned())
}

/// O `sessionId` da linha.
pub fn sid(linha: &Value) -> Option<&str> {
    linha.get("sessionId").and_then(Value::as_str)
}

/// O `entrypoint` da linha (`cli`, `sdk-cli`, …).
pub fn entrypoint(linha: &Value) -> Option<&str> {
    linha.get("entrypoint").and_then(Value::as_str)
}

/// A forma do turno a partir do `turnOrigin` (decisão 0072): um turno humano é
/// [`ORIG_COMUM`]; um agendamento (`/loop`) ou uma notificação de tarefa é de
/// máquina ([`ORIG_NOTIFICACAO`]), e o pet não comemora.
fn orig_do_turno(linha: &Value) -> &'static str {
    match linha.get("turnOrigin").and_then(Value::as_str) {
        Some("scheduled" | "task_notification") => ORIG_NOTIFICACAO,
        _ => ORIG_COMUM,
    }
}

/// A linha `user` é um prompt de verdade (não um `tool_result`, que também tem
/// role user)? Olha só o `type` do conteúdo, nunca o texto.
pub fn eh_prompt(linha: &Value) -> bool {
    if linha.get("type").and_then(Value::as_str) != Some("user") {
        return false;
    }
    match linha.pointer("/message/content") {
        Some(Value::String(_)) => true,
        Some(Value::Array(blocos)) => {
            blocos
                .first()
                .and_then(|b| b.get("type"))
                .and_then(Value::as_str)
                != Some("tool_result")
        }
        _ => false,
    }
}

/// A linha `assistant` fecha o turno (Claude terminou): `stop_reason ==
/// "end_turn"` (um `tool_use` é meio de turno).
pub fn eh_fim_de_turno(linha: &Value) -> bool {
    linha.get("type").and_then(Value::as_str) == Some("assistant")
        && linha
            .pointer("/message/stop_reason")
            .and_then(Value::as_str)
            == Some("end_turn")
}

fn texto<'a>(linha: &'a Value, chave: &str) -> Option<&'a str> {
    linha.get(chave).and_then(Value::as_str)
}

/// Classifica uma linha num [`Evento`] do fio, se ela for um prompt ou um fim
/// de turno; senão `None`. O `ts` (o relógio) fica para o daemon pôr.
pub fn classificar(linha: &Value) -> Option<Evento> {
    let sid = sid(linha)?.to_owned();
    let ent = entrypoint(linha).map(str::to_owned);
    let proj = texto(linha, "cwd").and_then(proj_do_cwd);
    let turno = texto(linha, "promptId").map(str::to_owned);
    if eh_prompt(linha) {
        Some(Evento {
            e: "UserPromptSubmit".into(),
            sid: Some(sid),
            turno,
            orig: Some(orig_do_turno(linha).to_owned()),
            proj,
            ent,
            ..Evento::default()
        })
    } else if eh_fim_de_turno(linha) {
        Some(Evento {
            e: "Stop".into(),
            sid: Some(sid),
            turno,
            proj,
            ent,
            ..Evento::default()
        })
    } else {
        None
    }
}

/// Um `SessionStart` para o daemon semear a consciência de uma sessão achada
/// aberta (o estado dela vem dos prompts/fins de turno seguintes).
pub fn inicio(sid: &str, cwd: Option<&str>, ent: Option<&str>) -> Evento {
    Evento {
        e: "SessionStart".into(),
        sid: Some(sid.to_owned()),
        proj: cwd.and_then(proj_do_cwd),
        ent: ent.map(str::to_owned),
        ..Evento::default()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn prompt_humano_digitado_vira_userpromptsubmit_comum() {
        let linha = json!({
            "type": "user",
            "sessionId": "d1a3458c-0000",
            "promptId": "p7",
            "cwd": "/Users/x/dev/claude-pet",
            "entrypoint": "cli",
            "turnOrigin": "human",
            "message": {"role": "user", "content": "texto que NÃO deve ser lido"}
        });
        let e = classificar(&linha).expect("é um prompt");
        assert_eq!(e.e, "UserPromptSubmit");
        assert_eq!(e.sid.as_deref(), Some("d1a3458c-0000"));
        assert_eq!(e.turno.as_deref(), Some("p7"));
        assert_eq!(e.proj.as_deref(), Some("claude-pet"));
        assert_eq!(e.ent.as_deref(), Some("cli"));
        assert_eq!(e.orig.as_deref(), Some(ORIG_COMUM));
    }

    #[test]
    fn prompt_de_notificacao_de_tarefa_e_de_maquina() {
        let linha = json!({
            "type": "user", "sessionId": "s1", "promptId": "p1",
            "turnOrigin": "task_notification",
            "message": {"role": "user", "content": "..."}
        });
        let e = classificar(&linha).expect("é um prompt");
        assert_eq!(e.orig.as_deref(), Some(ORIG_NOTIFICACAO));
    }

    #[test]
    fn loop_agendado_e_de_maquina() {
        let linha = json!({
            "type": "user", "sessionId": "s1", "turnOrigin": "scheduled",
            "message": {"role": "user", "content": "..."}
        });
        assert_eq!(
            classificar(&linha).unwrap().orig.as_deref(),
            Some(ORIG_NOTIFICACAO)
        );
    }

    #[test]
    fn tool_result_nao_e_prompt() {
        let linha = json!({
            "type": "user", "sessionId": "s1",
            "message": {"role": "user", "content": [{"type": "tool_result", "content": "saída"}]}
        });
        assert!(classificar(&linha).is_none());
        assert!(!eh_prompt(&linha));
    }

    #[test]
    fn assistant_end_turn_vira_stop() {
        let linha = json!({
            "type": "assistant", "sessionId": "s1", "promptId": "p1",
            "cwd": "/home/x/proj", "entrypoint": "cli",
            "message": {"role": "assistant", "stop_reason": "end_turn", "content": []}
        });
        let e = classificar(&linha).expect("fim de turno");
        assert_eq!(e.e, "Stop");
        assert_eq!(e.sid.as_deref(), Some("s1"));
        assert_eq!(e.turno.as_deref(), Some("p1"));
        assert_eq!(e.proj.as_deref(), Some("proj"));
    }

    #[test]
    fn assistant_tool_use_nao_e_fim_de_turno() {
        let linha = json!({
            "type": "assistant", "sessionId": "s1",
            "message": {"stop_reason": "tool_use", "content": []}
        });
        assert!(classificar(&linha).is_none());
        assert!(!eh_fim_de_turno(&linha));
    }

    #[test]
    fn proj_corta_o_caminho_e_recusa_barra() {
        assert_eq!(
            proj_do_cwd("/Users/x/dev/claude-pet"),
            Some("claude-pet".into())
        );
        assert_eq!(proj_do_cwd("C:\\Users\\x\\projeto"), Some("projeto".into()));
        assert_eq!(proj_do_cwd("/"), None);
    }

    #[test]
    fn inicio_e_um_sessionstart_so_com_metadados() {
        let e = inicio("s1", Some("/home/x/proj"), Some("cli"));
        assert_eq!(e.e, "SessionStart");
        assert_eq!(e.sid.as_deref(), Some("s1"));
        assert_eq!(e.proj.as_deref(), Some("proj"));
        assert_eq!(e.ent.as_deref(), Some("cli"));
        assert_eq!(e.turno, None);
    }

    #[test]
    fn linha_sem_sessao_nao_classifica() {
        assert!(classificar(&json!({"type": "user", "message": {"content": "x"}})).is_none());
    }
}
