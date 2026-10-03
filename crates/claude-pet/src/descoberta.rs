//! Descoberta do compositor em tempo de execução (decisão 0007, T1.1).
//!
//! O container monta o `/run/user` do host inteiro (somente leitura, rslave)
//! porque o socket do Wayland e a assinatura do Hyprland mudam a cada login.
//! Nada é fixado no compose: a cada 2 s, enquanto espera, o daemon:
//!
//! 1. lista `base/hypr/*`, da assinatura mais nova para a mais velha pelo
//!    epoch de `<hash>_<epoch>_<rand>`;
//! 2. lê o `hyprland.lock` (linha 2 = nome do socket Wayland);
//! 3. só aceita a instância se `connect()` funcionar no `.socket2.sock` **e**
//!    no `wayland-N`. Pastas velhas de um Hyprland que caiu recusam a conexão
//!    e são puladas. O `.socket2.sock` é aberto e fechado na hora: é o único
//!    toque no IPC do Hyprland no M1, e o `.socket.sock` (que executa
//!    comandos no host) nunca é aberto (decisão 0006).
//!
//! Caminhos com 108 bytes ou mais não cabem no `sun_path` do AF_UNIX; esses
//! passam por um symlink curto no `XDG_RUNTIME_DIR` privado do container.
//!
//! O backoff ([`Reconexao`]) vale só para nova tentativa na **mesma**
//! assinatura depois de uma falha do lado do cliente; uma assinatura nova
//! conecta na hora.

use std::fs;
use std::io;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Limite do `sun_path` do AF_UNIX, contando o NUL final.
const LIMITE_SUN_PATH: usize = 108;

/// Primeiro atraso depois de uma falha na mesma assinatura.
pub const ATRASO_MIN: Duration = Duration::from_secs(1);
/// Teto do backoff.
pub const ATRASO_MAX: Duration = Duration::from_secs(30);
/// Uma sessão que durou isto antes de cair zera o backoff.
pub const VIDA_PARA_ZERAR: Duration = Duration::from_secs(60);

/// Uma instância viva do Hyprland, já provada por `connect()`.
#[derive(Debug)]
pub struct Instancia {
    /// `<hash>_<epoch>_<rand>`, o nome da pasta em `hypr/`.
    pub assinatura: String,
    /// Nome do socket Wayland lido do `hyprland.lock` (`wayland-1`).
    pub nome_wayland: String,
    /// Conexão já aberta no socket Wayland (a da prova vira a de verdade).
    pub wayland: UnixStream,
}

/// Por que ainda não há compositor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Espera {
    /// `base` não existe: antes do login (sem linger) ou bind errado.
    SemRuntime,
    /// Não há nenhuma pasta de instância em `base/hypr`.
    SemHyprland,
    /// Há pastas, mas nenhuma viva (Hyprland caiu, ou ainda subindo).
    NenhumaViva { candidatas: usize },
}

impl Espera {
    pub fn descrever(&self) -> String {
        match self {
            Espera::SemRuntime => "runtime do usuário ainda não existe".into(),
            Espera::SemHyprland => "nenhuma instância do Hyprland".into(),
            Espera::NenhumaViva { candidatas } => {
                format!("{candidatas} instância(s) do Hyprland, nenhuma respondendo")
            }
        }
    }
}

/// Epoch de uma assinatura `<hash>_<epoch>_<rand>`; `None` se o formato for
/// outro (essas ficam por último).
pub fn epoch_da_assinatura(assinatura: &str) -> Option<u64> {
    let mut partes = assinatura.split('_');
    let (Some(hash), Some(epoch), Some(_), None) =
        (partes.next(), partes.next(), partes.next(), partes.next())
    else {
        return None;
    };
    if hash.is_empty() {
        return None;
    }
    epoch.parse().ok()
}

/// Nome do socket Wayland: a segunda linha do `hyprland.lock`.
pub fn nome_wayland_do_lock(conteudo: &str) -> Option<String> {
    let nome = conteudo.lines().nth(1)?.trim();
    let valido = !nome.is_empty()
        && nome.len() <= 64
        && nome
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        && nome != "."
        && nome != "..";
    valido.then(|| nome.to_owned())
}

