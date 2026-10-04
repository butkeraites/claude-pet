//! Uma instância de mentira do Hyprland para o daemon de verdade (decisão
//! 0050): o `hyprland.lock`, um `.socket2.sock` que manda linhas (as do
//! começo a cada conexão, e depois as que o teste pedir) e um socket Wayland
//! que desliga na hora (o handshake falha e o laço espera o backoff, mas o
//! socket2 já é lido). O `.socket.sock` existe só para provar que ninguém
//! conecta nele.

#![allow(dead_code)] // cada arquivo de teste usa uma parte

use std::io::{ErrorKind, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

pub struct HyprlandFalso {
    /// A pasta que faz de `/run/user` (`PET_HOST_RUNTIME`).
    pub raiz: PathBuf,
    comandos: UnixListener,
    linhas: mpsc::Sender<String>,
}

impl Drop for HyprlandFalso {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.raiz);
    }
}

impl HyprlandFalso {
    /// Numa pasta curta (o AF_UNIX tem limite de 108 bytes no caminho).
    pub fn novo(nome: &str, iniciais: &'static str) -> HyprlandFalso {
        let raiz = std::env::temp_dir().join(format!("bichinho-h-{}-{nome}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        std::fs::create_dir_all(&raiz).unwrap();
        let uid = std::fs::metadata(&raiz).unwrap().uid();
        let base = raiz.join(uid.to_string());
        let instancia = base.join("hypr").join("a_1790000000_1");
        std::fs::create_dir_all(&instancia).unwrap();
        std::fs::write(instancia.join("hyprland.lock"), "1\nwayland-9\n").unwrap();
        let wayland = UnixListener::bind(base.join("wayland-9")).unwrap();
        thread::spawn(move || {
            for conexao in wayland.incoming() {
                drop(conexao);
            }
        });
        let abertas: Arc<Mutex<Vec<UnixStream>>> = Arc::default();
        let eventos = UnixListener::bind(instancia.join(".socket2.sock")).unwrap();
        let delas = Arc::clone(&abertas);
        thread::spawn(move || {
            for conexao in eventos.incoming() {
                let Ok(mut fluxo) = conexao else {
                    continue;
                };
                let _ = fluxo.write_all(iniciais.as_bytes());
                delas.lock().unwrap().push(fluxo);
            }
        });
        let (linhas, recebe) = mpsc::channel::<String>();
        thread::spawn(move || {
            for texto in recebe {
                // As conexões da prova da descoberta já fecharam: a escrita
                // nelas falha e elas saem da lista.
                abertas
                    .lock()
                    .unwrap()
                    .retain_mut(|f| f.write_all(texto.as_bytes()).is_ok());
            }
        });
        let comandos = UnixListener::bind(instancia.join(".socket.sock")).unwrap();
        comandos.set_nonblocking(true).unwrap();
        HyprlandFalso {
            raiz,
            comandos,
            linhas,
        }
    }

    /// O `PET_HOST_RUNTIME` do daemon.
    pub fn runtime(&self) -> String {
        self.raiz.display().to_string()
    }

    /// Manda linhas do socket2 às conexões abertas (a do leitor).
    pub fn mandar(&self, linhas: &str) {
        self.linhas.send(linhas.to_owned()).unwrap();
    }

    /// Ninguém conectou no socket de comandos.
    pub fn comandos_intocado(&self) -> bool {
        matches!(self.comandos.accept(), Err(e) if e.kind() == ErrorKind::WouldBlock)
    }
}
