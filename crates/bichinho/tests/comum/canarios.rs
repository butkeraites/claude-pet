//! Os casos dos testes canário, iguais para o `avisar.sh` (reserva até a
//! troca) e para o `bichinho avisar` (o hook nativo, decisão 0041): todo
//! campo de conteúdo carrega um `SEGREDO-…`, e nada com `segredo`, em caixa
//! nenhuma, pode sair.

#![allow(dead_code)] // cada arquivo de teste usa uma parte

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Os 13 eventos do plano (SubagentStop fica de fora de propósito).
pub const EVENTOS: [&str; 13] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "PermissionRequest",
    "Notification",
    "SubagentStart",
    "PreCompact",
    "PostCompact",
    "Stop",
    "StopFailure",
    "SessionEnd",
];
/// Todas as chaves que o fio v1 pode ter (o `term` desde a decisão 0054; o
/// `orig` e o `crn` desde a 0072; o `app` do macOS desde a 0105).
pub const CHAVES_DO_FIO: [&str; 27] = [
    "v", "e", "ts", "sid", "turno", "agente", "aid", "tool", "nt", "err", "src", "orig", "reason",
    "intr", "sha", "bg", "bgt", "bgi", "crn", "dur", "arq", "proj", "ent", "dnd", "teste", "term",
    "app",
];
pub const EDICAO: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];
pub const SID: &str = "3f0c2a9e-1d2b-4c5d-8e9f-0a1b2c3d4e5f";
pub const TURNO: &str = "9d8c7b6a-5f4e-4d3c-8b2a-1f0e9d8c7b6a";
/// Nada de `segredo`, em caixa nenhuma: um campo de conteúdo que passasse por
/// uma normalização (`ascii_downcase`) também tem de ser pego.
pub fn sem_segredo(onde: &str, texto: &str) {
    assert!(
        !texto.to_lowercase().contains("segredo"),
        "{onde}: vazou: {texto}"
    );
}

