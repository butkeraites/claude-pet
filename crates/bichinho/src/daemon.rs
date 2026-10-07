//! Subcomandos `rodar` (o daemon) e `saude` (healthcheck do Docker).
//!
//! `rodar` faz o que é igual em todo sistema (ambiente, config, estado
//! compartilhado, a porta da entrada HTTP) e entrega ao laço do sistema
//! (decisão 0040): no Linux, o calloop com o Wayland (`laco`); no Windows e
//! no macOS, por enquanto, o laço sem janela ([`crate::sem_janela`]).

use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use pet_core::config::ConfigEfetiva;
use pet_core::plataforma::Caixa;

use crate::ambiente::Ambiente;
use crate::comando::Comando;
use crate::estado::Compartilhado;
use crate::ingress;

pub fn rodar() -> ExitCode {
    // Antes de tudo (o leitor do socket2 e a conexão Wayland trazem títulos
    // de janela para a memória): nenhum core dump (decisão 0061).
    crate::privacidade::sem_core_dump();
    let ambiente = match Ambiente::ler(|nome| std::env::var(nome).ok()) {
        Ok(a) => a,
        Err(motivo) => {
            erro!("{motivo}");
            return ExitCode::from(2);
        }
    };
    #[cfg(unix)]
    if let Err(e) = preparar_runtime_privado() {
        aviso!("XDG_RUNTIME_DIR privado: {e}");
    }

    let arquivo = ambiente.arquivo_config();
    if arquivo.ends_with(crate::ambiente::ARQUIVO_CONFIG_ANTIGO) {
        aviso!(
            "config: lendo o nome antigo {}; renomeie para {} (decisão 0041)",
            arquivo.display(),
            crate::ambiente::ARQUIVO_CONFIG
        );
    }
    let config = carregar_config(&arquivo);
    for aviso in &config.avisos {
        aviso!("config: {aviso}");
    }
    let comp = Arc::new(Compartilhado::novo(config.clone(), ambiente.debug));
    comp.bater();

    let ouvinte = match TcpListener::bind(ambiente.escuta) {
        Ok(o) => o,
        Err(e) => {
            erro!("não consegui escutar em {}: {e}", ambiente.escuta);
            return ExitCode::FAILURE;
        }
    };
    rodar_no_sistema(ambiente, config, comp, ouvinte)
}

#[cfg(target_os = "linux")]
fn rodar_no_sistema(
    ambiente: Ambiente,
    config: ConfigEfetiva,
    comp: Arc<Compartilhado>,
    ouvinte: TcpListener,
) -> ExitCode {
    crate::laco::rodar(ambiente, config, comp, ouvinte)
}

/// macOS (M8, T8.5): o painel nativo (NSPanel + CALayer) pelo laço do AppKit.
/// Com `PET_SEM_JANELA=1`, o laço sem janela (os testes do daemon não abrem um
/// NSPanel na tela; o cérebro e o `/v1/estado` funcionam igual).
#[cfg(target_os = "macos")]
fn rodar_no_sistema(
    ambiente: Ambiente,
    config: ConfigEfetiva,
    comp: Arc<Compartilhado>,
    ouvinte: TcpListener,
) -> ExitCode {
    if std::env::var("PET_SEM_JANELA").as_deref() == Ok("1") {
        aviso!("PET_SEM_JANELA=1: laço sem janela (sem NSPanel)");
        return crate::sem_janela::rodar(
            ambiente,
            config,
            comp,
            ouvinte,
            "PET_SEM_JANELA=1: sem janela",
        );
    }
    crate::laco_macos::rodar(ambiente, config, comp, ouvinte)
}

/// Windows (M8): a janela nativa layered (`UpdateLayeredWindow`) pelo laço de
/// mensagens. Com `PET_SEM_JANELA=1`, o laço sem janela (os testes do daemon não
/// abrem uma janela na tela; o cérebro e o `/v1/estado` funcionam igual).
#[cfg(windows)]
fn rodar_no_sistema(
    ambiente: Ambiente,
    config: ConfigEfetiva,
    comp: Arc<Compartilhado>,
    ouvinte: TcpListener,
) -> ExitCode {
    if std::env::var("PET_SEM_JANELA").as_deref() == Ok("1") {
        aviso!("PET_SEM_JANELA=1: laço sem janela (sem janela nativa)");
        return crate::sem_janela::rodar(
            ambiente,
            config,
            comp,
            ouvinte,
            "PET_SEM_JANELA=1: sem janela",
        );
    }
    crate::laco_windows::rodar(ambiente, config, comp, ouvinte)
}

