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
//! passam por um symlink curto numa pasta só nossa (`claude-pet/`, 0700)
//! dentro do `XDG_RUNTIME_DIR` privado do container. Nada fora dela é
//! apagado, e dentro dela só symlinks: rodando fora do container, o
//! `XDG_RUNTIME_DIR` é o do usuário, onde mora o socket do compositor.
//!
//! O backoff ([`Reconexao`]) vale só para nova tentativa na **mesma**
//! assinatura depois de uma falha do lado do cliente; uma assinatura nova
//! conecta na hora. Enquanto a assinatura espera, nenhuma conexão é aberta
//! nela (nem a prova).

use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Limite do `sun_path` do AF_UNIX, contando o NUL final.
const LIMITE_SUN_PATH: usize = 108;
/// Pasta dos symlinks curtos, dentro de `curto`.
const PASTA_LINKS: &str = "claude-pet";

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
    /// A instância mais nova ainda candidata falhou do nosso lado há pouco:
    /// espera o backoff sem abrir conexão nela.
    Recuo { restante: Duration },
}

impl Espera {
    /// Motivo para o log. Sem contagem regressiva: o laço só registra quando
    /// o motivo muda.
    pub fn descrever(&self) -> String {
        match self {
            Espera::SemRuntime => "runtime do usuário ainda não existe".into(),
            Espera::SemHyprland => "nenhuma instância do Hyprland".into(),
            Espera::NenhumaViva { candidatas } => {
                format!("{candidatas} instância(s) do Hyprland, nenhuma respondendo")
            }
            Espera::Recuo { .. } => {
                "a mesma instância falhou há pouco; nova tentativa depois do backoff".into()
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
///
/// `em_espera(assinatura)` diz se a assinatura está no backoff e quanto
/// falta. Ao chegar nela (as mais novas já foram tentadas), a procura para
/// com [`Espera::Recuo`] sem conectar em nada: nem nela, nem nas mais velhas.
pub fn procurar(
    base: &Path,
    curto: &Path,
    em_espera: impl Fn(&str) -> Option<Duration>,
) -> Result<Instancia, Espera> {
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
        if let Some(restante) = em_espera(&assinatura) {
            return Err(Espera::Recuo { restante });
        }
        let eventos = pasta.join(".socket2.sock");
        match conectar_unix(&eventos, curto, &format!("eventos-{assinatura}")) {
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
/// `sun_path`, passa por um symlink `curto/claude-pet/pet-<apelido>`.
fn conectar_unix(caminho: &Path, curto: &Path, apelido: &str) -> io::Result<UnixStream> {
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

    fn sem_backoff(_: &str) -> Option<Duration> {
        None
    }

    /// Ouvintes de uma instância de mentira (vivos enquanto ela existir),
    /// não bloqueantes para dar para conferir se alguém conectou.
    struct Falsa {
        eventos: UnixListener,
        wayland: UnixListener,
        /// O socket de comandos do Hyprland: nunca pode receber conexão.
        comandos: UnixListener,
    }

    /// Ninguém conectou neste ouvinte.
    fn intocado(ouvinte: &UnixListener) -> bool {
        matches!(ouvinte.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    }

    fn ouvir(caminho: &Path) -> UnixListener {
        let ouvinte = UnixListener::bind(caminho).unwrap();
        ouvinte.set_nonblocking(true).unwrap();
        ouvinte
    }

    fn pasta_com_lock(base: &Path, assinatura: &str, wayland: &str) -> PathBuf {
        let pasta = base.join("hypr").join(assinatura);
        fs::create_dir_all(&pasta).unwrap();
        fs::write(pasta.join("hyprland.lock"), format!("1501\n{wayland}\n")).unwrap();
        pasta
    }

    /// `base/hypr/<assinatura>` com lock e os três sockets ouvindo.
    fn viva(base: &Path, assinatura: &str, wayland: &str) -> Falsa {
        let pasta = pasta_com_lock(base, assinatura, wayland);
        Falsa {
            eventos: ouvir(&pasta.join(".socket2.sock")),
            wayland: ouvir(&base.join(wayland)),
            comandos: ouvir(&pasta.join(".socket.sock")),
        }
    }

    /// `base/hypr/<assinatura>` com lock e sockets que sobraram de um
    /// Hyprland que caiu: existem e recusam.
    fn morta(base: &Path, assinatura: &str, wayland: &str) {
        let pasta = pasta_com_lock(base, assinatura, wayland);
        drop(UnixListener::bind(pasta.join(".socket2.sock")).unwrap());
        let socket_wayland = base.join(wayland);
        if !socket_wayland.exists() {
            drop(UnixListener::bind(&socket_wayland).unwrap());
        }
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
            procurar(&t.0.join("nada"), &t.0, sem_backoff).unwrap_err(),
            Espera::SemRuntime
        );
        assert_eq!(
            procurar(&t.0, &t.0, sem_backoff).unwrap_err(),
            Espera::SemHyprland
        );
        fs::create_dir_all(t.0.join("hypr")).unwrap();
        assert_eq!(
            procurar(&t.0, &t.0, sem_backoff).unwrap_err(),
            Espera::SemHyprland
        );
    }

    #[test]
    fn a_mais_nova_viva_vence() {
        let t = Temp::nova("nova");
        let _velha = viva(&t.0, "aaa_100_1", "wayland-0");
        let _nova = viva(&t.0, "bbb_200_2", "wayland-1");
        let achada = procurar(&t.0, &t.0, sem_backoff).unwrap();
        assert_eq!(achada.assinatura, "bbb_200_2");
        assert_eq!(achada.nome_wayland, "wayland-1");
    }

    #[test]
    fn pasta_velha_que_recusa_e_pulada() {
        let t = Temp::nova("velha");
        let _viva = viva(&t.0, "aaa_100_1", "wayland-1");
        morta(&t.0, "ccc_300_3", "wayland-2");
        let achada = procurar(&t.0, &t.0, sem_backoff).unwrap();
        assert_eq!(achada.assinatura, "aaa_100_1");
    }

    #[test]
    fn sem_lock_e_pulada_e_so_mortas_dao_nenhuma_viva() {
        let t = Temp::nova("semlock");
        fs::create_dir_all(t.0.join("hypr/ddd_400_4")).unwrap();
        morta(&t.0, "aaa_100_1", "wayland-1");
        assert_eq!(
            procurar(&t.0, &t.0, sem_backoff).unwrap_err(),
            Espera::NenhumaViva { candidatas: 2 }
        );
        let _viva = viva(&t.0, "bbb_200_2", "wayland-3");
        assert_eq!(
            procurar(&t.0, &t.0, sem_backoff).unwrap().assinatura,
            "bbb_200_2"
        );
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
    fn socket_de_comandos_nunca_e_aberto() {
        let t = Temp::nova("comandos");
        let falsa = viva(&t.0, "aaa_100_1", "wayland-1");
        let achada = procurar(&t.0, &t.0, sem_backoff).unwrap();
        assert_eq!(achada.assinatura, "aaa_100_1");
        assert!(
            !intocado(&falsa.eventos),
            "a prova conecta no .socket2.sock"
        );
        assert!(!intocado(&falsa.wayland));
        assert!(intocado(&falsa.comandos), "o .socket.sock nunca é aberto");
    }

    #[test]
    fn backoff_nao_abre_conexao_nenhuma() {
        let t = Temp::nova("recuo");
        let velha = viva(&t.0, "aaa_100_1", "wayland-0");
        let nova = viva(&t.0, "bbb_200_2", "wayland-1");
        let espera = |a: &str| (a == "bbb_200_2").then_some(Duration::from_secs(7));
        assert_eq!(
            procurar(&t.0, &t.0, espera).unwrap_err(),
            Espera::Recuo {
                restante: Duration::from_secs(7)
            }
        );
        for falsa in [&velha, &nova] {
            assert!(intocado(&falsa.eventos) && intocado(&falsa.wayland));
            assert!(intocado(&falsa.comandos));
        }
    }

    #[test]
    fn instancia_mais_nova_que_a_do_backoff_conecta_na_hora() {
        let t = Temp::nova("mais-nova");
        let velha = viva(&t.0, "aaa_100_1", "wayland-0");
        let _nova = viva(&t.0, "bbb_200_2", "wayland-1");
        let espera = |a: &str| (a == "aaa_100_1").then_some(Duration::from_secs(30));
        assert_eq!(
            procurar(&t.0, &t.0, espera).unwrap().assinatura,
            "bbb_200_2"
        );
        assert!(intocado(&velha.eventos) && intocado(&velha.wayland));
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
