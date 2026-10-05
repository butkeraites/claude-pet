//! A memória das sessões no daemon de verdade (decisão 0093): o SIGTERM grava
//! `sessoes.json` na pasta do estado, só com metadados das sessões reais; a
//! partida seguinte, com o mesmo estado, devolve as sessões abertas sem
//! evento nenhum, marcadas como restauradas e sem tocar nada; um arquivo de
//! outra partida da máquina (outro boot id) não traz nada.

mod comum;

use std::path::Path;

use comum::Daemon;
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
    let texto = std::fs::read_to_string(&arquivo).expect("o SIGTERM gravou a memória");
    assert!(texto.contains(SID), "{texto}");
    assert!(!texto.contains("7e57e57e"), "a de teste nunca vai: {texto}");
    let memoria: Value = serde_json::from_str(&texto).unwrap();
    assert_eq!(memoria["versao"], 1);
    assert!(memoria["boot"].is_string());
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