/// Sem backend de janela ainda: o laço sem janela, com o cérebro e o
/// `/v1/estado`.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn rodar_no_sistema(
    ambiente: Ambiente,
    config: ConfigEfetiva,
    comp: Arc<Compartilhado>,
    ouvinte: TcpListener,
) -> ExitCode {
    crate::sem_janela::rodar(
        ambiente,
        config,
        comp,
        ouvinte,
        "este sistema ainda não tem backend de janela",
    )
}

/// Liga a entrada HTTP (uma thread) à caixa do laço do sistema.
pub fn iniciar_entrada(
    ambiente: &Ambiente,
    comp: &Arc<Compartilhado>,
    ouvinte: TcpListener,
    caixa: Caixa<Comando>,
) -> Result<(), String> {
    // O observador de transcript usa a mesma caixa do laço (o clone carrega o
    // Despertador da plataforma); liga antes de a caixa ir para o ingress.
    crate::observador::iniciar(Arc::clone(comp), caixa.clone());
    let ctx = Arc::new(ingress::Contexto {
        comp: Arc::clone(comp),
        porta_publica: ambiente.porta_publica,
        debug: ambiente.debug,
        comandos: Some(caixa),
        onde: ambiente.onde(),
        aprovando: Mutex::new(()),
    });
    thread::Builder::new()
        .name("ingress".into())
        .spawn(move || ingress::servir(ouvinte, ctx))
        .map(|_| ())
        .map_err(|e| format!("não consegui iniciar a entrada HTTP: {e}"))
}

/// `bichinho saude`: pergunta ao próprio daemon se o laço principal está
/// vivo. Sai 0 se sim, 1 se não.
pub fn saude() -> ExitCode {
    let escuta = match Ambiente::ler(|nome| std::env::var(nome).ok()) {
        Ok(a) => a.escuta,
        Err(motivo) => {
            eprintln!("{motivo}");
            return ExitCode::FAILURE;
        }
    };
    if checar_saude(escuta) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

pub(crate) fn checar_saude(escuta: SocketAddr) -> bool {
    let ip = if escuta.ip().is_unspecified() {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
        escuta.ip()
    };
    let alvo = SocketAddr::new(ip, escuta.port());
    let limite = Duration::from_secs(3);
    let Ok(mut fluxo) = TcpStream::connect_timeout(&alvo, limite) else {
        return false;
    };
    let _ = fluxo.set_read_timeout(Some(limite));
    let _ = fluxo.set_write_timeout(Some(limite));
    let pedido = format!(
        "GET /saude HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        escuta.port()
    );
    if fluxo.write_all(pedido.as_bytes()).is_err() {
        return false;
    }
    let mut resposta = Vec::new();
    let _ = fluxo.take(4096).read_to_end(&mut resposta);
    resposta.starts_with(b"HTTP/1.1 200 ")
}

/// Config efetiva: o arquivo (se houver), depois as variáveis `PET_*`.
pub fn carregar_config(caminho: &Path) -> ConfigEfetiva {
    let texto = match std::fs::read_to_string(caminho) {
        Ok(texto) => Some(texto),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => {
            aviso!("não consegui ler {}: {e}", caminho.display());
            None
        }
    };
    ConfigEfetiva::carregar(texto.as_deref(), |nome| std::env::var(nome).ok())
}

/// Cria o `XDG_RUNTIME_DIR` do container (privado, 0700) se ele não existir.
/// Nada daqui escreve no runtime do host (decisão 0007).
#[cfg(unix)]
fn preparar_runtime_privado() -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    let Some(caminho) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return Ok(());
    };
    let caminho = Path::new(&caminho);
    if caminho.starts_with("/run/user") || caminho.starts_with("/host") {
        return Ok(()); // rodando fora do container: é o runtime do próprio usuário
    }
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(caminho)?;
    std::fs::set_permissions(caminho, std::fs::Permissions::from_mode(0o700))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn saude_falha_sem_daemon() {
        let livre = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        // a porta foi liberada ao soltar o ouvinte: ninguém escuta nela
        assert!(!checar_saude(livre));
    }

    #[test]
    fn saude_passa_com_daemon() {
        let ouvinte = TcpListener::bind("127.0.0.1:0").unwrap();
        let endereco = ouvinte.local_addr().unwrap();
        let comp = Arc::new(Compartilhado::novo(
            ConfigEfetiva::carregar(None, |_| None),
            false,
        ));
        comp.bater();
        let ctx = Arc::new(ingress::Contexto {
            comp,
            porta_publica: endereco.port(),
            debug: false,
            comandos: None,
            onde: crate::ambiente::Ambiente::ler(|_| None).unwrap().onde(),
            aprovando: Mutex::new(()),
        });
        thread::spawn(move || ingress::servir(ouvinte, ctx));
        assert!(checar_saude(endereco));
    }
}
