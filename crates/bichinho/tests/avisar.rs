//! Testes canário do `plugin/scripts/avisar.sh` (decisões 0009 e 0019): o
//! hook de antes do `bichinho avisar`, que o plugin instalado (0.1.0) ainda
//! chama e que fica no plugin de reserva até a troca (decisão 0041). O
//! contrato do `hooks.json` e os canários do hook nativo estão em `hook.rs`.
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

use comum::canarios::*;
use serde_json::{Value, json};

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
