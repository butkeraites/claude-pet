//! O que a descoberta usa em qualquer compositor (decisão 0007).
//!
//! - **Caminho longo:** caminhos com 108 bytes ou mais não cabem no
//!   `sun_path` do AF_UNIX; esses passam por um symlink curto numa pasta só
//!   nossa (`claude-pet/`, 0700) dentro do `XDG_RUNTIME_DIR` privado do
//!   container. Nada fora dela é apagado, e dentro dela só symlinks: rodando
//!   fora do container, o `XDG_RUNTIME_DIR` é o do usuário, onde mora o
//!   socket do compositor.
//! - **Backoff** ([`Reconexao`]): vale só para nova tentativa na **mesma**
//!   instância depois de uma falha do lado do cliente; uma instância nova
//!   conecta na hora.

use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Limite do `sun_path` do AF_UNIX, contando o NUL final.
pub const LIMITE_SUN_PATH: usize = 108;
/// Pasta dos symlinks curtos, dentro de `curto`.
pub const PASTA_LINKS: &str = "claude-pet";

/// Primeiro atraso depois de uma falha na mesma instância.
pub const ATRASO_MIN: Duration = Duration::from_secs(1);
/// Teto do backoff.
pub const ATRASO_MAX: Duration = Duration::from_secs(30);
/// Uma sessão que durou isto antes de cair zera o backoff.
pub const VIDA_PARA_ZERAR: Duration = Duration::from_secs(60);

/// `connect()` num socket Unix; se o caminho for longo demais para o
/// `sun_path`, passa por um symlink `curto/claude-pet/pet-<apelido>`.
pub fn conectar_unix(caminho: &Path, curto: &Path, apelido: &str) -> io::Result<UnixStream> {
    if caminho.as_os_str().len() < LIMITE_SUN_PATH {
        return UnixStream::connect(caminho);
    }
    let pasta = curto.join(PASTA_LINKS);
    let link = pasta.join(format!("pet-{apelido}"));
    if link.as_os_str().len() >= LIMITE_SUN_PATH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "caminho do socket longo demais, mesmo pelo XDG_RUNTIME_DIR",
        ));
    }
    preparar_pasta_de_links(&pasta)?;
    match fs::symlink_metadata(&link) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if fs::read_link(&link)? != caminho {
                fs::remove_file(&link)?;
                std::os::unix::fs::symlink(caminho, &link)?;
            }
        }
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} existe e não é um symlink nosso", link.display()),
            ));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            std::os::unix::fs::symlink(caminho, &link)?;
        }
        Err(e) => return Err(e),
    }
    UnixStream::connect(&link)
}

/// Cria a pasta dos symlinks (0700) se faltar; recusa se ela existir e não
/// for uma pasta de verdade (um symlink plantado, por exemplo).
fn preparar_pasta_de_links(pasta: &Path) -> io::Result<()> {
    match fs::symlink_metadata(pasta) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} existe e não é uma pasta", pasta.display()),
        )),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::DirBuilder::new().mode(0o700).create(pasta)?;
            fs::set_permissions(pasta, fs::Permissions::from_mode(0o700))
        }
        Err(e) => Err(e),
    }
}

/// Pasta do runtime do usuário dentro de `runtime_host` (`<runtime>/<uid>`).
pub fn base_do_usuario(runtime_host: &Path) -> PathBuf {
    runtime_host.join(rustix::process::getuid().as_raw().to_string())
}

/// Backoff de reconexão por instância (a assinatura, no Hyprland).
#[derive(Debug, Default)]
pub struct Reconexao {
    falha: Option<Falha>,
}

#[derive(Debug)]
struct Falha {
    assinatura: String,
    atraso: Duration,
    liberada_em: Instant,
}

impl Reconexao {
    /// Pode tentar `assinatura` agora? Uma assinatura diferente da que
    /// falhou sempre pode (instância nova conecta na hora).
    pub fn pode_tentar(&self, assinatura: &str, agora: Instant) -> bool {
        match &self.falha {
            Some(f) if f.assinatura == assinatura => agora >= f.liberada_em,
            _ => true,
        }
    }

    /// Quanto falta para poder tentar `assinatura` de novo.
    pub fn espera_restante(&self, assinatura: &str, agora: Instant) -> Duration {
        match &self.falha {
            Some(f) if f.assinatura == assinatura => f.liberada_em.saturating_duration_since(agora),
            _ => Duration::ZERO,
        }
    }

