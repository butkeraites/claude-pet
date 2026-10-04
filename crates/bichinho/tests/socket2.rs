//! O leitor do socket2 no daemon de verdade (decisões 0006 e 0050), com uma
//! instância de mentira do Hyprland: o `hyprland.lock`, um `.socket2.sock`
//! que manda linhas e um socket Wayland que desliga na hora (o handshake
//! falha e o laço espera o backoff, mas o socket2 já é lido).
//!
//! O canário: o título e a classe que o Hyprland manda (`activewindow>>
//! firefox,SEGREDO-T`, um `openwindow` e um `windowtitlev2` com segredos)
//! nunca aparecem no log (com `PET_LOG=debug`), no `/v1/estado` nem no
//! `/v1/debug/eventos`. O socket de comandos (`.socket.sock`) da instância
//! nunca recebe conexão.

mod comum;

use std::thread;
use std::time::{Duration, Instant};

use comum::Daemon;
use comum::hyprland::HyprlandFalso;
use serde_json::Value;

/// As linhas com segredo vêm depois das úteis, e uma sentinela fecha: o que
/// vazasse de uma delas ficaria no estado final, sem nada depois para
/// cobrir.
const LINHAS: &str = "focusedmonv2>>HDMI-A-1,3\n\
    activewindowv2>>abc123\n\
    activewindow>>firefox,SEGREDO-T\n\
    windowtitlev2>>abc123,SEGREDO-titulo-novo\n\
    openwindow>>abc124,SEGREDO-area,SEGREDO-classe,SEGREDO-janela\n\
    workspacev2>>2,SEGREDO-area-de-trabalho\n\
    activewindowv2>>fff999\n";

/// Espera `condicao` no `/v1/estado` (até 15 s: o laço pode estar no
/// handshake com prazo do Wayland de mentira).
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
        thread::sleep(Duration::from_millis(50));
    }
}

fn sem_segredo(onde: &str, texto: &str) {
    let minusculo = texto.to_lowercase();
    assert!(!minusculo.contains("segredo"), "{onde}: vazou: {texto}");
    assert!(
        !minusculo.contains("firefox"),
        "{onde}: a classe vazou: {texto}"
    );
}

#[test]
fn o_socket2_chega_ao_estado_sem_titulo_nem_classe() {
    let h = HyprlandFalso::novo("canario", LINHAS);
    let d = Daemon::subir_com(true, &[("PET_HOST_RUNTIME", h.runtime().as_str())]);
    // A sentinela (a última linha) chegou: todas as outras já passaram.
    let estado = esperar(&d, "eventos do Hyprland", |e| {
        e["desktop"]["eventos"] == "ligado" && e["desktop"]["janela_ativa"] == "fff999"
    });
    let desktop = &estado["desktop"];
    assert_eq!(desktop["monitor_em_foco"], "HDMI-A-1");
    assert_eq!(
        desktop["olhando_claude"], false,
        "o título não começa com ✳"
    );
    let anel = desktop["anel"].as_array().expect("o anel");
    assert_eq!(anel.len(), 2, "{anel:?}");
    assert_eq!(anel[0]["janela"], "fff999", "a mais nova primeiro");
    assert_eq!(anel[1]["janela"], "abc123");
    assert_eq!(anel[0]["tipo"], "troca");
    assert!(anel[0]["em_ms"].as_u64().unwrap() > 1_700_000_000_000);
    // O canário em todo lugar que sai do pet.
    sem_segredo("/v1/estado", &estado.to_string());
    let (status, eventos) = d.get("/v1/debug/eventos");
    assert_eq!(status, 200);
    sem_segredo("/v1/debug/eventos", &eventos);
    let log = d.log();
    assert!(log.contains("eventos do Hyprland: ligado"), "{log}");
    sem_segredo("log", &log);
    assert!(h.comandos_intocado(), "alguém conectou no .socket.sock");
}

#[test]
fn o_glifo_do_claude_vira_so_o_booleano() {
    // O título do terminal do Claude Code (✳ parado) na janela em foco: só o
    // booleano sai, nada do resto do título.
    let h = HyprlandFalso::novo(
        "glifo",
        "activewindowv2>>def456\nactivewindow>>foot,\u{2733} SEGREDO-tarefa\n",
    );
    let d = Daemon::subir_com(true, &[("PET_HOST_RUNTIME", h.runtime().as_str())]);
    let estado = esperar(&d, "presença", |e| {
        e["desktop"]["olhando_claude"] == true && e["desktop"]["janela_ativa"] == "def456"
    });
    sem_segredo("/v1/estado", &estado.to_string());
    sem_segredo("log", &d.log());
    let (_, eventos) = d.get("/v1/debug/eventos");
    sem_segredo("/v1/debug/eventos", &eventos);
}

#[test]
fn a_protecao_de_tela_do_omarchy_chega_como_booleano() {
    let h = HyprlandFalso::novo(
        "protetor",
        "openwindow>>aaa111,1,org.omarchy.screensaver,SEGREDO-protetor\nactivewindowv2>>aaa111\n",
    );
    let d = Daemon::subir_com(true, &[("PET_HOST_RUNTIME", h.runtime().as_str())]);
    let estado = esperar(&d, "proteção de tela", |e| {
        e["desktop"]["protetor_de_tela"] == true && e["desktop"]["janela_ativa"] == "aaa111"
    });
    sem_segredo("/v1/estado", &estado.to_string());
    sem_segredo("log", &d.log());
    assert!(d.log().contains("proteção de tela abriu"), "{}", d.log());
}

/// Sem a conexão Wayland (o compositor de mentira desliga na hora e o laço
/// fica no backoff) o socket2 continua lido: um `focusedmonv2` arma o prazo
/// de seguir o monitor, que tem de vencer mesmo sem a janela. Antes da
/// revisão do M4 o prazo vencido ficava armado e o laço girava a 100% de
/// CPU até a conexão voltar (decisão 0059).
#[test]
fn sem_a_conexao_wayland_o_foco_de_monitor_nao_gira_o_laco() {
    let h = HyprlandFalso::novo("cpu", "focusedmonv2>>HDMI-A-1,3\n");
    let d = Daemon::subir_com(false, &[("PET_HOST_RUNTIME", h.runtime().as_str())]);
    esperar(&d, "o monitor em foco", |e| {
        e["desktop"]["monitor_em_foco"] == "HDMI-A-1"
    });
    // Passado o debounce de 300 ms, o prazo já venceu sem janela.
    thread::sleep(Duration::from_millis(600));
    let antes = d.cpu_tiques();
    thread::sleep(Duration::from_secs(2));
    let gasto = d.cpu_tiques() - antes;
    // Girando, seriam ~200 tiques (2 s de um núcleo a 100 por segundo).
    assert!(
        gasto < 40,
        "o laço girou sem a conexão: {gasto} tiques de CPU em 2 s\nlog:\n{}",
        d.log()
    );
    assert!(h.comandos_intocado(), "alguém conectou no .socket.sock");
}
