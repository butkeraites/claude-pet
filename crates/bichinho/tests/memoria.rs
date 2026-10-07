//! A memória das sessões no daemon de verdade (decisão 0093): o SIGTERM grava
//! `sessoes.json` na pasta do estado, só com metadados das sessões reais; a
//! partida seguinte, com o mesmo estado, devolve as sessões abertas sem
//! evento nenhum, marcadas como restauradas e sem tocar nada; um arquivo de
//! outra partida da máquina (outro boot id) não traz nada. Com uma instância
//! de mentira do Hyprland, a descoberta tira as janelas vistas noutra
//! instância (decisão 0095).

mod comum;

use std::path::Path;
// Só o teste da descoberta do compositor (Linux) usa estes.
#[cfg(target_os = "linux")]
use std::thread;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use comum::Daemon;
#[cfg(target_os = "linux")]
use comum::hyprland::HyprlandFalso;
use serde_json::{Value, json};

const SID: &str = "5e55a0d3-1111-4222-8333-444444444444";

fn mandar(d: &Daemon, e: &str, extra: Value) {
    let mut v = json!({"v": 1, "e": e, "sid": SID, "ent": "cli", "proj": "agenda"});
    for (k, valor) in extra.as_object().unwrap() {
        v[k] = valor.clone();
    }
    let corpo = v.to_string();
    let (status, resposta) = d.post_json("/v1/evento", &corpo);
    assert_eq!(status, 204, "{corpo}: {resposta}");
}

fn subir_com_estado(estado: &Path) -> Daemon {
    Daemon::subir_com(false, &[("PET_ESTADO", estado.to_str().unwrap())])
}

