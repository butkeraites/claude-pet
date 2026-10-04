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
    // Sem personagem aprovado (o Zeca nem está na busca), a reação fica
    // registrada no /v1/estado e nada toca na tela (decisão 0030).
    assert!(estado["skin"]["id"].is_null(), "{estado:#}");
    assert_eq!(estado["skin"]["pedida"], "zeca");
    assert!(estado["reacao"].is_null());
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

fn agora_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[test]
fn stop_atrasado_e_continuacao_de_stop_bloqueado() {
    // Hooks async chegam fora de ordem (decisão 0032). O Stop do p1 que chega
    // depois do prompt do p2 ainda comemora o p1; e a continuação de um Stop
    // segurado por outro plugin reabre o turno, com o pulinho no Stop final.
    let d = Daemon::subir(false);
    let t = agora_ms();
    mandar(
        &d,
        evento("UserPromptSubmit", json!({"turno": "p1", "ts": t - 6_000})),
    );
    mandar(
        &d,
        evento(
            "PostToolUse",
            json!({"turno": "p1", "tool": "Edit", "arq": "0123456789ab", "ts": t - 5_000}),
        ),
    );
    mandar(
        &d,
        evento("UserPromptSubmit", json!({"turno": "p2", "ts": t - 2_997})),
    );
    mandar(&d, evento("Stop", json!({"turno": "p1", "ts": t - 3_000})));
    let estado = d.esperar_estado("pulinho do p1", |e| reacao(e) == Some("done_small"));
    assert_eq!(estado["turnos"][0]["turno8"], "p1");
    assert_eq!(estado["turnos"][0]["fim"], "stop");
    assert_eq!(estado["sessoes"][0]["estado"], "pensando", "o p2 segue");

    mandar(&d, evento("Stop", json!({"turno": "p2", "ts": t - 2_000})));
    d.esperar_estado("aceno do p2", |e| reacao(e) == Some("nod"));
    mandar(
        &d,
        evento(
            "PostToolUse",
            json!({"turno": "p2", "tool": "Bash", "dur": 300, "ts": agora_ms()}),
        ),
    );
    mandar(
        &d,
        evento(
            "Stop",
            json!({"turno": "p2", "sha": true, "ts": agora_ms() + 1}),
        ),
    );
    let estado = d.esperar_estado("pulinho do p2", |e| {
        reacao(e) == Some("done_small")
            && e["ultima_reacao"]["nivel"] == "T1"
            && e["turnos"][0]["turno8"] == "p2"
            && e["turnos"][0]["continuacoes"] == 1
    });
    let turnos = estado["turnos"].as_array().unwrap();
    assert_eq!(turnos.len(), 2, "um registro por turno: {estado:#}");
    assert_eq!(turnos[0]["sha"], true);
    assert_eq!(turnos[0]["trabalho"], 1);
    assert_eq!(estado["cerebro"]["ignorados"], json!({}));
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

#[test]
fn config_relida_na_aprovacao_vale_para_o_cerebro() {
    // O daemon relê o config a cada aprovação ou revogação (decisão 0029);
    // as origens e o modo do cérebro vêm junto (decisão 0030).
    let d = Daemon::subir(false);
    assert_eq!(
        d.get_json("/v1/estado")["cerebro"]["origens"],
        json!(["cli"])
    );
    let pasta = d.pasta.join("config");
    std::fs::create_dir_all(&pasta).unwrap();
    std::fs::write(
        pasta.join("claude-pet.toml"),
        "[sessoes]\norigens = [\"cli\", \"sdk-cli\"]\n",
    )
    .unwrap();
    let (status, corpo) = d.post_json("/v1/comando", r#"{"cmd":"revogar_skin","arg":"zeca"}"#);
    assert_eq!(status, 200, "{corpo}");
    let estado = d.get_json("/v1/estado");
    assert_eq!(estado["cerebro"]["origens"], json!(["cli", "sdk-cli"]));
    assert_eq!(
        estado["config"]["chaves"]["sessoes.origens"]["origem"],
        "arquivo"
    );
    let mut v: Value = serde_json::from_str(&evento("Stop", json!({"turno": "p1"}))).unwrap();
    v["ent"] = json!("sdk-cli");
    mandar(&d, v.to_string());
    d.esperar_estado("aceno do sdk-cli", |e| reacao(e) == Some("nod"));
    assert!(
        d.log()
            .contains("o cérebro passa a acompanhar as origens cli, sdk-cli")
    );
}