/// Assinaturas em `base/hypr`, da mais nova para a mais velha.
fn candidatas(pasta_hypr: &Path) -> io::Result<Vec<String>> {
    let mut lista: Vec<(Option<u64>, String)> = Vec::new();
    for entrada in fs::read_dir(pasta_hypr)? {
        let entrada = entrada?;
        if !entrada.file_type()?.is_dir() {
            continue;
        }
        let Ok(nome) = entrada.file_name().into_string() else {
            continue;
        };
        lista.push((epoch_da_assinatura(&nome), nome));
    }
    // Mais nova primeiro; sem epoch vai para o fim; empate decide pelo nome
    // para a ordem não depender do sistema de arquivos.
    lista.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    Ok(lista.into_iter().map(|(_, nome)| nome).collect())
}

/// Procura a instância viva mais nova do Hyprland em `base` (o runtime do
/// usuário, `/host/run/user/<uid>` no container). `curto` é onde criar
/// symlinks para caminhos longos demais para o AF_UNIX.
pub fn procurar(base: &Path, curto: &Path) -> Result<Instancia, Espera> {
    if !base.is_dir() {
        return Err(Espera::SemRuntime);
    }
    let pasta_hypr = base.join("hypr");
    let nomes = match candidatas(&pasta_hypr) {
        Ok(nomes) if !nomes.is_empty() => nomes,
        _ => return Err(Espera::SemHyprland),
    };
    let total = nomes.len();
    for assinatura in nomes {
        let pasta = pasta_hypr.join(&assinatura);
        let Ok(lock) = fs::read_to_string(pasta.join("hyprland.lock")) else {
            depurar!("descoberta: {assinatura} sem hyprland.lock legível");
            continue;
        };
        let Some(nome_wayland) = nome_wayland_do_lock(&lock) else {
            depurar!("descoberta: {assinatura} com hyprland.lock sem nome de socket");
            continue;
        };
        let eventos = pasta.join(".socket2.sock");
        match conectar_unix(&eventos, curto, &format!("hypr-{assinatura}")) {
            // Só a prova: fecha na hora. Este é o único toque no IPC do
            // Hyprland no M1.
            Ok(fluxo) => drop(fluxo),
            Err(e) => {
                depurar!("descoberta: {assinatura} sem .socket2.sock vivo ({e})");
                continue;
            }
        }
        match conectar_unix(&base.join(&nome_wayland), curto, &nome_wayland) {
            Ok(wayland) => {
                return Ok(Instancia {
                    assinatura,
                    nome_wayland,
                    wayland,
                });
            }
            Err(e) => {
                depurar!("descoberta: {assinatura} sem {nome_wayland} vivo ({e})");
            }
        }
    }
    Err(Espera::NenhumaViva { candidatas: total })
}

/// `connect()` num socket Unix; se o caminho for longo demais para o
/// `sun_path`, passa por um symlink `curto/<apelido>`.
fn conectar_unix(caminho: &Path, curto: &Path, apelido: &str) -> io::Result<UnixStream> {
    if caminho.as_os_str().len() < LIMITE_SUN_PATH {
        return UnixStream::connect(caminho);
    }
    let link = curto.join(apelido);
    if link.as_os_str().len() >= LIMITE_SUN_PATH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "caminho do socket longo demais, mesmo pelo XDG_RUNTIME_DIR",
        ));
    }
    if fs::read_link(&link).ok().as_deref() != Some(caminho) {
        match fs::remove_file(&link) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        std::os::unix::fs::symlink(caminho, &link)?;
    }
    UnixStream::connect(&link)
}

/// Pasta do runtime do usuário dentro de `runtime_host` (`<runtime>/<uid>`).
pub fn base_do_usuario(runtime_host: &Path) -> PathBuf {
    runtime_host.join(rustix::process::getuid().as_raw().to_string())
}

