//! A janela de cada sessão no daemon de verdade (decisão 0055): linhas do
//! socket2 (gravadas, de uma instância de mentira do Hyprland) e eventos dos
//! hooks pelo `/v1/evento`, com o `ts` de quando o hook rodou. O
//! `/v1/estado.sessoes[].janela` diz a janela que estava ativa na hora do
//! prompt, ou a dúvida quando a troca foi perto demais.

mod comum;

use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use comum::Daemon;
use comum::hyprland::HyprlandFalso;
use serde_json::{Value, json};

fn agora_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn esperar(d: &Daemon, descricao: &str, condicao: impl Fn(&Value) -> bool) -> Value {
    let limite = Instant::now() + Duration::from_secs(15);
    loop {
        let estado = d.get_json("/v1/estado");
        if condicao(&estado) {
            return estado;
        }
        assert!(
            Instant::now() < limite,
            "{descricao}: não chegou: {estado:#}\nlog:\n{}",
            d.log()
        );
        thread::sleep(Duration::from_millis(30));
    }
}

fn prompt(d: &Daemon, sid: &str, ts: u64, term: Value) {
    let corpo = json!({"v": 1, "e": "UserPromptSubmit", "sid": sid, "turno": format!("{sid}-{ts}"),
                       "ts": ts, "ent": "cli", "proj": "meu-projeto", "term": term});
    let (status, resposta) = d.post_json("/v1/evento", &corpo.to_string());
    assert_eq!(status, 204, "{resposta}");
}

fn janela<'a>(estado: &'a Value, sid8: &str) -> &'a Value {
    estado["sessoes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["sid8"] == sid8)
        .map(|s| &s["janela"])
        .unwrap_or(&Value::Null)
}

#[test]
fn o_prompt_casa_a_sessao_com_a_janela_ativa_e_a_troca_perto_deixa_duvida() {
    let h = HyprlandFalso::novo("janela", "activewindowv2>>f00d01\n");
    let d = Daemon::subir_com(false, &[("PET_HOST_RUNTIME", h.runtime().as_str())]);
    esperar(&d, "a primeira janela ativa", |e| {
        e["desktop"]["janela_ativa"] == "f00d01"
    });
    // O Renan manda o prompt da sessão A no foot1 (o hook carimba a hora).
    prompt(&d, "aaaa1111-sessao", agora_ms(), json!({"tmux": "%2"}));
    let estado = esperar(&d, "a janela de A", |e| !janela(e, "aaaa1111").is_null());
    assert_eq!(
        janela(&estado, "aaaa1111"),
        &json!({"endereco": "f00d01", "certeza": "certa", "terminal": {"tmux": "%2"}})
    );
    // Troca para o foot2 e manda o prompt de B na mesma hora: dúvida.
    h.mandar("activewindowv2>>f00d02\n");
    esperar(&d, "a segunda janela", |e| {
        e["desktop"]["janela_ativa"] == "f00d02"
    });
    prompt(&d, "bbbb2222-sessao", agora_ms(), json!(null));
    let estado = esperar(&d, "a dúvida de B", |e| !janela(e, "bbbb2222").is_null());
    assert_eq!(
        janela(&estado, "bbbb2222"),
        &json!({"endereco": null, "certeza": "duvida"})
    );
    // Passado o segundo de dúvida, o próximo prompt de B casa o foot2.
    thread::sleep(Duration::from_millis(1_100));
    prompt(&d, "bbbb2222-sessao", agora_ms(), json!(null));
    let estado = esperar(&d, "a janela de B", |e| {
        janela(e, "bbbb2222")["certeza"] == "certa"
    });
    assert_eq!(janela(&estado, "bbbb2222")["endereco"], "f00d02");
    // A de A não mudou.
    assert_eq!(janela(&estado, "aaaa1111")["endereco"], "f00d01");
    // O foot1 fecha: A fica sem janela.
    h.mandar("closewindow>>f00d01\n");
    let estado = esperar(&d, "a janela de A fechou", |e| {
        janela(e, "aaaa1111")["certeza"] == "fechou"
    });
    assert!(janela(&estado, "aaaa1111")["endereco"].is_null());
    assert!(h.comandos_intocado(), "alguém conectou no .socket.sock");
}
