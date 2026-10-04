//! Testes canário do `plugin/scripts/avisar.sh` (decisões 0009 e 0019) e o
//! contrato do `hooks.json`.
//!
//! Cada evento ligado no `hooks.json` roda com um JSON de hook em que todo
//! campo de conteúdo (prompt, `tool_input` menos o caminho, `tool_response`,
//! texto de erro, mensagem, títulos, última resposta, transcript, pastas
//! acima do projeto, campos que ainda nem existem) carrega um `SEGREDO-…`.
//! Um `curl` falso na frente do PATH guarda o corpo e os argumentos. Nada com
//! `segredo`, em maiúsculas ou minúsculas, pode sair, o script sai sempre 0
//! e não imprime nada. Com o curl de verdade, nem proxy nem `~/.curlrc`
//! desviam o evento do 127.0.0.1, e um `~/.jq` não muda a lista branca
//! (decisão 0031).

mod comum;

use std::collections::BTreeSet;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Os 13 eventos do plano (SubagentStop fica de fora de propósito).
const EVENTOS: [&str; 13] = [
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
/// Todas as chaves que o fio v1 pode ter.
const CHAVES_DO_FIO: [&str; 23] = [
    "v", "e", "ts", "sid", "turno", "agente", "aid", "tool", "nt", "err", "src", "reason", "intr",
    "sha", "bg", "bgt", "bgi", "dur", "arq", "proj", "ent", "dnd", "teste",
];
const EDICAO: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];
const SID: &str = "3f0c2a9e-1d2b-4c5d-8e9f-0a1b2c3d4e5f";
const TURNO: &str = "9d8c7b6a-5f4e-4d3c-8b2a-1f0e9d8c7b6a";
const SH: &str = "/bin/sh";

fn script() -> PathBuf {
    comum::raiz().join("plugin/scripts/avisar.sh")
}

/// Pasta com o `curl` falso e os PATHs mutilados (sem jq, sem curl).
struct Banca {
    pasta: PathBuf,
}

struct Rodada {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    corpo: Option<String>,
    args: Option<String>,
    duracao: Duration,
}

fn achar_no_sistema(programa: &str) -> PathBuf {
    ["/usr/bin", "/bin"]
        .iter()
        .map(|p| Path::new(p).join(programa))
        .find(|p| p.exists())
        .unwrap_or_else(|| panic!("{programa} não está no sistema"))
}

impl Banca {
    fn nova() -> Banca {
        let pasta = comum::pasta_temporaria("avisar");
        let falsos = pasta.join("falsos");
        std::fs::create_dir_all(&falsos).unwrap();
        let curl = falsos.join("curl");
        std::fs::write(
            &curl,
            "#!/bin/sh\ncat > \"$CAPTURA/corpo\"\nprintf '%s\\n' \"$@\" > \"$CAPTURA/args\"\nexit \"${CURL_FALSO_SAI:-0}\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&curl, std::fs::Permissions::from_mode(0o755)).unwrap();
        for (nome, programas) in [
            ("sem-jq", &["date", "cat", "cut", "sha256sum"][..]),
            ("sem-curl", &["date", "cat", "cut", "sha256sum", "jq"][..]),
        ] {
            let dir = pasta.join(nome);
            std::fs::create_dir_all(&dir).unwrap();
            for programa in programas {
                std::os::unix::fs::symlink(achar_no_sistema(programa), dir.join(programa)).unwrap();
            }
        }
        std::fs::create_dir_all(pasta.join("home")).unwrap();
        std::fs::create_dir_all(pasta.join("captura")).unwrap();
        Banca { pasta }
    }

    fn path_normal(&self) -> String {
        format!("{}:/usr/bin:/bin", self.pasta.join("falsos").display())
    }

    /// Roda `sh avisar.sh <evento>` com `entrada` no stdin. O ambiente é
    /// limpo: PATH com o curl falso na frente, HOME próprio,
    /// `CLAUDE_CODE_ENTRYPOINT=cli`, porta 9 (ninguém escuta).
    fn rodar(&self, evento: &str, entrada: &[u8], ajuste: impl FnOnce(&mut Command)) -> Rodada {
        let captura = self.pasta.join("captura");
        for nome in ["corpo", "args"] {
            let _ = std::fs::remove_file(captura.join(nome));
        }
        let mut cmd = Command::new(SH);
        cmd.arg(script())
            .arg(evento)
            .env_clear()
            .env("PATH", self.path_normal())
            .env("HOME", self.pasta.join("home"))
            .env("CAPTURA", &captura)
            .env("CLAUDE_CODE_ENTRYPOINT", "cli")
            .env("PET_PORTA", "9")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        ajuste(&mut cmd);
        let inicio = Instant::now();
        let mut filho = cmd.spawn().expect("rodar o sh");
        let mut stdin = filho.stdin.take().unwrap();
        // O script pode nem ler a entrada (sem jq): erro de pipe aqui é normal.
        let _ = stdin.write_all(entrada);
        drop(stdin);
        let saida = filho.wait_with_output().unwrap();
        Rodada {
            status: saida.status.code(),
            stdout: saida.stdout,
            stderr: saida.stderr,
            corpo: std::fs::read_to_string(captura.join("corpo")).ok(),
            args: std::fs::read_to_string(captura.join("args")).ok(),
            duracao: inicio.elapsed(),
        }
    }
}

impl Drop for Banca {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.pasta);
    }
}