    /// Registra que a sessão em `assinatura` falhou depois de viver `viveu`.
    /// Devolve o atraso até a próxima tentativa nessa assinatura.
    pub fn falhou(&mut self, assinatura: &str, viveu: Duration, agora: Instant) -> Duration {
        let atraso = match &self.falha {
            Some(f) if f.assinatura == assinatura && viveu < VIDA_PARA_ZERAR => {
                (f.atraso * 2).min(ATRASO_MAX)
            }
            _ => ATRASO_MIN,
        };
        self.falha = Some(Falha {
            assinatura: assinatura.to_owned(),
            atraso,
            liberada_em: agora + atraso,
        });
        atraso
    }
}

#[cfg(test)]
mod testes {
    use std::os::unix::net::UnixListener;

    use super::*;
    use crate::apoio::Temp;

    #[test]
    fn caminho_longo_passa_por_symlink_curto() {
        let t = Temp::nova("longo");
        let curto = t.0.join("xdg");
        fs::create_dir_all(&curto).unwrap();
        let fundo = t.0.join("x".repeat(60)).join("y".repeat(40));
        fs::create_dir_all(&fundo).unwrap();
        let socket = fundo.join("s.sock");
        assert!(socket.as_os_str().len() >= LIMITE_SUN_PATH);
        // O bind também precisa do caminho curto.
        let link_dir = curto.join("dir");
        std::os::unix::fs::symlink(&fundo, &link_dir).unwrap();
        let _ouvinte = UnixListener::bind(link_dir.join("s.sock")).unwrap();
        let fluxo = conectar_unix(&socket, &curto, "apelido");
        assert!(fluxo.is_ok(), "{fluxo:?}");
        let pasta = curto.join(PASTA_LINKS);
        assert_eq!(fs::read_link(pasta.join("pet-apelido")).unwrap(), socket);
        let modo = fs::metadata(&pasta).unwrap().permissions().mode() & 0o777;
        assert_eq!(modo, 0o700, "pasta dos links privada");
        // Segunda vez reaproveita o link.
        assert!(conectar_unix(&socket, &curto, "apelido").is_ok());
    }

    #[test]
    fn link_curto_nunca_apaga_o_que_nao_e_symlink_nosso() {
        let t = Temp::nova("seguro");
        let curto = t.0.join("xdg");
        let pasta = curto.join(PASTA_LINKS);
        fs::create_dir_all(&pasta).unwrap();
        let socket = t.0.join("z".repeat(110)).join("s.sock");
        // Um arquivo de verdade (fora do container seria, por exemplo, o
        // socket do compositor) no lugar do apelido: fica intacto.
        let apelido = pasta.join("pet-wayland-1");
        fs::write(&apelido, "não é nosso").unwrap();
        assert!(conectar_unix(&socket, &curto, "wayland-1").is_err());
        assert_eq!(fs::read_to_string(&apelido).unwrap(), "não é nosso");
        // Um symlink velho (de uma instância anterior) é trocado.
        let velho = pasta.join("pet-velho");
        std::os::unix::fs::symlink(t.0.join("outro"), &velho).unwrap();
        let _ = conectar_unix(&socket, &curto, "velho");
        assert_eq!(fs::read_link(&velho).unwrap(), socket);
        // A pasta dos links trocada por um symlink: recusa.
        let curto2 = t.0.join("xdg2");
        fs::create_dir_all(&curto2).unwrap();
        std::os::unix::fs::symlink(&t.0, curto2.join(PASTA_LINKS)).unwrap();
        assert!(conectar_unix(&socket, &curto2, "x").is_err());
    }

    #[test]
    fn backoff_so_na_mesma_assinatura() {
        let t0 = Instant::now();
        let mut r = Reconexao::default();
        assert!(r.pode_tentar("a_1_1", t0));
        assert_eq!(
            r.falhou("a_1_1", Duration::ZERO, t0),
            Duration::from_secs(1)
        );
        assert!(!r.pode_tentar("a_1_1", t0));
        assert!(
            r.pode_tentar("b_2_2", t0),
            "assinatura nova conecta na hora"
        );
        assert!(r.pode_tentar("a_1_1", t0 + Duration::from_secs(1)));
        let mut atrasos = Vec::new();
        for _ in 0..7 {
            atrasos.push(r.falhou("a_1_1", Duration::from_secs(2), t0).as_secs());
        }
        assert_eq!(atrasos, vec![2, 4, 8, 16, 30, 30, 30]);
        assert_eq!(
            r.espera_restante("a_1_1", t0 + Duration::from_secs(10)),
            Duration::from_secs(20)
        );
        // Viveu 60 s: zera.
        assert_eq!(r.falhou("a_1_1", VIDA_PARA_ZERAR, t0), ATRASO_MIN);
        // Falha numa assinatura diferente também recomeça do mínimo.
        assert_eq!(r.falhou("b_2_2", Duration::ZERO, t0), ATRASO_MIN);
        assert!(r.pode_tentar("a_1_1", t0));
    }
}
