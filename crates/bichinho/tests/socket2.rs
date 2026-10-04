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

use std::io::{ErrorKind, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use comum::Daemon;
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

/// Uma instância de mentira do Hyprland numa pasta curta (o AF_UNIX tem
/// limite de 108 bytes no caminho).
struct Hyprland {
    raiz: PathBuf,
    comandos: UnixListener,
}

impl Drop for Hyprland {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.raiz);
    }
}

fn hyprland_de_mentira(nome: &str, linhas: &'static str) -> Hyprland {
    let raiz = std::env::temp_dir().join(format!("bichinho-s2-{}-{nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&raiz);
    std::fs::create_dir_all(&raiz).unwrap();
    let uid = std::fs::metadata(&raiz).unwrap().uid();
    let base = raiz.join(uid.to_string());
    let instancia = base.join("hypr").join("a_1790000000_1");
    std::fs::create_dir_all(&instancia).unwrap();
    std::fs::write(instancia.join("hyprland.lock"), "1\nwayland-9\n").unwrap();
    // O Wayland desliga cada conexão na hora: o handshake falha rápido.
    let wayland = UnixListener::bind(base.join("wayland-9")).unwrap();
    thread::spawn(move || {
        for conexao in wayland.incoming() {
            drop(conexao);
        }
    });
    // O socket2 manda as linhas a cada conexão e a segura aberta (a prova da
    // descoberta fecha a dela na hora; a do leitor fica).
    let eventos = UnixListener::bind(instancia.join(".socket2.sock")).unwrap();
    thread::spawn(move || {
        let mut abertas = Vec::new();
        for conexao in eventos.incoming() {
            let Ok(mut fluxo) = conexao else {
                continue;
            };
            let _ = fluxo.write_all(linhas.as_bytes());
            abertas.push(fluxo);
        }
    });
    let comandos = UnixListener::bind(instancia.join(".socket.sock")).unwrap();
    comandos.set_nonblocking(true).unwrap();
    Hyprland { raiz, comandos }
}

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

fn runtime(h: &Hyprland) -> String {
    Path::new(&h.raiz).display().to_string()
}

#[test]
fn o_socket2_chega_ao_estado_sem_titulo_nem_classe() {
    let h = hyprland_de_mentira("canario", LINHAS);
    let d = Daemon::subir_com(true, &[("PET_HOST_RUNTIME", runtime(&h).as_str())]);
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
    // O socket de comandos nunca foi aberto.
    match h.comandos.accept() {
        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
        outro => panic!("alguém conectou no .socket.sock: {outro:?}"),
    }
}

#[test]
fn o_glifo_do_claude_vira_so_o_booleano() {
    // O título do terminal do Claude Code (✳ parado) na janela em foco: só o
    // booleano sai, nada do resto do título.
    let h = hyprland_de_mentira(
        "glifo",
        "activewindowv2>>def456\nactivewindow>>foot,\u{2733} SEGREDO-tarefa\n",
    );
    let d = Daemon::subir_com(true, &[("PET_HOST_RUNTIME", runtime(&h).as_str())]);
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
    let h = hyprland_de_mentira(
        "protetor",
        "openwindow>>aaa111,1,org.omarchy.screensaver,SEGREDO-protetor\nactivewindowv2>>aaa111\n",
    );
    let d = Daemon::subir_com(true, &[("PET_HOST_RUNTIME", runtime(&h).as_str())]);
    let estado = esperar(&d, "proteção de tela", |e| {
        e["desktop"]["protetor_de_tela"] == true && e["desktop"]["janela_ativa"] == "aaa111"
    });
    sem_segredo("/v1/estado", &estado.to_string());
    sem_segredo("log", &d.log());
    assert!(d.log().contains("proteção de tela abriu"), "{}", d.log());
}