impl Rodada {
    /// Saiu 0 e não imprimiu nada.
    fn calada(&self, caso: &str) {
        assert_eq!(self.status, Some(0), "{caso}: código de saída");
        assert!(
            self.stdout.is_empty(),
            "{caso}: stdout {:?}",
            String::from_utf8_lossy(&self.stdout)
        );
        assert!(
            self.stderr.is_empty(),
            "{caso}: stderr {:?}",
            String::from_utf8_lossy(&self.stderr)
        );
    }

    fn corpo_json(&self, caso: &str) -> Value {
        let corpo = self
            .corpo
            .as_deref()
            .unwrap_or_else(|| panic!("{caso}: o curl não foi chamado"));
        serde_json::from_str(corpo).unwrap_or_else(|e| panic!("{caso}: corpo não é JSON ({e})"))
    }
}

/// Nada de `segredo`, em caixa nenhuma: um campo de conteúdo que passasse por
/// uma normalização (`ascii_downcase`) também tem de ser pego.
fn sem_segredo(onde: &str, texto: &str) {
    assert!(
        !texto.to_lowercase().contains("segredo"),
        "{onde}: vazou: {texto}"
    );
}

fn sha12(texto: &str) -> String {
    Sha256::digest(texto.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()[..12]
        .to_owned()
}

/// Campos comuns a todo hook, com segredos onde é conteúdo.
fn base(evento: &str) -> Value {
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

fn com(evento: &str, extra: Value) -> Value {
    let mut v = base(evento);
    let mapa = v.as_object_mut().unwrap();
    for (k, valor) in extra.as_object().unwrap() {
        mapa.insert(k.clone(), valor.clone());
    }
    v
}

struct Caso {
    nome: &'static str,
    evento: &'static str,
    entrada: Value,
    /// Caminho editado (o hash dele tem de ir em `arq`).
    caminho: Option<&'static str>,
}

fn casos() -> Vec<Caso> {
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

#[test]
fn nenhum_segredo_sai_em_evento_nenhum() {
    let banca = Banca::nova();
    let casos = casos();
    let cobertos: BTreeSet<&str> = casos.iter().map(|c| c.evento).collect();
    assert_eq!(
        cobertos,
        EVENTOS.into_iter().collect(),
        "um caso por evento"
    );
    for caso in &casos {
        let nome = caso.nome;
        let r = banca.rodar(caso.evento, caso.entrada.to_string().as_bytes(), |_| {});
        r.calada(nome);
        let bruto = r.corpo.clone().unwrap_or_default();
        sem_segredo(&format!("{nome}, corpo"), &bruto);
        let args = r.args.clone().unwrap_or_default();
        sem_segredo(&format!("{nome}, argumentos do curl"), &args);
        let corpo = r.corpo_json(nome);
        assert_eq!(corpo["v"], 1, "{nome}");
        assert_eq!(corpo["e"], caso.evento, "{nome}");
        assert_eq!(corpo["sid"], SID, "{nome}");
        assert_eq!(corpo["turno"], TURNO, "{nome}");
        assert_eq!(corpo["proj"], "meu-projeto", "{nome}: só a última pasta");
        assert_eq!(corpo["ent"], "cli", "{nome}");
        assert_eq!(corpo["dnd"], false, "{nome}");
        assert!(corpo.get("teste").is_none(), "{nome}: teste sem PET_TESTE");
        for chave in corpo.as_object().unwrap().keys() {
            assert!(
                CHAVES_DO_FIO.contains(&chave.as_str()),
                "{nome}: chave fora do fio: {chave}"
            );
        }
        let ferramenta = caso.entrada["tool_name"].as_str().unwrap_or_default();
        let editou = caso.evento == "PostToolUse" && EDICAO.contains(&ferramenta);
        assert_eq!(editou, caso.caminho.is_some(), "{nome}: tabela de casos");
        match caso.caminho {
            Some(caminho) => assert_eq!(corpo["arq"], sha12(caminho), "{nome}: arq"),
            None => assert!(
                corpo.get("arq").is_none(),
                "{nome}: arq sem edição: {corpo}"
            ),
        }
        // O daemon aceita o corpo inteiro, sem descartar nada.
        let lido = pet_core::evento::ler(bruto.as_bytes())
            .unwrap_or_else(|e| panic!("{nome}: o daemon recusaria: {e}"));
        assert!(
            lido.descartados.is_empty(),
            "{nome}: {:?}",
            lido.descartados
        );
    }
}

#[test]
fn campos_de_cada_evento() {
    let banca = Banca::nova();
    let rodar = |nome: &str| {
        let caso = casos().into_iter().find(|c| c.nome == nome).unwrap();
        let r = banca.rodar(caso.evento, caso.entrada.to_string().as_bytes(), |_| {});
        r.calada(nome);
        r.corpo_json(nome)
    };
    let stop = rodar("Stop");
    assert_eq!(stop["sha"], true);
    assert_eq!(stop["bg"], 4);
    // Os rótulos do Claude Code normalizados, por lista fechada: um tipo
    // que ela não conhece vira `outro`, nunca o texto dele.
    assert_eq!(
        stop["bgt"],
        json!(["mcp_task", "auto_mode_scan", "shell", "outro"])
    );
    assert_eq!(stop["bgi"], json!(["task-1", "task-2", "task-3", "task-4"]));

    let bash = rodar("PostToolUse Bash");
    assert_eq!(bash["tool"], "Bash");
    assert_eq!(bash["dur"], 1830, "duração arredondada para baixo");
    assert!(bash.get("agente").is_none());

    let sub = rodar("PostToolUse de subagente");
    assert_eq!(
        (sub["agente"].clone(), sub["aid"].clone()),
        (json!(true), json!("agent-abc123"))
    );

    let falha = rodar("PostToolUseFailure Bash interrompido");
    assert_eq!(
        (falha["intr"].clone(), falha["dur"].clone()),
        (json!(true), json!(5))
    );
    let falha_edit = rodar("PostToolUseFailure Edit");
    assert!(
        falha_edit.get("err").is_none(),
        "texto de erro de ferramenta nunca vira err"
    );

    assert_eq!(rodar("StopFailure")["err"], "rate_limit");
    assert_eq!(rodar("SessionEnd")["reason"], "prompt_input_exit");
    assert_eq!(rodar("SessionStart")["src"], "startup");
    assert_eq!(rodar("UserPromptSubmit")["src"], "user");
    assert_eq!(rodar("Notification permissão")["nt"], "permission_prompt");
    assert_eq!(rodar("PreToolUse plano")["tool"], "ExitPlanMode");
    let sub_inicio = rodar("SubagentStart");
    assert_eq!(sub_inicio["aid"], "agent-def456");

    let ts = rodar("SessionEnd")["ts"].as_u64().expect("ts em ms");
    let agora = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(
        agora.abs_diff(ts) < 60_000,
        "ts {ts} longe de agora {agora}"
    );
}

#[test]
fn teste_so_com_pet_teste_1() {
    let banca = Banca::nova();
    let entrada = com("Stop", json!({})).to_string();
    for (valor, esperado) in [
        (Some("1"), Some(true)),
        (Some("0"), None),
        (Some("sim"), None),
        (None, None),
    ] {
        let r = banca.rodar("Stop", entrada.as_bytes(), |c| {
            if let Some(v) = valor {
                c.env("PET_TESTE", v);
            }
        });
        r.calada("PET_TESTE");
        let corpo = r.corpo_json("PET_TESTE");
        assert_eq!(
            corpo.get("teste").and_then(Value::as_bool),
            esperado,
            "{valor:?}"
        );
    }
}

#[test]
fn nao_perturbe_do_omarchy() {
    let banca = Banca::nova();
    let entrada = com("Stop", json!({})).to_string();
    let estado = banca.pasta.join("estado-xdg");
    let arquivo = estado.join("omarchy/notifications.json");
    std::fs::create_dir_all(arquivo.parent().unwrap()).unwrap();
    for (conteudo, esperado) in [
        (Some(r#"{"dnd":true}"#), true),
        (Some(r#"{"dnd":false}"#), false),
        (Some(r#"{"dnd":"true"}"#), false),
        (Some("não é json"), false),
        (
            Some(r#"{"dnd":true,"past":[{"body":"SEGREDO-notificacao"}]}"#),
            true,
        ),
        (None, false),
    ] {
        match conteudo {
            Some(c) => std::fs::write(&arquivo, c).unwrap(),
            None => {
                let _ = std::fs::remove_file(&arquivo);
            }
        }
        let r = banca.rodar("Stop", entrada.as_bytes(), |c| {
            c.env("XDG_STATE_HOME", &estado);
        });
        r.calada("dnd");
        assert_eq!(r.corpo_json("dnd")["dnd"], esperado, "{conteudo:?}");
        sem_segredo("dnd", &r.corpo.unwrap());
    }
    // Sem XDG_STATE_HOME vale ~/.local/state.
    let padrao = banca
        .pasta
        .join("home/.local/state/omarchy/notifications.json");
    std::fs::create_dir_all(padrao.parent().unwrap()).unwrap();
    std::fs::write(&padrao, r#"{"dnd":true}"#).unwrap();
    let r = banca.rodar("Stop", entrada.as_bytes(), |_| {});
    assert_eq!(r.corpo_json("dnd padrão")["dnd"], true);
}

#[test]
fn curl_falhando_sai_0_calado() {
    let banca = Banca::nova();
    let entrada = com("Stop", json!({})).to_string();
    for codigo in ["7", "28", "1"] {
        let r = banca.rodar("Stop", entrada.as_bytes(), |c| {
            c.env("CURL_FALSO_SAI", codigo);
        });
        r.calada(&format!("curl saindo {codigo}"));
        assert!(r.corpo.is_some(), "o curl foi chamado");
    }
}

#[test]
fn sem_jq_manda_o_minimo() {
    let banca = Banca::nova();
    let caminho = format!(
        "{}:{}",
        banca.pasta.join("falsos").display(),
        banca.pasta.join("sem-jq").display()
    );
    let entrada = com("Stop", json!({})).to_string();
    let r = banca.rodar("Stop", entrada.as_bytes(), |c| {
        c.env("PATH", &caminho);
    });
    r.calada("sem jq");
    assert_eq!(r.corpo.as_deref(), Some(r#"{"v":1,"e":"Stop"}"#));
    let r = banca.rodar("Stop", entrada.as_bytes(), |c| {
        c.env("PATH", &caminho).env("PET_TESTE", "1");
    });
    r.calada("sem jq, teste");
    assert_eq!(
        r.corpo.as_deref(),
        Some(r#"{"v":1,"e":"Stop","teste":true}"#)
    );
}

#[test]
fn sem_curl_sai_0_calado() {
    let banca = Banca::nova();
    let caminho = banca.pasta.join("sem-curl").display().to_string();
    let entrada = com("Stop", json!({})).to_string();
    let r = banca.rodar("Stop", entrada.as_bytes(), |c| {
        c.env("PATH", &caminho);
    });
    r.calada("sem curl");
    assert!(r.corpo.is_none());
}

#[test]
fn entrada_que_nao_e_json_manda_o_minimo() {
    let banca = Banca::nova();
    for entrada in [
        &b""[..],
        b"nao e json SEGREDO-1",
        b"[1, 2, \"SEGREDO-2\"]",
        b"\"SEGREDO-3\"",
        b"{\"session_id\": \"SEGREDO-4",
        b"\xff\xfe SEGREDO-5",
    ] {
        let r = banca.rodar("Stop", entrada, |_| {});
        r.calada("entrada ruim");
        assert_eq!(
            r.corpo.as_deref(),
            Some(r#"{"v":1,"e":"Stop"}"#),
            "{:?}",
            String::from_utf8_lossy(entrada)
        );
    }
}

#[test]
fn campos_com_tipo_errado_caem() {
    let banca = Banca::nova();
    let entrada = json!({
        "session_id": "tem espaço SEGREDO-1",
        "prompt_id": 42,
        "cwd": "/",
        "tool_name": ["Edit"],
        "duration_ms": -3,
        "stop_hook_active": "true",
        "background_tasks": "SEGREDO-2",
        "notification_type": "Permission Prompt SEGREDO-3"
    });
    for evento in ["Stop", "PostToolUse", "Notification"] {
        let r = banca.rodar(evento, entrada.to_string().as_bytes(), |_| {});
        r.calada(evento);
        let corpo = r.corpo_json(evento);
        let chaves: BTreeSet<&str> = corpo
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            chaves,
            BTreeSet::from(["v", "e", "ts", "ent", "dnd"]),
            "{evento}: {corpo}"
        );
    }
}

#[test]
fn nome_de_evento_invalido_nao_manda_nada() {
    let banca = Banca::nova();
    let longo = "A".repeat(41);
    for nome in ["", "Stop;rm", "Stop Stop", "Stóp", "Stop\"", longo.as_str()] {
        let r = banca.rodar(nome, b"{}", |_| {});
        r.calada(nome);
        assert!(r.corpo.is_none(), "{nome:?} não deveria chamar o curl");
    }
    // Sem argumento nenhum.
    let mut cmd = Command::new(SH);
    let saida = cmd
        .arg(script())
        .env_clear()
        .env("PATH", banca.path_normal())
        .env("CAPTURA", banca.pasta.join("captura"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(saida.status.code(), Some(0));
    assert!(saida.stdout.is_empty() && saida.stderr.is_empty());
}

#[test]
fn porta_e_cabecalhos_do_curl() {
    let banca = Banca::nova();
    let entrada = com("Stop", json!({})).to_string();
    for (porta, url) in [
        (Some("28001"), "http://127.0.0.1:28001/v1/evento"),
        (Some("abc"), "http://127.0.0.1:27380/v1/evento"),
        (Some("123456"), "http://127.0.0.1:27380/v1/evento"),
        (Some("1:2@evil.com"), "http://127.0.0.1:27380/v1/evento"),
        (None, "http://127.0.0.1:27380/v1/evento"),
    ] {
        let r = banca.rodar("Stop", entrada.as_bytes(), |c| match porta {
            Some(p) => {
                c.env("PET_PORTA", p);
            }
            None => {
                c.env_remove("PET_PORTA");
            }
        });
        r.calada("porta");
        let args: Vec<String> = r.args.unwrap().lines().map(str::to_owned).collect();
        // `-q` primeiro (nenhum curlrc) e `--noproxy '*'` (nenhum proxy do
        // ambiente): o evento só vai ao 127.0.0.1 (decisão 0031).
        assert_eq!(
            args,
            vec![
                "-q",
                "--noproxy",
                "*",
                "-sS",
                "-m",
                "2",
                "-o",
                "/dev/null",
                "-H",
                "Content-Type: application/json",
                "-H",
                "X-Pet: 1",
                "--data-binary",
                "@-",
                url
            ],
            "{porta:?}"
        );
    }
}

#[test]
fn origem_valida_ou_nada() {
    let banca = Banca::nova();
    let entrada = com("Stop", json!({})).to_string();
    for (ent, esperado) in [
        (Some("sdk-cli"), Some("sdk-cli")),
        (Some("claude-vscode"), Some("claude-vscode")),
        (Some("CLI"), None),
        (Some("cli; SEGREDO"), None),
        (None, None),
    ] {
        let r = banca.rodar("Stop", entrada.as_bytes(), |c| match ent {
            Some(e) => {
                c.env("CLAUDE_CODE_ENTRYPOINT", e);
            }
            None => {
                c.env_remove("CLAUDE_CODE_ENTRYPOINT");
            }
        });
        r.calada("ent");
        let corpo = r.corpo_json("ent");
        assert_eq!(
            corpo.get("ent").and_then(Value::as_str),
            esperado,
            "{ent:?}"
        );
    }
}

#[test]
fn hooks_json_tem_os_13_eventos_async_e_calados() {
    let texto = std::fs::read_to_string(comum::raiz().join("plugin/hooks/hooks.json")).unwrap();
    let hooks: Value = serde_json::from_str(&texto).unwrap();
    let mapa = hooks["hooks"].as_object().expect("hooks");
    let nomes: BTreeSet<&str> = mapa.keys().map(String::as_str).collect();
    assert_eq!(nomes, EVENTOS.into_iter().collect());
    assert!(
        !mapa.contains_key("SubagentStop"),
        "subagente nunca dispara festa"
    );
    for (evento, grupos) in mapa {
        let grupos = grupos.as_array().unwrap();
        assert_eq!(grupos.len(), 1, "{evento}");
        let matcher = grupos[0].get("matcher").and_then(Value::as_str);
        let esperado = match evento.as_str() {
            "PreToolUse" => Some("AskUserQuestion|ExitPlanMode"),
            "Notification" => Some(
                "permission_prompt|worker_permission_prompt|elicitation_dialog|elicitation_url_dialog|agent_needs_input|idle_prompt",
            ),
            _ => None,
        };
        assert_eq!(matcher, esperado, "{evento}");
        let lista = grupos[0]["hooks"].as_array().unwrap();
        assert_eq!(lista.len(), 1, "{evento}");
        let hook = &lista[0];
        assert_eq!(hook["type"], "command", "{evento}");
        assert_eq!(hook["async"], true, "{evento}");
        assert_eq!(
            hook["command"],
            format!("sh \"${{CLAUDE_PLUGIN_ROOT}}/scripts/avisar.sh\" {evento} || true"),
            "{evento}"
        );
    }
}

#[test]
fn manifests_do_plugin_e_do_marketplace() {
    let ler = |caminho: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(comum::raiz().join(caminho)).unwrap())
            .unwrap()
    };
    let plugin = ler("plugin/.claude-plugin/plugin.json");
    assert_eq!(plugin["name"], "bichinho");
    assert_eq!(plugin["version"], "0.1.0");
    assert_eq!(plugin["license"], "MIT");
    let mercado = ler(".claude-plugin/marketplace.json");
    assert_eq!(mercado["name"], "bichinho-local");
    let entradas = mercado["plugins"].as_array().unwrap();
    assert_eq!(entradas.len(), 1);
    assert_eq!(entradas[0]["name"], "bichinho");
    assert_eq!(entradas[0]["source"], "./plugin");
    assert!(
        entradas[0].get("version").is_none(),
        "a versão mora só no plugin.json"
    );
}

#[test]
fn pelo_curl_de_verdade_ate_o_daemon() {
    let banca = Banca::nova();
    let daemon = comum::Daemon::subir(true);
    let porta = daemon.porta.to_string();
    let mut enviados = 0;
    for caso in casos() {
        let r = banca.rodar(caso.evento, caso.entrada.to_string().as_bytes(), |c| {
            c.env("PATH", "/usr/bin:/bin").env("PET_PORTA", &porta);
        });
        r.calada(caso.nome);
        assert!(r.corpo.is_none(), "o curl falso não pode ter rodado");
        enviados += 1;
    }
    let estado = daemon.esperar_estado("eventos chegaram", |e| e["eventos"]["aceitos"] == enviados);
    assert_eq!(estado["eventos"]["recusados"], 0);
    let eventos = daemon.get_json("/v1/debug/eventos");
    let lista = eventos["eventos"].as_array().unwrap();
    assert_eq!(lista.len(), enviados as usize);
    assert!(
        lista.iter().all(|e| e.get("descartados").is_none()),
        "{eventos:#}"
    );
    std::thread::sleep(Duration::from_millis(100));
    for (onde, texto) in [
        ("/v1/debug/eventos", eventos.to_string()),
        ("/v1/estado", daemon.get_json("/v1/estado").to_string()),
        ("log do daemon", daemon.log()),
    ] {
        sem_segredo(onde, &texto);
    }
}

/// Um `.curlrc` que manda tudo para outro lugar (proxy e `connect-to`).
fn curlrc_desviando(pasta: &Path) {
    let desvio = "proxy = \"http://127.0.0.1:9\"\nconnect-to = \"::127.0.0.1:9\"\n";
    std::fs::create_dir_all(pasta).unwrap();
    for nome in [".curlrc", "curlrc"] {
        std::fs::write(pasta.join(nome), desvio).unwrap();
    }
}

#[test]
fn proxy_e_curlrc_nunca_desviam_o_evento() {
    // Os hooks herdam o ambiente do Claude Code: um proxy nas variáveis ou
    // um `~/.curlrc` levariam os metadados para outra máquina, e o pet
    // ficaria surdo sem erro nenhum. O curl de verdade, com proxy em todas
    // as variáveis e curlrc desviando em todo lugar onde o curl procura,
    // ainda entrega no daemon do 127.0.0.1 (decisão 0031).
    let banca = Banca::nova();
    let daemon = comum::Daemon::subir(true);
    let porta = daemon.porta.to_string();
    let config = banca.pasta.join("config-curl");
    curlrc_desviando(&config);
    curlrc_desviando(&banca.pasta.join("home"));
    let desvio = "http://127.0.0.1:9";
    let entrada = com("Stop", json!({})).to_string();
    let r = banca.rodar("Stop", entrada.as_bytes(), |c| {
        c.env("PATH", "/usr/bin:/bin")
            .env("PET_PORTA", &porta)
            .env("CURL_HOME", &config)
            .env("XDG_CONFIG_HOME", &config);
        for variavel in [
            "http_proxy",
            "HTTP_PROXY",
            "https_proxy",
            "HTTPS_PROXY",
            "all_proxy",
            "ALL_PROXY",
        ] {
            c.env(variavel, desvio);
        }
    });
    r.calada("proxy e curlrc");
    assert!(r.corpo.is_none(), "o curl falso não pode ter rodado");
    let estado = daemon.esperar_estado("o evento chegou ao pet", |e| e["eventos"]["aceitos"] == 1);
    assert_eq!(estado["eventos"]["recusados"], 0);
}

#[test]
fn um_jq_do_usuario_nao_muda_a_lista_branca() {
    // O jq lê o `~/.jq` sozinho, antes do programa: um que redefine `test`,
    // `select` e `with_entries` deixaria passar qualquer texto nos campos
    // lidos. O avisar.sh roda o jq sem esse arquivo (decisão 0031).
    let banca = Banca::nova();
    std::fs::write(
        banca.pasta.join("home/.jq"),
        "def test($re): true;\ndef select(f): .;\ndef with_entries(f): .;\n",
    )
    .unwrap();
    let entrada = json!({
        "session_id": "tem espaço SEGREDO-1",
        "prompt_id": "SEGREDO 2",
        "cwd": "/home/x/SEGREDO pasta\u{0007}",
        "tool_name": "Bash; SEGREDO-3",
        "notification_type": "Permission Prompt SEGREDO-4",
        "source": "SEGREDO user"
    });
    for evento in ["Notification", "PostToolUse", "UserPromptSubmit"] {
        let r = banca.rodar(evento, entrada.to_string().as_bytes(), |_| {});
        r.calada(evento);
        let bruto = r.corpo.clone().unwrap_or_default();
        sem_segredo(evento, &bruto);
        let lido = pet_core::evento::ler(bruto.as_bytes())
            .unwrap_or_else(|e| panic!("{evento}: o daemon recusaria: {e}"));
        assert!(lido.descartados.is_empty(), "{evento}: {bruto}");
    }
}

#[test]
fn validadores_iguais_aos_do_daemon() {
    // Todo campo que o avisar.sh manda, o `pet_core::evento` aceita: o que um
    // descartaria, o outro nem manda. O `$` do jq aceita um "\n" no fim do
    // texto (o `\z` não), e as marcas Unicode da pasta são as mesmas dos
    // dois lados (decisão 0031).
    let banca = Banca::nova();
    let casos = [
        (
            "Stop",
            json!({"session_id": "s1\n", "prompt_id": "p1\n", "cwd": "/home/x/proj\n",
                   "background_tasks": [{"id": "t1\n", "type": "shell"},
                                        {"id": "t2", "type": "shell\n"}]}),
            Some("cli\n"),
            vec!["sid", "turno", "proj", "ent"],
        ),
        (
            "PostToolUse",
            json!({"session_id": "s1", "tool_name": "Edit\n", "agent_id": "a1\n",
                   "tool_input": {"file_path": "/tmp/x"}, "cwd": "/home/x/1\u{fe0f}\u{20e3}"}),
            None,
            vec!["tool", "aid", "arq", "proj"],
        ),
        (
            "Notification",
            json!({"session_id": "s1", "notification_type": "idle_prompt\n"}),
            None,
            vec!["nt"],
        ),
        (
            "StopFailure",
            json!({"session_id": "s1", "error": "rate_limit\n"}),
            None,
            vec!["err"],
        ),
        (
            "SessionEnd",
            json!({"session_id": "s1", "reason": "clear\n"}),
            None,
            vec!["reason"],
        ),
        (
            "UserPromptSubmit",
            json!({"session_id": "s1", "source": "user\n", "prompt": "SEGREDO"}),
            None,
            vec!["src"],
        ),
    ];
    for (evento, entrada, ent, ausentes) in casos {
        let r = banca.rodar(evento, entrada.to_string().as_bytes(), |c| {
            if let Some(ent) = ent {
                c.env("CLAUDE_CODE_ENTRYPOINT", ent);
            }
        });
        r.calada(evento);
        let bruto = r.corpo.clone().unwrap_or_default();
        let lido = pet_core::evento::ler(bruto.as_bytes())
            .unwrap_or_else(|e| panic!("{evento}: o daemon recusaria: {e}"));
        assert!(
            lido.descartados.is_empty(),
            "{evento}: o daemon descartaria {:?} de {bruto}",
            lido.descartados
        );
        let corpo = r.corpo_json(evento);
        for campo in ausentes {
            assert!(corpo.get(campo).is_none(), "{evento}: {campo} em {corpo}");
        }
        if evento == "Stop" {
            assert_eq!(corpo["bg"], 2);
            assert_eq!(corpo["bgi"], json!(["t2"]), "{corpo}");
            assert_eq!(corpo["bgt"], json!(["outro"]), "{corpo}");
        }
        if evento == "PostToolUse" {
            assert_eq!(corpo["agente"], true, "{corpo}");
        }
    }
    // Pastas com acento (composto ou decomposto) e de outras escritas passam
    // dos dois lados.
    for pasta in ["ação", "acao\u{0301}", "日本", "Meu Projeto 2"] {
        let entrada = json!({"session_id": "s1", "cwd": format!("/home/x/{pasta}")});
        let r = banca.rodar("SessionStart", entrada.to_string().as_bytes(), |_| {});
        r.calada(pasta);
        let corpo = r.corpo_json(pasta);
        assert_eq!(corpo["proj"], pasta, "{corpo}");
        let lido = pet_core::evento::ler(r.corpo.unwrap().as_bytes()).unwrap();
        assert!(
            lido.descartados.is_empty(),
            "{pasta}: {:?}",
            lido.descartados
        );
    }
}

#[test]
fn entrada_enorme_sai_0_calada() {
    // Um prompt de 8 MiB (colado de um arquivo, por exemplo): o script lê,
    // manda só os metadados e sai 0, calado.
    let banca = Banca::nova();
    let prompt = "SEGREDO-enorme ".repeat(8 * 1024 * 1024 / 15);
    let entrada = com("UserPromptSubmit", json!({"prompt": prompt})).to_string();
    let r = banca.rodar("UserPromptSubmit", entrada.as_bytes(), |_| {});
    r.calada("entrada enorme");
    sem_segredo("entrada enorme", &r.corpo.clone().unwrap_or_default());
    assert_eq!(r.corpo_json("entrada enorme")["sid"], SID);
    assert!(r.duracao < Duration::from_secs(10), "levou {:?}", r.duracao);
}

#[test]
fn rapido_com_o_pet_desligado() {
    let banca = Banca::nova();
    // Uma porta onde ninguém escuta: o curl de verdade é recusado na hora.
    let porta = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|o| o.local_addr())
        .unwrap()
        .port()
        .to_string();
    let r = banca.rodar("Stop", b"{}", |c| {
        c.env("PATH", "/usr/bin:/bin").env("PET_PORTA", &porta);
    });
    r.calada("pet desligado");
    assert!(
        r.duracao < Duration::from_millis(2100),
        "levou {:?}",
        r.duracao
    );
}
