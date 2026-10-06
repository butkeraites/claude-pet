//! Apoio dos testes de integração: o daemon de verdade (o binário que o
//! cargo acabou de compilar) numa porta livre, sem compositor e com tudo em
//! pastas temporárias. Nunca toca o Hyprland nem o pet de produção: o
//! runtime do host aponta para uma pasta que não existe.

#![allow(dead_code)] // cada arquivo de teste usa uma parte

pub mod canarios;
pub mod hyprland;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

static CONTADOR: AtomicU32 = AtomicU32::new(0);

/// Raiz do repositório.
pub fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("raiz do repositório")
}

/// Pasta temporária própria do teste, dentro de `target/tmp` (apagada no
/// fim por quem a criou).
pub fn pasta_temporaria(nome: &str) -> PathBuf {
    let n = CONTADOR.fetch_add(1, Ordering::Relaxed);
    let pasta =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{nome}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&pasta);
    std::fs::create_dir_all(&pasta).expect("pasta temporária");
    pasta
}

/// O daemon rodando; morre (SIGKILL) quando sai de escopo.
pub struct Daemon {
    filho: Child,
    pub porta: u16,
    pub pasta: PathBuf,
}

impl Daemon {
    /// Sobe o daemon com `PET_LOG=debug` (para os canários olharem o log) e,
    /// se `debug`, `PET_DEBUG=1`.
    pub fn subir(debug: bool) -> Daemon {
        Daemon::subir_com(debug, &[])
    }