/// Backoff de reconexão por assinatura.
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

    /// Pasta temporária apagada no fim do teste.
    struct Temp(PathBuf);

    impl Temp {
        fn nova(nome: &str) -> Temp {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static CONTADOR: AtomicUsize = AtomicUsize::new(0);
            let caminho = std::env::temp_dir().join(format!(
                "claude-pet-{}-{}-{nome}",
                std::process::id(),
                CONTADOR.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&caminho);
            fs::create_dir_all(&caminho).unwrap();
            Temp(caminho)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Cria `base/hypr/<assinatura>` com lock e, se `viva`, os sockets.
    fn instancia(base: &Path, assinatura: &str, wayland: &str, viva: bool) -> Vec<UnixListener> {
        let pasta = base.join("hypr").join(assinatura);
        fs::create_dir_all(&pasta).unwrap();
        fs::write(pasta.join("hyprland.lock"), format!("1501\n{wayland}\n")).unwrap();
        let mut vivos = Vec::new();
        let eventos = pasta.join(".socket2.sock");
        let socket_wayland = base.join(wayland);
        if viva {
            vivos.push(UnixListener::bind(&eventos).unwrap());
            vivos.push(UnixListener::bind(&socket_wayland).unwrap());
        } else {
            // Socket que sobrou de um Hyprland que caiu: existe e recusa.
            drop(UnixListener::bind(&eventos).unwrap());
            if !socket_wayland.exists() {
                drop(UnixListener::bind(&socket_wayland).unwrap());
            }
        }
        vivos
    }

    #[test]
    fn epoch_e_lock() {
        assert_eq!(
            epoch_da_assinatura("efb50993780079460b0cbed1363e2166a2de1d9f_1790020208_1687561921"),
            Some(1_790_020_208)
        );
        assert_eq!(epoch_da_assinatura("sem_epoch"), None);
        assert_eq!(epoch_da_assinatura("a_x_1"), None);
        assert_eq!(epoch_da_assinatura("_1_2"), None);
        assert_eq!(
            nome_wayland_do_lock("1501\nwayland-1\n").as_deref(),
            Some("wayland-1")
        );
        assert_eq!(nome_wayland_do_lock("1501\n"), None);
        assert_eq!(nome_wayland_do_lock("1501\n../../etc\n"), None);
        assert_eq!(nome_wayland_do_lock("1501\n..\n"), None);
    }

    #[test]
    fn sem_runtime_e_sem_hyprland() {
        let t = Temp::nova("vazio");
        assert_eq!(
            procurar(&t.0.join("nada"), &t.0).unwrap_err(),
            Espera::SemRuntime
        );
        assert_eq!(procurar(&t.0, &t.0).unwrap_err(), Espera::SemHyprland);
        fs::create_dir_all(t.0.join("hypr")).unwrap();
        assert_eq!(procurar(&t.0, &t.0).unwrap_err(), Espera::SemHyprland);
    }

    #[test]
    fn a_mais_nova_viva_vence() {
        let t = Temp::nova("nova");
        let _velha = instancia(&t.0, "aaa_100_1", "wayland-0", true);
        let _nova = instancia(&t.0, "bbb_200_2", "wayland-1", true);
        let achada = procurar(&t.0, &t.0).unwrap();
        assert_eq!(achada.assinatura, "bbb_200_2");
        assert_eq!(achada.nome_wayland, "wayland-1");
    }

    #[test]
    fn pasta_velha_que_recusa_e_pulada() {
        let t = Temp::nova("velha");
        let _viva = instancia(&t.0, "aaa_100_1", "wayland-1", true);
        let _morta = instancia(&t.0, "ccc_300_3", "wayland-2", false);
        let achada = procurar(&t.0, &t.0).unwrap();
        assert_eq!(achada.assinatura, "aaa_100_1");
    }

    #[test]
    fn sem_lock_e_pulada_e_so_mortas_dao_nenhuma_viva() {
        let t = Temp::nova("semlock");
        fs::create_dir_all(t.0.join("hypr/ddd_400_4")).unwrap();
        let _morta = instancia(&t.0, "aaa_100_1", "wayland-1", false);
        assert_eq!(
            procurar(&t.0, &t.0).unwrap_err(),
            Espera::NenhumaViva { candidatas: 2 }
        );
        let _viva = instancia(&t.0, "bbb_200_2", "wayland-3", true);
        assert_eq!(procurar(&t.0, &t.0).unwrap().assinatura, "bbb_200_2");
    }

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
        assert_eq!(fs::read_link(curto.join("apelido")).unwrap(), socket);
        // Segunda vez reaproveita o link.
        assert!(conectar_unix(&socket, &curto, "apelido").is_ok());
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
