//! Entrada HTTP de ponta a ponta: o binário de verdade numa porta livre,
//! requisições cruas (decisões 0008 e 0019).

mod comum;

use comum::Daemon;

const VALIDO: &str = r#"{"v":1,"e":"SessionStart","sid":"s-ingress","src":"startup","ent":"cli"}"#;

#[test]
fn validacao_de_ponta_a_ponta() {
    let d = Daemon::subir(false);
    let ok = d.cabecalhos();
    let porta = d.porta;
    let post = |cabecalhos: &str, corpo: &str| {
        d.bruto(&format!(
            "POST /v1/evento HTTP/1.1\r\n{cabecalhos}Content-Length: {}\r\n\r\n{corpo}",
            corpo.len()
        ))
        .0
    };
    let json = format!("{ok}Content-Type: application/json\r\n");

    assert_eq!(post(&json, VALIDO), 204, "válido");
    assert_eq!(
        post(
            &format!("Host: evil.com:{porta}\r\nX-Pet: 1\r\nContent-Type: application/json\r\n"),
            VALIDO
        ),
        403,
        "Host de fora"
    );
    assert_eq!(
        post(
            &format!("Host: 127.0.0.1:{porta}\r\nContent-Type: application/json\r\n"),
            VALIDO
        ),
        403,
        "sem X-Pet"
    );
    assert_eq!(
        post(&format!("{ok}Content-Type: text/plain\r\n"), VALIDO),
        415,
        "text/plain"
    );
    let grande = format!(r#"{{"v":1,"e":"Stop","x":"{}"}}"#, "a".repeat(9 * 1024));
    assert_eq!(post(&json, &grande), 413, "9 KiB");
    let (status, _) = d.bruto(&format!(
        "POST /v1/evento HTTP/1.1\r\n{json}Transfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n"
    ));
    assert_eq!(status, 411, "chunked");
    assert_eq!(post(&json, "{lixo"), 400, "JSON inválido");
    assert_eq!(post(&json, r#"{"e":"Stop"}"#), 400, "sem v");
    assert_eq!(post(&json, r#"{"v":1}"#), 400, "sem e");

    let estado = d.esperar_estado("contagem", |e| e["eventos"]["aceitos"] == 1);
    assert_eq!(estado["eventos"]["recusados"], 4, "415 e os três 400");
    assert_eq!(d.get("/v1/debug/eventos").0, 404, "sem debug a rota some");
}

#[test]
fn debug_eventos_so_com_metadados() {
    let d = Daemon::subir(true);
    let corpo = r#"{"v":1,"e":"PostToolUse","sid":"s-canario","turno":"p-1","tool":"Bash","dur":12,
        "ent":"cli","nt":"SEGREDO-nt","proj":"/home/SEGREDO-pasta/x",
        "prompt":"SEGREDO-prompt","tool_input":{"command":"echo SEGREDO-cmd"},
        "tool_response":"SEGREDO-resp","last_assistant_message":"SEGREDO-msg"}"#;
    assert_eq!(d.post_json("/v1/evento", corpo).0, 204);
    let eventos = d.get_json("/v1/debug/eventos");
    let lista = eventos["eventos"].as_array().expect("lista");
    assert_eq!(lista.len(), 1, "{eventos:#}");
    let ev = &lista[0];
    assert_eq!(ev["e"], "PostToolUse");
    assert_eq!(ev["sid"], "s-canario");
    assert_eq!(ev["tool"], "Bash");
    assert_eq!(ev["dur"], 12);
    assert!(ev["recebido_ms"].as_u64().unwrap() > 1_700_000_000_000);
    assert_eq!(ev["descartados"], serde_json::json!(["nt", "proj"]));
    // Espera o laço principal processar e registrar no log.
    d.esperar_estado("evento processado", |e| e["eventos"]["aceitos"] == 1);
    std::thread::sleep(std::time::Duration::from_millis(100));
    for (onde, texto) in [
        ("/v1/debug/eventos", eventos.to_string()),
        ("/v1/estado", d.get_json("/v1/estado").to_string()),
        ("log do daemon", d.log()),
    ] {
        assert!(!texto.contains("SEGREDO"), "{onde} vazou: {texto}");
    }
    assert!(
        d.log().contains("evento PostToolUse da sessão s-canari"),
        "o log registra só nome e começo do id: {}",
        d.log()
    );
}

#[test]
fn comandos_de_ponta_a_ponta() {
    let d = Daemon::subir(true);
    assert_eq!(
        d.post_json("/v1/comando", r#"{"cmd":"tocar","arg":"nod"}"#)
            .0,
        204
    );
    assert_eq!(d.post_json("/v1/comando", r#"{"cmd":"esconder"}"#).0, 204);
    assert_eq!(d.post_json("/v1/comando", r#"{"cmd":"mostrar"}"#).0, 204);
    assert_eq!(d.post_json("/v1/comando", r#"{"cmd":"voar"}"#).0, 400);
    // As aprovações do M2 vêm pelo mesmo `/v1/comando` (decisão 0030): o
    // laço escolhe o personagem de novo e a resposta traz o resultado.
    let (status, corpo) = d.post_json("/v1/comando", r#"{"cmd":"revogar_skin","arg":"zeca"}"#);
    assert_eq!(status, 200, "{corpo}");
    let resposta: serde_json::Value = serde_json::from_str(&corpo).unwrap();
    assert_eq!(resposta["revogada"], false);
    assert_eq!(resposta["aplicado"], true);
    assert_eq!(
        resposta["personagem"]["pedida"], "_teste",
        "debug: a skin de teste"
    );
    let sha = "a".repeat(64);
    let aprova = format!(r#"{{"cmd":"aprovar_skin","arg":{{"id":"zeca","sha256":"{sha}"}}}}"#);
    assert_eq!(
        d.post_json("/v1/comando", &aprova).0,
        404,
        "sem o Zeca na busca"
    );
    let (status, _) = d.bruto(&format!(
        "POST /v1/comando HTTP/1.1\r\n{}Content-Type: text/plain\r\nContent-Length: 2\r\n\r\n{{}}",
        d.cabecalhos()
    ));
    assert_eq!(status, 415);
    // Sem compositor: o tocar chega ao laço e avisa que não há onde tocar.
    let limite = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !d.log().contains("tocar «nod» sem compositor") {
        assert!(std::time::Instant::now() < limite, "{}", d.log());
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
