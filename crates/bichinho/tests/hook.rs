//! Testes canário do hook nativo, `bichinho avisar <Evento>` (decisões
//! 0009, 0031 e 0041), e o contrato do `hooks.json` em exec form.
//!
//! Os mesmos casos do `avisar.sh` (`comum::canarios`): todo campo de
//! conteúdo carrega um `SEGREDO-…`. O binário de verdade roda com o ambiente
//! limpo e manda para um pet falso numa porta livre do 127.0.0.1, que guarda
//! o pedido inteiro (linha, cabeçalhos e corpo). Nada com `segredo`, em caixa
//! nenhuma, pode sair; o hook sai sempre 0, não imprime nada e nunca passa
//! do prazo. Nenhum teste roda o hook sem `PET_PORTA` de teste: o pet de
//! produção nunca recebe nada daqui.

mod comum;

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use comum::canarios::*;
use serde_json::{Value, json};

/// O prazo do hook inteiro (`avisar::PRAZO_TOTAL`), com folga para o
/// processo subir.
const PRAZO_TOTAL: Duration = Duration::from_secs(4);

/// Um pedido que chegou ao pet falso.
#[derive(Debug, Clone)]
struct Pedido {
    /// Tudo o que chegou, como texto (para procurar segredo).
    bruto: String,
    linha: String,
    cabecalhos: Vec<(String, String)>,
    corpo: String,
}

impl Pedido {
    fn cabecalho(&self, nome: &str) -> Option<&str> {
        self.cabecalhos
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(nome))
            .map(|(_, v)| v.as_str())
    }
}

/// O pet falso: aceita conexões numa porta livre e guarda cada pedido. Com
/// `responder`, devolve 204 como o pet; sem, segura a conexão calado (um pet
/// travado).
struct Captor {
    porta: u16,
    recebidos: mpsc::Receiver<Pedido>,
}

impl Captor {
    fn novo(responder: bool) -> Captor {
        let ouvinte = TcpListener::bind("127.0.0.1:0").expect("porta livre");
        let porta = ouvinte.local_addr().unwrap().port();
        let (manda, recebidos) = mpsc::channel();
        thread::spawn(move || {
            let mut presas = Vec::new();
            for conexao in ouvinte.incoming() {
                let Ok(mut fluxo) = conexao else {
                    continue;
                };
                let _ = fluxo.set_read_timeout(Some(Duration::from_secs(5)));
                if let Some(pedido) = ler_pedido(&mut fluxo) {
                    let _ = manda.send(pedido);
                }
                if responder {
                    let _ =
                        fluxo.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n");
                } else {
                    presas.push(fluxo);
                }
            }
        });
        Captor { porta, recebidos }
    }

    fn receber(&self) -> Option<Pedido> {
        self.recebidos.recv_timeout(Duration::from_millis(500)).ok()
    }

    fn nada(&self) -> bool {
        self.recebidos
            .recv_timeout(Duration::from_millis(200))
            .is_err()
    }
}

fn ler_pedido(fluxo: &mut TcpStream) -> Option<Pedido> {
    let mut lido = Vec::new();
    let mut pedaco = [0u8; 4096];
    let fim = loop {
        if let Some(pos) = lido.windows(4).position(|j| j == b"\r\n\r\n") {
            break pos;
        }
        let n = fluxo.read(&mut pedaco).ok()?;
        if n == 0 {
            return None;
        }
        lido.extend_from_slice(&pedaco[..n]);
    };
    let cabeca = String::from_utf8_lossy(&lido[..fim]).into_owned();
    let mut linhas = cabeca.split("\r\n");
    let linha = linhas.next()?.to_owned();
    let cabecalhos: Vec<(String, String)> = linhas
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_owned(), v.trim().to_owned()))
        .collect();
    let tamanho: usize = cabecalhos
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())?;
    let mut corpo = lido[fim + 4..].to_vec();
    while corpo.len() < tamanho {
        let n = fluxo.read(&mut pedaco).ok()?;
        if n == 0 {
            break;
        }
        corpo.extend_from_slice(&pedaco[..n]);
    }
    let mut bruto = lido[..fim + 4].to_vec();
    bruto.extend_from_slice(&corpo);
    Some(Pedido {
        bruto: String::from_utf8_lossy(&bruto).into_owned(),
        linha,
        cabecalhos,
        corpo: String::from_utf8_lossy(&corpo).into_owned(),
    })
}

