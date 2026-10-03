//! O cérebro dentro do daemon de verdade (sem compositor): eventos pelo
//! `/v1/evento`, reações no `/v1/estado` (decisão 0020).

mod comum;

use comum::Daemon;
use serde_json::{Value, json};

const SID: &str = "c0ffee00-1111-4222-8333-444444444444";

fn evento(e: &str, extra: Value) -> String {
    let mut v = json!({"v": 1, "e": e, "sid": SID, "ent": "cli", "proj": "meu-projeto"});
    for (k, valor) in extra.as_object().unwrap() {
        v[k] = valor.clone();
    }
    v.to_string()
}

fn mandar(d: &Daemon, corpo: String) {
    let (status, resposta) = d.post_json("/v1/evento", &corpo);
    assert_eq!(status, 204, "{corpo}: {resposta}");
}

fn reacao(e: &Value) -> Option<&str> {
    e["ultima_reacao"]["nome"].as_str()
}

#[test]
fn rapido_acena_pequeno_pula_e_fim_da_sessao_da_tchau() {
    let d = Daemon::subir(false);
    mandar(&d, evento("SessionStart", json!({"src": "startup"})));
    mandar(
        &d,
        evento("UserPromptSubmit", json!({"turno": "p1", "src": "user"})),
    );
    mandar(&d, evento("Stop", json!({"turno": "p1"})));
    let estado = d.esperar_estado("aceno", |e| reacao(e) == Some("nod"));
    assert_eq!(estado["ultima_reacao"]["sid8"], "c0ffee00");
    assert_eq!(estado["ultima_reacao"]["proj"], "meu-projeto");
    assert_eq!(estado["ultima_reacao"]["nivel"], "T0");
    assert_eq!(estado["ultima_reacao"]["teste"], false);

    mandar(
        &d,
        evento("UserPromptSubmit", json!({"turno": "p2", "src": "user"})),
    );
    mandar(
        &d,
        evento(
            "PostToolUse",
            json!({"turno": "p2", "tool": "Edit", "arq": "0123456789ab", "dur": 40}),
        ),
    );
    mandar(
        &d,
        evento(
            "PostToolUse",
            json!({"turno": "p2", "tool": "Bash", "dur": 900}),
        ),
    );
    mandar(&d, evento("Stop", json!({"turno": "p2"})));
    let estado = d.esperar_estado("pulinho", |e| reacao(e) == Some("done_small"));
    let turnos = estado["turnos"].as_array().unwrap();
    assert_eq!(turnos.len(), 2);
    assert_eq!(turnos[0]["nivel"], "T1");
    assert_eq!(turnos[0]["trabalho"], 2);
    assert_eq!(turnos[0]["arquivos"], 1);
    assert_eq!(turnos[0]["dur_ms"], 940);
    assert_eq!(turnos[1]["nivel"], "T0");
    let sessao = &estado["sessoes"][0];
    assert_eq!(sessao["sid8"], "c0ffee00");
    assert_eq!(sessao["estado"], "parada");
    assert_eq!(sessao["contadores"]["reacoes"], 2);

    // Stop repetido: nada muda.
    mandar(&d, evento("Stop", json!({"turno": "p2"})));
    mandar(
        &d,
        evento("SessionEnd", json!({"reason": "prompt_input_exit"})),
    );
    let estado = d.esperar_estado("tchau", |e| reacao(e) == Some("bye"));
    assert_eq!(estado["sessoes"], json!([]));
    assert_eq!(estado["cerebro"]["ignorados"]["stop_repetido"], 1);
    let log = d.log();
    assert!(log.contains("reação nod (T0) da sessão c0ffee00"), "{log}");
    assert!(
        log.contains("reação done_small (T1) da sessão c0ffee00"),
        "{log}"
    );
    assert!(!log.contains(SID), "o log só tem o sid curto: {log}");
}

#[test]
fn origem_de_fora_ignorada_e_teste_isolado() {
    let d = Daemon::subir(false);
    for e in ["UserPromptSubmit", "Stop"] {
        let mut v: Value = serde_json::from_str(&evento(e, json!({"turno": "p1"}))).unwrap();
        v["ent"] = json!("sdk-cli");
        mandar(&d, v.to_string());
    }
    let estado = d.esperar_estado("ignorados", |e| {
        e["cerebro"]["ignorados"]["origem:sdk-cli"] == 2
    });
    assert_eq!(estado["sessoes"], json!([]));
    assert!(estado["ultima_reacao"].is_null());

    mandar(
        &d,
        evento("UserPromptSubmit", json!({"turno": "p1", "teste": true})),
    );
    mandar(&d, evento("Stop", json!({"turno": "p1", "teste": true})));
    let estado = d.esperar_estado("aceno de teste", |e| reacao(e) == Some("nod"));
    assert_eq!(estado["ultima_reacao"]["teste"], true);
    assert_eq!(estado["sessoes"][0]["teste"], true);
}

#[test]
fn origens_pela_config() {
    let d = Daemon::subir_com(false, &[("PET_SESSOES_ORIGENS", "cli,sdk-cli")]);
    let estado = d.get_json("/v1/estado");
    assert_eq!(estado["cerebro"]["origens"], json!(["cli", "sdk-cli"]));
    let mut v: Value = serde_json::from_str(&evento("Stop", json!({"turno": "p1"}))).unwrap();
    v["ent"] = json!("sdk-cli");
    mandar(&d, v.to_string());
    d.esperar_estado("aceno do sdk-cli", |e| reacao(e) == Some("nod"));
}
