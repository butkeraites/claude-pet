//! Ida e volta ao compositor, com prazo.
//!
//! O `roundtrip()` do wayland-client espera para sempre. Aqui o pedido é o
//! mesmo (`wl_display.sync`), mas a espera tem prazo. Serve para duas coisas:
//!
//! - **antes do handshake:** um `connect()` aceito só prova que o kernel pôs
//!   a conexão na fila; um compositor travado (ou ainda subindo) não lê. Sem
//!   prazo, o laço principal ficaria preso e o vigia abortaria a cada 60 s;
//! - **antes de sair:** o libwayland-server destrói um cliente que desligou
//!   sem ler o que ainda estava no socket. Esperar a resposta do `sync` prova
//!   que o compositor processou o quadro transparente e a destruição da
//!   camada, então o fade de saída do Hyprland não guarda o pet (fantasma).

use std::io::ErrorKind;
use std::os::fd::OwnedFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};
use smithay_client_toolkit::reexports::client::Connection;
use smithay_client_toolkit::reexports::client::backend::protocol::Message;
use smithay_client_toolkit::reexports::client::backend::{
    Backend, ObjectData, ObjectId, WaylandError,
};
use smithay_client_toolkit::reexports::client::protocol::wl_display;

/// Marca a chegada do `wl_callback.done` do `sync`.
#[derive(Debug, Default)]
struct Sinal(AtomicBool);

impl ObjectData for Sinal {
    fn event(
        self: Arc<Self>,
        _: &Backend,
        _: Message<ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData>> {
        self.0.store(true, Ordering::Relaxed);
        None
    }

    fn destroyed(&self, _: ObjectId) {}
}

/// Manda um `wl_display.sync` e espera o compositor responder, por no
/// máximo `limite`. `Ok` prova que tudo o que foi pedido antes nesta conexão
/// já foi processado.
pub fn sincronizar(conexao: &Connection, limite: Duration) -> Result<(), String> {
    let sinal = Arc::new(Sinal::default());
    conexao
        .send_request(
            &conexao.display(),
            wl_display::Request::Sync {},
            Some(sinal.clone()),
        )
        .map_err(|_| "a conexão já estava fechada".to_owned())?;
    let prazo = Instant::now() + limite;
    let estourou = || format!("o compositor não respondeu em {} ms", limite.as_millis());
    while !sinal.0.load(Ordering::Relaxed) {
        conexao.flush().map_err(|e| format!("envio: {e}"))?;
        let Some(guarda) = conexao.prepare_read() else {
            conexao
                .backend()
                .dispatch_inner_queue()
                .map_err(|e| format!("leitura: {e}"))?;
            continue;
        };
        let falta = prazo.saturating_duration_since(Instant::now());
        if falta.is_zero() {
            return Err(estourou());
        }
        let espera = Timespec::try_from(falta).map_err(|_| estourou())?;
        let prontos = {
            let fd = guarda.connection_fd();
            let mut fds = [PollFd::new(&fd, PollFlags::IN | PollFlags::ERR)];
            match rustix::event::poll(&mut fds, Some(&espera)) {
                Ok(n) => n,
                Err(rustix::io::Errno::INTR) => continue,
                Err(e) => return Err(format!("poll: {e}")),
            }
        };
        if prontos == 0 {
            return Err(estourou());
        }
        match guarda.read() {
            Ok(_) => {}
            Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => return Err(format!("leitura: {e}")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    use super::*;

    /// Cabeçalho de mensagem Wayland: objeto, tamanho << 16 | opcode.
    fn mensagem(objeto: u32, opcode: u16, argumento: u32) -> Vec<u8> {
        let mut m = Vec::with_capacity(12);
        m.extend_from_slice(&objeto.to_ne_bytes());
        m.extend_from_slice(&((12u32 << 16) | opcode as u32).to_ne_bytes());
        m.extend_from_slice(&argumento.to_ne_bytes());
        m
    }

    #[test]
    fn compositor_que_nao_le_estoura_o_prazo() {
        let (cliente, _servidor_mudo) = UnixStream::pair().unwrap();
        let conexao = Connection::from_socket(cliente).unwrap();
        let t0 = Instant::now();
        let erro = sincronizar(&conexao, Duration::from_millis(150)).unwrap_err();
        assert!(erro.contains("não respondeu"), "{erro}");
        let levou = t0.elapsed();
        assert!(levou >= Duration::from_millis(150), "{levou:?}");
        assert!(levou < Duration::from_secs(2), "{levou:?}");
    }

    #[test]
    fn resposta_do_sync_confirma() {
        let (cliente, mut servidor) = UnixStream::pair().unwrap();
        let fio = std::thread::spawn(move || {
            // wl_display(1).sync(new_id): 12 bytes.
            let mut pedido = [0u8; 12];
            servidor.read_exact(&mut pedido).unwrap();
            assert_eq!(u32::from_ne_bytes(pedido[0..4].try_into().unwrap()), 1);
            let id = u32::from_ne_bytes(pedido[8..12].try_into().unwrap());
            // wl_callback(id).done(0) e wl_display.delete_id(id).
            servidor.write_all(&mensagem(id, 0, 0)).unwrap();
            servidor.write_all(&mensagem(1, 1, id)).unwrap();
            servidor
        });
        let conexao = Connection::from_socket(cliente).unwrap();
        sincronizar(&conexao, Duration::from_secs(5)).unwrap();
        drop(fio.join().unwrap());
    }

    #[test]
    fn compositor_que_fecha_da_erro_na_hora() {
        let (cliente, servidor) = UnixStream::pair().unwrap();
        drop(servidor);
        let conexao = Connection::from_socket(cliente).unwrap();
        let t0 = Instant::now();
        assert!(sincronizar(&conexao, Duration::from_secs(5)).is_err());
        assert!(t0.elapsed() < Duration::from_secs(1));
    }
}