#[test]
fn o_sigterm_grava_e_a_partida_seguinte_devolve_as_sessoes() {
    let pasta = comum::pasta_temporaria("memoria");
    let estado = pasta.join("estado");
    let arquivo = estado.join("sessoes.json");
    let mut primeiro = subir_com_estado(&estado);
    mandar(
        &primeiro,
        "UserPromptSubmit",
        json!({"turno": "p1", "orig": "comum"}),
    );
    mandar(&primeiro, "Stop", json!({"turno": "p1", "bg": 0, "crn": 0}));
    let mut teste = json!({"turno": "x1", "teste": true});
    teste["sid"] = Value::from("7e57e57e-teste");
    mandar(&primeiro, "UserPromptSubmit", teste);
    primeiro.esperar_estado("o pronto", |e| {
        e["sessoes"]
            .as_array()
            .is_some_and(|s| s.iter().any(|s| s["aviso"]["tipo"] == "pronto"))
    });
    assert!(
        primeiro.parar(),
        "o SIGTERM sai sozinho: {}",
        primeiro.log()
    );
    // Quem gravou foi a saída, não o batimento (decisão 0095).
    assert!(
        primeiro
            .log()
            .contains("memória das sessões: 1 sessão(ões) gravada(s) na saída"),
        "{}",
        primeiro.log()
    );
    let texto = std::fs::read_to_string(&arquivo).expect("o SIGTERM gravou a memória");
    assert!(texto.contains(SID), "{texto}");
    assert!(!texto.contains("7e57e57e"), "a de teste nunca vai: {texto}");
    let memoria: Value = serde_json::from_str(&texto).unwrap();
    assert_eq!(memoria["versao"], 1);
    assert!(memoria["boot"].is_string());
    assert!(memoria["laco_ms"].is_u64(), "o relógio do laço: {texto}");
    assert_eq!(memoria["sessoes"][0]["aviso"]["tipo"], "pronto");
    // A partida seguinte, sem evento nenhum: a sessão está lá, restaurada.
    let segundo = subir_com_estado(&estado);
    let e = segundo.get_json("/v1/estado");
    let sessoes = e["sessoes"].as_array().unwrap();
    assert_eq!(sessoes.len(), 1, "{e:#}");
    assert_eq!(sessoes[0]["sid8"], "5e55a0d3");
    assert_eq!(sessoes[0]["restaurada"], true);
    assert_eq!(sessoes[0]["aviso"]["tipo"], "pronto");
    assert_eq!(e["eventos"]["aceitos"], 0, "nenhum evento desde a partida");
    assert!(e["ultima_reacao"].is_null(), "nada tocou");
    assert!(
        !e.to_string().contains(SID),
        "no /v1/estado, só o sid curto"
    );
    assert!(
        segundo
            .log()
            .contains("memória das sessões: 1 sessão(ões) de volta")
    );
    drop(segundo);
    // Outra partida da máquina: o boot id gravado não é o de agora.
    let mut memoria = memoria;
    memoria["boot"] = Value::from("00000000-0000-0000-0000-000000000000");
    std::fs::create_dir_all(&estado).unwrap();
    std::fs::write(&arquivo, memoria.to_string()).unwrap();
    let terceiro = subir_com_estado(&estado);
    let e = terceiro.get_json("/v1/estado");
    assert_eq!(e["sessoes"], json!([]), "{e:#}");
    assert!(
        e["intencoes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["i"] == "restauracao" && i["motivo"] == "maquina_reiniciou")
    );
    drop(terceiro);
    let _ = std::fs::remove_dir_all(&pasta);
}

/// O boot id desta máquina, como o daemon lê (só no Linux; o macOS lê pelo
/// `sysctl`, e o único teste que usa isto é o da descoberta do compositor,
/// que é do Wayland).
#[cfg(target_os = "linux")]
fn boot_id() -> String {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .expect("o boot id do Linux")
        .trim()
        .to_owned()
}

// A descoberta das janelas por instância do compositor é do Hyprland/Wayland
// (Linux); no macOS não existe esse conceito, então o teste é só no Linux.
#[cfg(target_os = "linux")]
#[test]
fn a_descoberta_tira_as_janelas_vistas_noutra_instancia_do_compositor() {
    // Duas sessões guardadas, cada uma com a janela vista numa instância do
    // Hyprland: a de mentira (`a_1790000000_1`) e outra (um logout e um
    // login). Na partida, a memória volta antes de achar o compositor; quando
    // a descoberta acha a instância, a janela da outra sai, e a desta fica.
    let pasta = comum::pasta_temporaria("memoria-compositor");
    let estado = pasta.join("estado");
    std::fs::create_dir_all(&estado).unwrap();
    let agora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let sessao = |sid: &str, proj: &str, endereco: &str, compositor: &str| {
        json!({
            "sid": sid, "proj": proj, "ent": "cli", "estado": "parada",
            "estado_desde_ms": agora - 1_000, "ultimo_evento_ms": agora - 1_000,
            "janela": {
                "endereco": endereco, "compositor": compositor,
                "certeza": "certa", "em_ms": agora - 2_000
            }
        })
    };
    let memoria = json!({
        "versao": 1, "gravada_ms": agora, "boot": boot_id(),
        "sessoes": [
            sessao("a1a1a1a1-desta", "api", "f00d01", "a_1790000000_1"),
            sessao("b2b2b2b2-outra", "web", "f00d02", "z_1700000000_9"),
        ]
    });
    std::fs::write(estado.join("sessoes.json"), memoria.to_string()).unwrap();
    let h = HyprlandFalso::novo("memoria", "activewindowv2>>f00d03\n");
    let d = Daemon::subir_com(
        false,
        &[
            ("PET_ESTADO", estado.to_str().unwrap()),
            ("PET_HOST_RUNTIME", h.runtime().as_str()),
        ],
    );
    let janela = |e: &Value, sid8: &str| {
        e["sessoes"]
            .as_array()
            .and_then(|s| s.iter().find(|s| s["sid8"] == sid8))
            .map(|s| s["janela"].clone())
            .unwrap_or(Value::Null)
    };
    let limite = Instant::now() + Duration::from_secs(15);
    let e = loop {
        let e = d.get_json("/v1/estado");
        if e["desktop"]["eventos"] == "ligado" && janela(&e, "b2b2b2b2")["certeza"] == "fechou" {
            break e;
        }
        assert!(
            Instant::now() < limite,
            "a descoberta não tirou a janela da outra instância: {e:#}\nlog:\n{}",
            d.log()
        );
        thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(janela(&e, "b2b2b2b2")["endereco"], Value::Null);
    assert_eq!(
        (
            &janela(&e, "a1a1a1a1")["endereco"],
            &janela(&e, "a1a1a1a1")["certeza"]
        ),
        (&json!("f00d01"), &json!("certa")),
        "a desta instância fica"
    );
    assert!(
        d.log()
            .contains("1 janela(s) de sessão ficaram sem endereço")
    );
    assert!(h.comandos_intocado());
    drop(d);
    let _ = std::fs::remove_dir_all(&pasta);
}