pub fn sha12(texto: &str) -> String {
    Sha256::digest(texto.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()[..12]
        .to_owned()
}

/// Campos comuns a todo hook, com segredos onde é conteúdo.
pub fn base(evento: &str) -> Value {
    json!({
        "session_id": SID,
        "prompt_id": TURNO,
        "transcript_path": "/home/SEGREDO-transcript/.claude/projects/SEGREDO-proj/sessao.jsonl",
        "cwd": "/home/SEGREDO-cwd-1/SEGREDO-cwd-2/meu-projeto",
        "permission_mode": "SEGREDO-modo",
        "hook_event_name": evento,
        "effort": {"level": "SEGREDO-esforco"},
        "campo_que_ainda_nem_existe": "SEGREDO-futuro",
        "outro_objeto_novo": {"texto": "SEGREDO-futuro-2"}
    })
}

pub fn com(evento: &str, extra: Value) -> Value {
    let mut v = base(evento);
    let mapa = v.as_object_mut().unwrap();
    for (k, valor) in extra.as_object().unwrap() {
        mapa.insert(k.clone(), valor.clone());
    }
    v
}

pub struct Caso {
    pub nome: &'static str,
    pub evento: &'static str,
    pub entrada: Value,
    /// Caminho editado (o hash dele tem de ir em `arq`).
    pub caminho: Option<&'static str>,
}

pub fn casos() -> Vec<Caso> {
    let caso = |nome, evento, extra| Caso {
        nome,
        evento,
        entrada: com(evento, extra),
        caminho: None,
    };
    let editado = |nome, extra, caminho| Caso {
        nome,
        evento: "PostToolUse",
        entrada: com("PostToolUse", extra),
        caminho: Some(caminho),
    };
    vec![
        caso(
            "SessionStart",
            "SessionStart",
            json!({"source": "startup", "model": "SEGREDO-modelo",
                   "session_title": "SEGREDO-titulo", "agent_type": "SEGREDO-agente"}),
        ),
        caso(
            "UserPromptSubmit",
            "UserPromptSubmit",
            json!({"prompt": "SEGREDO-prompt", "source": "user",
                   "session_title": "SEGREDO-titulo-sessao"}),
        ),
        caso(
            "PreToolUse pergunta",
            "PreToolUse",
            json!({"tool_name": "AskUserQuestion", "tool_use_id": "toolu_SEGREDO",
                   "tool_input": {"questions": [{"question": "SEGREDO-pergunta",
                                   "options": [{"label": "SEGREDO-opcao"}]}]}}),
        ),
        caso(
            "PreToolUse plano",
            "PreToolUse",
            json!({"tool_name": "ExitPlanMode", "tool_input": {"plan": "SEGREDO-plano"}}),
        ),
        editado(
            "PostToolUse Edit",
            json!({"tool_name": "Edit", "duration_ms": 37,
                   "tool_input": {"file_path": "/home/SEGREDO-caminho/meu-projeto/src/main.rs",
                                  "old_string": "SEGREDO-antes", "new_string": "SEGREDO-depois"},
                   "tool_response": {"filePath": "/home/SEGREDO-caminho/x",
                                     "structuredPatch": "SEGREDO-patch"}}),
            "/home/SEGREDO-caminho/meu-projeto/src/main.rs",
        ),
        editado(
            "PostToolUse Write",
            json!({"tool_name": "Write", "duration_ms": 12,
                   "tool_input": {"file_path": "/tmp/SEGREDO-novo.txt", "content": "SEGREDO-conteudo"},
                   "tool_response": "SEGREDO-resposta"}),
            "/tmp/SEGREDO-novo.txt",
        ),
        editado(
            "PostToolUse MultiEdit",
            json!({"tool_name": "MultiEdit",
                   "tool_input": {"file_path": "/srv/SEGREDO-multi.rs",
                                  "edits": [{"old_string": "SEGREDO-a", "new_string": "SEGREDO-b"}]}}),
            "/srv/SEGREDO-multi.rs",
        ),
        editado(
            "PostToolUse NotebookEdit",
            json!({"tool_name": "NotebookEdit",
                   "tool_input": {"notebook_path": "/home/SEGREDO-nb/analise.ipynb",
                                  "new_source": "SEGREDO-celula"}}),
            "/home/SEGREDO-nb/analise.ipynb",
        ),
        caso(
            "PostToolUse Bash",
            "PostToolUse",
            json!({"tool_name": "Bash", "duration_ms": 1830.7,
                   "tool_input": {"command": "echo SEGREDO-comando", "description": "SEGREDO-desc"},
                   "tool_response": {"stdout": "SEGREDO-saida", "stderr": "SEGREDO-erro"}}),
        ),
        caso(
            "PostToolUse Read",
            "PostToolUse",
            json!({"tool_name": "Read", "tool_input": {"file_path": "/etc/SEGREDO-lido"},
                   "tool_response": {"file": {"content": "SEGREDO-arquivo"}}}),
        ),
        caso(
            "PostToolUse de subagente",
            "PostToolUse",
            json!({"tool_name": "Grep", "agent_id": "agent-abc123", "agent_type": "SEGREDO-tipo",
                   "tool_input": {"pattern": "SEGREDO-padrao"}, "tool_response": "SEGREDO-achado"}),
        ),
        caso(
            "PostToolUseFailure Bash interrompido",
            "PostToolUseFailure",
            json!({"tool_name": "Bash", "tool_input": {"command": "SEGREDO-falhou"},
                   "error": "SEGREDO-texto-do-erro", "is_interrupt": true, "duration_ms": 5}),
        ),
        caso(
            "PostToolUseFailure Edit",
            "PostToolUseFailure",
            json!({"tool_name": "Edit", "tool_input": {"file_path": "/home/SEGREDO-falha.rs"},
                   "error": "erro_de_uma_palavra_so"}),
        ),
        caso(
            "PermissionRequest",
            "PermissionRequest",
            json!({"tool_name": "Bash", "tool_input": {"command": "rm -rf SEGREDO-alvo"},
                   "permission_suggestions": [{"type": "addRules",
                       "rules": [{"toolName": "Bash", "ruleContent": "SEGREDO-regra"}],
                       "behavior": "allow", "destination": "session"}]}),
        ),
        caso(
            "Notification permissão",
            "Notification",
            json!({"notification_type": "permission_prompt", "message": "SEGREDO-mensagem",
                   "title": "SEGREDO-titulo"}),
        ),
        caso(
            "Notification ociosa",
            "Notification",
            json!({"notification_type": "idle_prompt", "message": "SEGREDO-esperando"}),
        ),
        caso(
            "SubagentStart",
            "SubagentStart",
            json!({"agent_id": "agent-def456", "agent_type": "SEGREDO-agente"}),
        ),
        caso(
            "PreCompact",
            "PreCompact",
            json!({"trigger": "auto", "custom_instructions": "SEGREDO-instrucoes"}),
        ),
        caso(
            "PostCompact",
            "PostCompact",
            json!({"trigger": "manual", "compact_summary": "SEGREDO-resumo"}),
        ),
        caso(
            "Stop",
            "Stop",
            json!({"stop_hook_active": true, "last_assistant_message": "SEGREDO-resposta",
                   "background_tasks": [
                       {"id": "task-1", "type": "MCP task", "status": "running",
                        "description": "SEGREDO-tarefa", "server": "SEGREDO-servidor",
                        "tool": "SEGREDO-ferramenta"},
                       {"id": "task-2", "type": "auto-mode scan", "status": "pending",
                        "description": "SEGREDO-varredura"},
                       {"id": "task-3", "type": "shell", "status": "running",
                        "command": "npm run SEGREDO-dev", "description": "SEGREDO-servidor-dev"},
                       {"id": "task-4", "type": "SEGREDO Tarefa", "status": "running",
                        "description": "SEGREDO-tipo-novo"}],
                   "session_crons": [{"id": "c1", "schedule": "0 9 * * 1-5", "recurring": true,
                                      "prompt": "SEGREDO-cron"}]}),
        ),
        caso(
            "StopFailure",
            "StopFailure",
            json!({"error": "rate_limit", "error_details": "SEGREDO-detalhes",
                   "last_assistant_message": "SEGREDO-ultima"}),
        ),
        caso(
            "SessionEnd",
            "SessionEnd",
            json!({"reason": "prompt_input_exit"}),
        ),
    ]
}
