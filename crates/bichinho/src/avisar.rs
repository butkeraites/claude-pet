//! `bichinho avisar <Evento>`: o hook nativo do plugin (decisão 0041).
//!
//! Lê o JSON do hook na entrada padrão, monta só os metadados pela lista
//! branca do core ([`pet_core::aviso`], com os validadores do fio v1) e manda
//! ao pet do 127.0.0.1 por uma conexão TCP direta: nenhum proxy, nenhum
//! curlrc, nenhum jq, nenhum shell. Regras de ouro (CLAUDE.md):
//!
//! - só METADADOS saem; prompt, código, resposta, texto de erro, títulos e
//!   caminhos nunca saem do host nem vão para log (o caminho editado vira o
//!   hash dele, aqui);
//! - não imprime nada, nem num pânico;
//! - SEMPRE sai 0 (um Stop hook que saísse com 2 seguraria o Claude) e nunca
//!   passa de [`PRAZO_TOTAL`], nem com a entrada padrão aberta para sempre.
//!
//! Ambiente: `PET_PORTA` (porta do pet, padrão 27380), `PET_TESTE=1` (evento
//! sintético do `bin/pet testar`: o pet o isola das sessões reais e o
//! esquece em 60 s), `CLAUDE_CODE_ENTRYPOINT` (posto pelo Claude Code) e
//! `XDG_STATE_HOME`/`HOME` (onde o Omarchy guarda o "não perturbe").

use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pet_core::aviso::{self, Contexto};

/// Com o pet desligado, a conexão é recusada na hora; isto é para uma porta
/// que não responde.
pub const PRAZO_CONEXAO: Duration = Duration::from_millis(300);
/// Conexão, envio e resposta juntos.
pub const PRAZO_POST: Duration = Duration::from_secs(2);
/// O hook inteiro: depois disto ele sai 0, aconteça o que acontecer.
pub const PRAZO_TOTAL: Duration = Duration::from_secs(4);
/// Teto da entrada lida (um prompt colado de 8 MiB passa folgado); além
/// disso o JSON não fecha e vai o mínimo.
const LIMITE_ENTRADA: u64 = 64 * 1024 * 1024;
/// Teto do arquivo do "não perturbe" (ele guarda o histórico das
/// notificações, que nunca sai daqui).
const LIMITE_DND: u64 = 16 * 1024 * 1024;

/// O subcomando. `argumentos` é o que vem depois de `avisar`.
pub fn rodar(mut argumentos: impl Iterator<Item = String>) -> ExitCode {
    // Pânico sai 0, calado: o gancho padrão imprimiria no stderr.
    std::panic::set_hook(Box::new(|_| std::process::exit(0)));
    let _ = std::thread::Builder::new().name("prazo".into()).spawn(|| {
        std::thread::sleep(PRAZO_TOTAL);
        std::process::exit(0);
    });
    let Some(evento) = argumentos.next().filter(|e| aviso::evento_valido(e)) else {
        return ExitCode::SUCCESS;
    };
    // A hora do evento, antes de tudo: os hooks async chegam ao pet fora de
    // ordem, e é por ela que o pet ordena.
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64);
    let porta = aviso::porta(std::env::var("PET_PORTA").ok().as_deref());
    let teste = std::env::var("PET_TESTE").ok().as_deref() == Some("1");
    let ent = std::env::var("CLAUDE_CODE_ENTRYPOINT").ok();
    // O JSON do hook fica só na memória deste processo.
    let mut entrada = Vec::new();
    let _ = std::io::stdin()
        .lock()
        .take(LIMITE_ENTRADA)
        .read_to_end(&mut entrada);
    let contexto = Contexto {
        evento: &evento,
        ts,
        ent: ent.as_deref(),
        dnd: nao_perturbe(),
        teste,
    };
    let corpo = aviso::corpo(&entrada, &contexto);
    drop(entrada);
    let _ = enviar(porta, &corpo);
    ExitCode::SUCCESS
}

/// O "não perturbe" do Omarchy, em
/// `${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/notifications.json`. Só o
/// booleano sai do arquivo.
fn nao_perturbe() -> bool {
    let base = match std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
        Some(estado) => PathBuf::from(estado),
        None => {
            let mut casa: OsString = std::env::var_os("HOME").unwrap_or_default();
            casa.push("/.local/state");
            PathBuf::from(casa)
        }
    };
    let Ok(arquivo) = std::fs::File::open(base.join("omarchy/notifications.json")) else {
        return false;
    };
    let mut conteudo = Vec::new();
    match arquivo.take(LIMITE_DND + 1).read_to_end(&mut conteudo) {
        Ok(n) if n as u64 <= LIMITE_DND => aviso::dnd_do_omarchy(&conteudo),
        _ => false,
    }
}

/// `POST /v1/evento` no 127.0.0.1, com prazo curto, e espera a resposta (o
/// pet responde 204 na hora) para o evento não se perder no fim do processo.
fn enviar(porta: u16, corpo: &str) -> std::io::Result<()> {
    let inicio = Instant::now();
    let resta = || {
        PRAZO_POST
            .saturating_sub(inicio.elapsed())
            .max(Duration::from_millis(1))
    };
    let alvo = SocketAddr::from((Ipv4Addr::LOCALHOST, porta));
    let mut fluxo = TcpStream::connect_timeout(&alvo, PRAZO_CONEXAO)?;
    fluxo.set_write_timeout(Some(resta()))?;
    let cabecalhos = format!(
        "POST /v1/evento HTTP/1.1\r\nHost: 127.0.0.1:{porta}\r\nContent-Type: application/json\r\nX-Pet: 1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        corpo.len()
    );
    fluxo.write_all(cabecalhos.as_bytes())?;
    fluxo.write_all(corpo.as_bytes())?;
    fluxo.flush()?;
    let mut resposta = [0u8; 512];
    while inicio.elapsed() < PRAZO_POST {
        fluxo.set_read_timeout(Some(resta()))?;
        match fluxo.read(&mut resposta) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
    Ok(())
}