/// Pasta com o HOME do teste.
struct Banca {
    pasta: PathBuf,
}

impl Banca {
    fn nova() -> Banca {
        let pasta = comum::pasta_temporaria("hook");
        std::fs::create_dir_all(pasta.join("home")).unwrap();
        Banca { pasta }
    }
}

impl Drop for Banca {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.pasta);
    }
}

struct Rodada {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    duracao: Duration,
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
}

fn hook() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bichinho"))
}

/// Roda `bichinho avisar <evento>` com `entrada` no stdin e o ambiente
/// limpo: HOME próprio, `CLAUDE_CODE_ENTRYPOINT=cli` e a porta `porta`.
fn rodar(
    banca: &Banca,
    porta: u16,
    evento: &str,
    entrada: &[u8],
    ajuste: impl FnOnce(&mut Command),
) -> Rodada {
    let mut cmd = hook();
    cmd.arg("avisar")
        .arg(evento)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", banca.pasta.join("home"))
        .env("CLAUDE_CODE_ENTRYPOINT", "cli")
        .env("PET_PORTA", porta.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    ajuste(&mut cmd);
    let inicio = Instant::now();
    let mut filho = cmd.spawn().expect("rodar o bichinho");
    let mut stdin = filho.stdin.take().unwrap();
    // O hook pode nem ler a entrada (nome de evento inválido).
    let _ = stdin.write_all(entrada);
    drop(stdin);
    let saida = filho.wait_with_output().unwrap();
    Rodada {
        status: saida.status.code(),
        stdout: saida.stdout,
        stderr: saida.stderr,
        duracao: inicio.elapsed(),
    }
}

/// Roda e devolve o pedido que chegou ao pet falso (tem de chegar).
fn rodar_e_pegar(
    banca: &Banca,
    captor: &Captor,
    nome: &str,
    evento: &str,
    entrada: &[u8],
    ajuste: impl FnOnce(&mut Command),
) -> Pedido {
    let r = rodar(banca, captor.porta, evento, entrada, ajuste);
    r.calada(nome);
    captor
        .receber()
        .unwrap_or_else(|| panic!("{nome}: nada chegou ao pet"))
}

fn json_do(pedido: &Pedido, caso: &str) -> Value {
    serde_json::from_str(&pedido.corpo)
        .unwrap_or_else(|e| panic!("{caso}: corpo não é JSON ({e}): {}", pedido.corpo))
}

#[test]
fn nenhum_segredo_sai_em_evento_nenhum() {
    let banca = Banca::nova();
    let captor = Captor::novo(true);
    let casos = casos();
    let cobertos: BTreeSet<&str> = casos.iter().map(|c| c.evento).collect();
    assert_eq!(
        cobertos,
        EVENTOS.into_iter().collect(),
        "um caso por evento"
    );
    for caso in &casos {
        let nome = caso.nome;
        let pedido = rodar_e_pegar(
            &banca,
            &captor,
            nome,
            caso.evento,
            caso.entrada.to_string().as_bytes(),
            |_| {},
        );
        sem_segredo(&format!("{nome}, pedido inteiro"), &pedido.bruto);
        let corpo = json_do(&pedido, nome);
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
        // O pet aceita o corpo inteiro, sem descartar nada.
        let lido = pet_core::evento::ler(pedido.corpo.as_bytes())
            .unwrap_or_else(|e| panic!("{nome}: o pet recusaria: {e}"));
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
    let captor = Captor::novo(true);
    let rodar = |nome: &str| {
        let caso = casos().into_iter().find(|c| c.nome == nome).unwrap();
        let pedido = rodar_e_pegar(
            &banca,
            &captor,
            nome,
            caso.evento,
            caso.entrada.to_string().as_bytes(),
            |_| {},
        );
        json_do(&pedido, nome)
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
    assert_eq!(rodar("SubagentStart")["aid"], "agent-def456");

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
    let captor = Captor::novo(true);
    let entrada = com("Stop", json!({})).to_string();
    for (valor, esperado) in [
        (Some("1"), Some(true)),
        (Some("0"), None),
        (Some("sim"), None),
        (None, None),
    ] {
        let pedido = rodar_e_pegar(
            &banca,
            &captor,
            "PET_TESTE",
            "Stop",
            entrada.as_bytes(),
            |c| {
                if let Some(v) = valor {
                    c.env("PET_TESTE", v);
                }
            },
        );
        let corpo = json_do(&pedido, "PET_TESTE");
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
    let captor = Captor::novo(true);
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
        let pedido = rodar_e_pegar(&banca, &captor, "dnd", "Stop", entrada.as_bytes(), |c| {
            c.env("XDG_STATE_HOME", &estado);
        });
        assert_eq!(json_do(&pedido, "dnd")["dnd"], esperado, "{conteudo:?}");
        sem_segredo("dnd", &pedido.bruto);
    }
    // Sem XDG_STATE_HOME vale ~/.local/state.
    let padrao = banca
        .pasta
        .join("home/.local/state/omarchy/notifications.json");
    std::fs::create_dir_all(padrao.parent().unwrap()).unwrap();
    std::fs::write(&padrao, r#"{"dnd":true}"#).unwrap();
    let pedido = rodar_e_pegar(
        &banca,
        &captor,
        "dnd padrão",
        "Stop",
        entrada.as_bytes(),
        |_| {},
    );
    assert_eq!(json_do(&pedido, "dnd padrão")["dnd"], true);
}

#[test]
fn entrada_que_nao_e_json_manda_o_minimo() {
    let banca = Banca::nova();
    let captor = Captor::novo(true);
    for entrada in [
        &b""[..],
        b"nao e json SEGREDO-1",
        b"[1, 2, \"SEGREDO-2\"]",
        b"\"SEGREDO-3\"",
        b"{\"session_id\": \"SEGREDO-4",
        b"\xff\xfe SEGREDO-5",
    ] {
        let pedido = rodar_e_pegar(&banca, &captor, "entrada ruim", "Stop", entrada, |_| {});
        assert_eq!(
            pedido.corpo,
            r#"{"v":1,"e":"Stop"}"#,
            "{:?}",
            String::from_utf8_lossy(entrada)
        );
    }
    let pedido = rodar_e_pegar(&banca, &captor, "teste", "Stop", b"lixo", |c| {
        c.env("PET_TESTE", "1");
    });
    assert_eq!(pedido.corpo, r#"{"v":1,"e":"Stop","teste":true}"#);
}

#[test]
fn campos_com_tipo_errado_caem() {
    let banca = Banca::nova();
    let captor = Captor::novo(true);
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
        let pedido = rodar_e_pegar(
            &banca,
            &captor,
            evento,
            evento,
            entrada.to_string().as_bytes(),
            |_| {},
        );
        let corpo = json_do(&pedido, evento);
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
    let captor = Captor::novo(true);
    let longo = "A".repeat(41);
    for nome in ["", "Stop;rm", "Stop Stop", "Stóp", "Stop\"", longo.as_str()] {
        let r = rodar(&banca, captor.porta, nome, b"{}", |_| {});
        r.calada(nome);
        assert!(captor.nada(), "{nome:?} não deveria mandar nada");
    }
    // Sem argumento nenhum.
    let saida = hook()
        .arg("avisar")
        .env_clear()
        .env("PET_PORTA", captor.porta.to_string())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(saida.status.code(), Some(0));
    assert!(saida.stdout.is_empty() && saida.stderr.is_empty());
    assert!(captor.nada());
}

#[test]
fn so_o_127_0_0_1_com_os_cabecalhos_do_pet() {
    let banca = Banca::nova();
    let captor = Captor::novo(true);
    let entrada = com("Stop", json!({})).to_string();
    let pedido = rodar_e_pegar(
        &banca,
        &captor,
        "cabeçalhos",
        "Stop",
        entrada.as_bytes(),
        |_| {},
    );
    assert_eq!(pedido.linha, "POST /v1/evento HTTP/1.1");
    let porta = captor.porta.to_string();
    assert_eq!(
        pedido.cabecalho("host"),
        Some(format!("127.0.0.1:{porta}").as_str())
    );
    assert_eq!(pedido.cabecalho("content-type"), Some("application/json"));
    assert_eq!(pedido.cabecalho("x-pet"), Some("1"));
    assert_eq!(
        pedido.cabecalho("content-length"),
        Some(pedido.corpo.len().to_string().as_str())
    );
}

#[test]
fn proxy_e_curlrc_nunca_desviam_o_evento() {
    // O hook herda o ambiente do Claude Code: proxy nas variáveis e curlrc
    // em todo lugar onde o curl procuraria. O hook nativo fala TCP direto com
    // o 127.0.0.1 e nem olha para eles (decisão 0031).
    let banca = Banca::nova();
    let captor = Captor::novo(true);
    let desvio = "proxy = \"http://127.0.0.1:9\"\nconnect-to = \"::127.0.0.1:9\"\n";
    for pasta in [banca.pasta.join("home"), banca.pasta.join("config-curl")] {
        std::fs::create_dir_all(&pasta).unwrap();
        for nome in [".curlrc", "curlrc"] {
            std::fs::write(pasta.join(nome), desvio).unwrap();
        }
    }
    std::fs::write(
        banca.pasta.join("home/.jq"),
        "def test($re): true;\ndef select(f): .;\n",
    )
    .unwrap();
    let entrada = com("Stop", json!({})).to_string();
    let pedido = rodar_e_pegar(&banca, &captor, "proxy", "Stop", entrada.as_bytes(), |c| {
        c.env("CURL_HOME", banca.pasta.join("config-curl"))
            .env("XDG_CONFIG_HOME", banca.pasta.join("config-curl"));
        for variavel in [
            "http_proxy",
            "HTTP_PROXY",
            "https_proxy",
            "HTTPS_PROXY",
            "all_proxy",
            "ALL_PROXY",
        ] {
            c.env(variavel, "http://127.0.0.1:9");
        }
    });
    assert_eq!(json_do(&pedido, "proxy")["sid"], SID);
}

#[test]
fn origem_valida_ou_nada() {
    let banca = Banca::nova();
    let captor = Captor::novo(true);
    let entrada = com("Stop", json!({})).to_string();
    for (ent, esperado) in [
        (Some("sdk-cli"), Some("sdk-cli")),
        (Some("claude-vscode"), Some("claude-vscode")),
        (Some("CLI"), None),
        (Some("cli; SEGREDO"), None),
        (Some("cli\n"), None),
        (None, None),
    ] {
        let pedido = rodar_e_pegar(
            &banca,
            &captor,
            "ent",
            "Stop",
            entrada.as_bytes(),
            |c| match ent {
                Some(e) => {
                    c.env("CLAUDE_CODE_ENTRYPOINT", e);
                }
                None => {
                    c.env_remove("CLAUDE_CODE_ENTRYPOINT");
                }
            },
        );
        let corpo = json_do(&pedido, "ent");
        assert_eq!(
            corpo.get("ent").and_then(Value::as_str),
            esperado,
            "{ent:?}"
        );
        sem_segredo("ent", &pedido.bruto);
    }
}

#[test]
fn validadores_iguais_aos_do_pet() {
    // Todo campo que o hook manda, o `pet_core::evento` aceita: o que um
    // descartaria, o outro nem manda ("\n" no fim, marcas Unicode da pasta).
    let banca = Banca::nova();
    let captor = Captor::novo(true);
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
        let pedido = rodar_e_pegar(
            &banca,
            &captor,
            evento,
            evento,
            entrada.to_string().as_bytes(),
            |c| {
                if let Some(ent) = ent {
                    c.env("CLAUDE_CODE_ENTRYPOINT", ent);
                }
            },
        );
        let lido = pet_core::evento::ler(pedido.corpo.as_bytes())
            .unwrap_or_else(|e| panic!("{evento}: o pet recusaria: {e}"));
        assert!(
            lido.descartados.is_empty(),
            "{evento}: o pet descartaria {:?} de {}",
            lido.descartados,
            pedido.corpo
        );
        let corpo = json_do(&pedido, evento);
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
    for pasta in ["ação", "acao\u{0301}", "日本", "Meu Projeto 2"] {
        let entrada = json!({"session_id": "s1", "cwd": format!("/home/x/{pasta}")});
        let pedido = rodar_e_pegar(
            &banca,
            &captor,
            pasta,
            "SessionStart",
            entrada.to_string().as_bytes(),
            |_| {},
        );
        assert_eq!(json_do(&pedido, pasta)["proj"], pasta);
        let lido = pet_core::evento::ler(pedido.corpo.as_bytes()).unwrap();
        assert!(
            lido.descartados.is_empty(),
            "{pasta}: {:?}",
            lido.descartados
        );
    }
}

#[test]
fn entrada_enorme_sai_0_calada() {
    // Um prompt de 8 MiB (colado de um arquivo, por exemplo): o hook lê, manda
    // só os metadados e sai 0, calado.
    let banca = Banca::nova();
    let captor = Captor::novo(true);
    let prompt = "SEGREDO-enorme ".repeat(8 * 1024 * 1024 / 15);
    let entrada = com("UserPromptSubmit", json!({"prompt": prompt})).to_string();
    let inicio = Instant::now();
    let pedido = rodar_e_pegar(
        &banca,
        &captor,
        "entrada enorme",
        "UserPromptSubmit",
        entrada.as_bytes(),
        |_| {},
    );
    sem_segredo("entrada enorme", &pedido.bruto);
    assert_eq!(json_do(&pedido, "entrada enorme")["sid"], SID);
    assert!(
        inicio.elapsed() < Duration::from_secs(3),
        "levou {:?}",
        inicio.elapsed()
    );
}

#[test]
fn rapido_com_o_pet_desligado() {
    let banca = Banca::nova();
    // Uma porta onde ninguém escuta: a conexão é recusada na hora.
    let porta = TcpListener::bind("127.0.0.1:0")
        .and_then(|o| o.local_addr())
        .unwrap()
        .port();
    let r = rodar(&banca, porta, "Stop", b"{}", |_| {});
    r.calada("pet desligado");
    assert!(
        r.duracao < Duration::from_millis(500),
        "levou {:?}",
        r.duracao
    );
}

#[test]
fn pet_travado_nao_segura_o_hook() {
    // O pet aceita e nunca responde: o hook desiste no prazo do POST (2 s),
    // calado, e o evento já foi entregue.
    let banca = Banca::nova();
    let captor = Captor::novo(false);
    let r = rodar(&banca, captor.porta, "Stop", b"{}", |_| {});
    r.calada("pet travado");
    assert!(
        r.duracao < Duration::from_millis(2_600),
        "levou {:?}",
        r.duracao
    );
    assert!(captor.receber().is_some(), "o evento chegou");
}

#[test]
fn entrada_que_nunca_fecha_tem_prazo() {
    // Quem chama nunca fecha a entrada padrão: o hook sai 0, calado, no
    // prazo total, sem mandar nada que não fecha.
    let banca = Banca::nova();
    let captor = Captor::novo(true);
    let mut filho = hook()
        .args(["avisar", "Stop"])
        .env_clear()
        .env("HOME", banca.pasta.join("home"))
        .env("PET_PORTA", captor.porta.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = filho.stdin.take().unwrap();
    stdin.write_all(br#"{"session_id": "SEGREDO"#).unwrap();
    let inicio = Instant::now();
    let saida = loop {
        if let Some(status) = filho.try_wait().unwrap() {
            break status;
        }
        assert!(
            inicio.elapsed() < PRAZO_TOTAL + Duration::from_secs(2),
            "o hook passou do prazo"
        );
        thread::sleep(Duration::from_millis(50));
    };
    drop(stdin);
    assert_eq!(saida.code(), Some(0));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    filho
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    filho
        .stderr
        .take()
        .unwrap()
        .read_to_end(&mut stderr)
        .unwrap();
    assert!(stdout.is_empty() && stderr.is_empty());
    assert!(captor.nada(), "nada sai de uma entrada pela metade");
}

#[test]
fn pelo_pet_de_verdade() {
    // O daemon de verdade (debug) recebe todos os casos sem recusar nem
    // descartar nada, e nada dos segredos aparece nos eventos de debug, no
    // estado nem no log.
    let banca = Banca::nova();
    let daemon = comum::Daemon::subir(true);
    let mut enviados = 0;
    for caso in casos() {
        let r = rodar(
            &banca,
            daemon.porta,
            caso.evento,
            caso.entrada.to_string().as_bytes(),
            |_| {},
        );
        r.calada(caso.nome);
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

#[test]
fn hooks_json_em_exec_form_com_os_13_eventos_async() {
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
        let hook = lista[0].as_object().unwrap();
        assert_eq!(hook["type"], "command", "{evento}");
        assert_eq!(hook["async"], true, "{evento}");
        // Exec form: o binário no PATH, sem shell, sem jq, sem curl.
        assert_eq!(hook["command"], "bichinho", "{evento}");
        assert_eq!(hook["args"], json!(["avisar", evento]), "{evento}");
        let chaves: BTreeSet<&str> = hook.keys().map(String::as_str).collect();
        assert_eq!(
            chaves,
            BTreeSet::from(["type", "async", "command", "args"]),
            "{evento}: nada de shell nem timeout"
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
    assert_eq!(plugin["version"], "0.2.0");
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
    // A reserva até a troca continua no plugin (decisão 0041).
    assert!(
        Path::new(&comum::raiz().join("plugin/scripts/avisar.sh")).is_file(),
        "o avisar.sh fica de reserva até a troca"
    );
}