    pub fn subir_com(debug: bool, extra: &[(&str, &str)]) -> Daemon {
        let pasta = pasta_temporaria("daemon");
        for tentativa in 0..5 {
            // Porta livre agora; outro processo pode pegá-la antes do daemon,
            // por isso as tentativas.
            let porta = TcpListener::bind("127.0.0.1:0")
                .and_then(|o| o.local_addr())
                .expect("porta livre")
                .port();
            let log = std::fs::File::create(pasta.join("daemon.log")).expect("log");
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_bichinho"));
            cmd.arg("rodar")
                .env_clear()
                .env("HOME", &pasta)
                .env("PET_LOG", "debug")
                .env("PET_ESCUTA", format!("127.0.0.1:{porta}"))
                .env("PET_PORTA_PUBLICA", porta.to_string())
                .env("PET_HOST_RUNTIME", pasta.join("sem-runtime"))
                .env("PET_CONFIG", pasta.join("config"))
                .env("PET_ESTADO", pasta.join("estado"))
                .env("PET_SKINS", raiz().join("skins"))
                .env("XDG_RUNTIME_DIR", pasta.join("xdg"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::from(log));
            if debug {
                cmd.env("PET_DEBUG", "1");
            }
            for (nome, valor) in extra {
                cmd.env(nome, valor);
            }
            let mut filho = cmd.spawn().expect("subir o daemon");
            if esperar_de_pe(&mut filho, porta) {
                return Daemon {
                    filho,
                    porta,
                    pasta,
                };
            }
            let _ = filho.kill();
            let _ = filho.wait();
            eprintln!("daemon não subiu na tentativa {tentativa}; tentando outra porta");
        }
        let _ = std::fs::remove_dir_all(&pasta);
        panic!("o daemon não subiu em 5 tentativas");
    }

    /// Manda bytes crus e devolve (status, corpo); status 0 se a conexão
    /// falhou.
    pub fn bruto(&self, requisicao: &str) -> (u16, String) {
        bruto(self.porta, requisicao)
    }

    /// Cabeçalhos certos (Host de loopback na porta, `X-Pet: 1`).
    pub fn cabecalhos(&self) -> String {
        format!("Host: 127.0.0.1:{}\r\nX-Pet: 1\r\n", self.porta)
    }

    pub fn get(&self, caminho: &str) -> (u16, String) {
        self.bruto(&format!(
            "GET {caminho} HTTP/1.1\r\n{}\r\n",
            self.cabecalhos()
        ))
    }

    pub fn get_json(&self, caminho: &str) -> Value {
        let (status, corpo) = self.get(caminho);
        assert_eq!(status, 200, "GET {caminho}: {corpo}");
        serde_json::from_str(&corpo).expect("JSON")
    }

    pub fn post_json(&self, caminho: &str, corpo: &str) -> (u16, String) {
        self.bruto(&format!(
            "POST {caminho} HTTP/1.1\r\n{}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{corpo}",
            self.cabecalhos(),
            corpo.len()
        ))
    }

    /// O PID do daemon.
    pub fn pid(&self) -> u32 {
        self.filho.id()
    }

    /// Para o daemon com um SIGTERM (como o `docker stop`) e espera ele sair
    /// (até 5 s, o `stop_grace_period` do compose). `true` se saiu sozinho,
    /// com sucesso.
    pub fn parar(&mut self) -> bool {
        let mandou = Command::new("kill")
            .args(["-TERM", &self.pid().to_string()])
            .status()
            .is_ok_and(|s| s.success());
        let limite = Instant::now() + Duration::from_secs(5);
        while mandou && Instant::now() < limite {
            if let Ok(Some(status)) = self.filho.try_wait() {
                return status.success();
            }
            thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// CPU que o daemon gastou até agora (usuário e sistema), em tiques do
    /// relógio do kernel (`/proc/<pid>/stat`, campos 14 e 15). O `stat` se
    /// lê mesmo com o processo sem core dump (dono root, modo 0444).
    pub fn cpu_tiques(&self) -> u64 {
        let stat = std::fs::read_to_string(format!("/proc/{}/stat", self.pid()))
            .expect("/proc/<pid>/stat do daemon");
        // O nome do processo vem entre parênteses e pode ter espaço: os
        // campos contam depois do último `)`.
        let depois = &stat[stat.rfind(')').expect("stat sem nome") + 2..];
        let campos: Vec<&str> = depois.split(' ').collect();
        // Depois do `)`, o estado é o campo 3: utime (14) e stime (15) são
        // os índices 11 e 12.
        campos[11].parse::<u64>().unwrap() + campos[12].parse::<u64>().unwrap()
    }

    /// O que o daemon escreveu no stderr até agora.
    pub fn log(&self) -> String {
        std::fs::read_to_string(self.pasta.join("daemon.log")).unwrap_or_default()
    }

    /// Espera `condicao` ficar verdadeira no `/v1/estado` (até 5 s).
    pub fn esperar_estado(&self, descricao: &str, condicao: impl Fn(&Value) -> bool) -> Value {
        let limite = Instant::now() + Duration::from_secs(5);
        loop {
            let estado = self.get_json("/v1/estado");
            if condicao(&estado) {
                return estado;
            }
            assert!(
                Instant::now() < limite,
                "{descricao}: o estado não chegou lá: {estado:#}"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Espera o `/saude` responder 200 (até 10 s); `false` se o processo morreu
/// antes (porta ocupada, por exemplo) ou não respondeu.
fn esperar_de_pe(filho: &mut Child, porta: u16) -> bool {
    let limite = Instant::now() + Duration::from_secs(10);
    while Instant::now() < limite {
        if let Ok(Some(_)) = filho.try_wait() {
            return false;
        }
        if bruto(porta, "GET /saude HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").0 == 200 {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

/// Bytes crus para a porta; (status, corpo), status 0 se a conexão falhou.
pub fn bruto(porta: u16, requisicao: &str) -> (u16, String) {
    let Ok(mut fluxo) = TcpStream::connect(("127.0.0.1", porta)) else {
        return (0, String::new());
    };
    let _ = fluxo.set_read_timeout(Some(Duration::from_secs(5)));
    if fluxo.write_all(requisicao.as_bytes()).is_err() {
        return (0, String::new());
    }
    let mut resposta = String::new();
    let _ = fluxo.read_to_string(&mut resposta);
    let status = resposta
        .split(' ')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let corpo = resposta
        .split_once("\r\n\r\n")
        .map(|(_, c)| c.to_owned())
        .unwrap_or_default();
    (status, corpo)
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.filho.kill();
        let _ = self.filho.wait();
        let _ = std::fs::remove_dir_all(&self.pasta);
    }
}
