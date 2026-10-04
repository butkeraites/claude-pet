//! O leitor do socket de eventos do Hyprland (`.socket2.sock`; decisões 0006
//! e 0050).
//!
//! Uma thread só para ler: o Hyprland desconecta um cliente com 64 eventos
//! acumulados, então ela drena sempre, sem esperar ninguém, e entrega ao laço
//! principal pela [`Caixa`] (que acorda o laço). Cai o socket, ela tenta de
//! novo com backoff (1 s dobrando até 30 s; uma ligação que durou 60 s zera),
//! até ser parada.
//!
//! **Privacidade.** Cada linha é traduzida na hora e esquecida. Do que o
//! Hyprland manda, só sai daqui:
//! - o nome do monitor em foco (`focusedmonv2`; o FALLBACK nunca);
//! - o endereço da janela ativa (`activewindowv2`), carimbado com a hora de
//!   parede em que a linha foi lida;
//! - se o título da janela em foco começa com um glifo do Claude Code (✳, ◐
//!   ou ◑; `activewindow`): só o booleano, nunca o título nem a classe;
//! - o endereço de uma janela que abriu (`openwindow`), com um booleano dizendo
//!   se é a proteção de tela do Omarchy, e o de uma que fechou
//!   (`closewindow`);
//! - que um monitor entrou ou saiu.
//!
//! O resto (títulos, classes, áreas de trabalho, `windowtitle`, …) nem é
//! interpretado. Nada da linha vai para o log: só ligou, caiu e quanto falta
//! para tentar de novo.

use std::io::{self, BufRead, BufReader, Read};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pet_core::plataforma::{Alca, Caixa, EventoDesktop};

use super::eh_reserva;
use crate::conexao::conectar_unix;

/// A classe da janela da proteção de tela do Omarchy.
pub const PROTETOR_DE_TELA: &str = "org.omarchy.screensaver";
/// Os glifos com que o Claude Code começa o título do terminal: parado (✳) e
/// trabalhando (◐, ◑).
pub const GLIFOS_DO_CLAUDE: [char; 3] = ['\u{2733}', '\u{25D0}', '\u{25D1}'];
/// O Hyprland corta o DATA em 1024 bytes; uma linha maior que isto é lixo e
/// é pulada sem ser guardada.
const LIMITE_LINHA: u64 = 4096;
/// Primeiro atraso depois de cair.
pub const ATRASO_MIN: Duration = Duration::from_secs(1);
/// Teto do backoff.
pub const ATRASO_MAX: Duration = Duration::from_secs(30);
/// Uma ligação que durou isto zera o backoff.
const VIDA_PARA_ZERAR: Duration = Duration::from_secs(60);

/// O que uma linha do socket2 vira (sem a hora; quem carimba é o leitor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Linha {
    MonitorEmFoco(String),
    /// `None`: nenhuma janela em foco.
    Ativa(Option<Alca>),
    OlhandoClaude(bool),
    Abriu {
        janela: Alca,
        protetor: bool,
    },
    Fechou(Alca),
    Monitores,
}

/// Endereço de janela do Hyprland nos eventos: hexadecimal sem `0x`.
fn endereco(texto: &str) -> Option<Alca> {
    let valido = !texto.is_empty()
        && texto.len() <= 16
        && texto
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    valido.then(|| Alca(texto.to_owned()))
}

/// Nome de monitor (conector): `eDP-1`, `HDMI-A-1`, `HEADLESS-2`.
fn nome_de_monitor(texto: &str) -> Option<String> {
    let valido = !texto.is_empty()
        && texto.len() <= 64
        && texto
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
    valido.then(|| texto.to_owned())
}

/// Traduz uma linha (`EVENTO>>DATA`, sem o `\n`). `None`: um evento que o pet
/// não usa, ou um valor que não passa nas regras.
pub fn traduzir(linha: &[u8]) -> Option<Linha> {
    let fim_do_nome = linha.windows(2).position(|j| j == b">>")?;
    let nome = &linha[..fim_do_nome];
    let dados = &linha[fim_do_nome + 2..];
    match nome {
        b"focusedmonv2" => {
            // MONNAME,WORKSPACEID: o id da área de trabalho fica aqui.
            let dados = std::str::from_utf8(dados).ok()?;
            let (monitor, _) = dados.rsplit_once(',')?;
            let monitor = nome_de_monitor(monitor)?;
            (!eh_reserva(&monitor)).then_some(Linha::MonitorEmFoco(monitor))
        }
        b"activewindowv2" => {
            let dados = std::str::from_utf8(dados).ok()?;
            if dados.is_empty() || dados == "," {
                return Some(Linha::Ativa(None));
            }
            endereco(dados).map(|a| Linha::Ativa(Some(a)))
        }
        b"activewindow" => {
            // CLASS,TITLE: só o primeiro caractere do título é olhado, e só
            // o booleano sai. Um título com bytes que não são UTF-8 conta
            // como "não é o Claude".
            let virgula = dados.iter().position(|&b| b == b',')?;
            let titulo = &dados[virgula + 1..];
            // Um caractere tem até 4 bytes; um partido ou inválido vira o de
            // substituição, que não é glifo do Claude.
            let inicio = &titulo[..titulo.len().min(4)];
            let primeiro = String::from_utf8_lossy(inicio).chars().next();
            Some(Linha::OlhandoClaude(
                primeiro.is_some_and(|c| GLIFOS_DO_CLAUDE.contains(&c)),
            ))
        }
        b"openwindow" => {
            // ADDR,WORKSPACENAME,CLASS,TITLE: o endereço e a classe só para
            // o booleano da proteção de tela.
            let mut partes = dados.splitn(4, |&b| b == b',');
            let janela = endereco(std::str::from_utf8(partes.next()?).ok()?)?;
            let _area = partes.next()?;
            let classe = partes.next()?;
            Some(Linha::Abriu {
                janela,
                protetor: classe == PROTETOR_DE_TELA.as_bytes(),
            })
        }
        b"closewindow" => {
            let dados = std::str::from_utf8(dados).ok()?;
            endereco(dados).map(Linha::Fechou)
        }
        b"monitoraddedv2" | b"monitorremovedv2" => Some(Linha::Monitores),
        _ => None,
    }
}

/// Hora de parede em ms desde 1970 (o relógio do `ts` dos hooks).
fn agora_parede_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Lembra o último valor de cada coisa que se repete: o Hyprland manda o
/// `activewindow` de novo a cada troca de título da janela em foco (o
/// spinner do Claude Code troca uma vez por segundo), e só a mudança
/// interessa ao laço.
#[derive(Debug, Default)]
struct Repetidos {
    ativa: Option<Option<Alca>>,
    olhando: Option<bool>,
}

impl Repetidos {
    /// O evento para o laço, ou `None` se não mudou nada.
    fn filtrar(&mut self, linha: Linha, parede_ms: u64) -> Option<EventoDesktop> {
        match linha {
            Linha::Ativa(janela) => {
                if self.ativa.as_ref() == Some(&janela) {
                    return None;
                }
                self.ativa = Some(janela.clone());
                Some(EventoDesktop::JanelaAtiva { janela, parede_ms })
            }
            Linha::OlhandoClaude(olhando) => {
                if self.olhando == Some(olhando) {
                    return None;
                }
                self.olhando = Some(olhando);
                Some(EventoDesktop::OlhandoClaude(olhando))
            }
            Linha::MonitorEmFoco(nome) => Some(EventoDesktop::MonitorEmFoco(nome)),
            Linha::Abriu { janela, protetor } => {
                Some(EventoDesktop::JanelaAbriu { janela, protetor })
            }
            Linha::Fechou(janela) => {
                // A janela ativa fechou: a próxima ativação passa, seja ela
                // qual for (até um endereço reaproveitado).
                if self.ativa.as_ref().and_then(Option::as_ref) == Some(&janela) {
                    self.ativa = None;
                }
                Some(EventoDesktop::JanelaFechou(janela))
            }
            Linha::Monitores => Some(EventoDesktop::Monitores),
        }
    }
}

/// O que o laço e o leitor dividem.
struct Comum {
    parar: AtomicBool,
    /// Uma cópia da conexão de agora, para o `parar` desbloquear a leitura.
    fluxo: Mutex<Option<UnixStream>>,
    /// Acorda a espera do backoff.
    sono: (Mutex<()>, Condvar),
    /// Eventos que não couberam na caixa (o laço atrasado).
    perdidos: AtomicU64,
}

/// O leitor rodando numa thread; para quando sai de escopo.
pub struct Leitor {
    comum: Arc<Comum>,
    fio: Option<JoinHandle<()>>,
}

impl Leitor {
    /// Começa a ler `caminho` (o `.socket2.sock` da instância). `curto` e
    /// `apelido`: o symlink curto para caminhos longos demais
    /// ([`conectar_unix`]). Os eventos vão para `caixa`.
    pub fn iniciar(
        caminho: PathBuf,
        curto: PathBuf,
        apelido: String,
        caixa: Caixa<EventoDesktop>,
    ) -> io::Result<Leitor> {
        Leitor::iniciar_com(caminho, curto, apelido, caixa, ATRASO_MIN)
    }

    /// [`Leitor::iniciar`] com outro atraso inicial (os testes).
    pub fn iniciar_com(
        caminho: PathBuf,
        curto: PathBuf,
        apelido: String,
        caixa: Caixa<EventoDesktop>,
        atraso_min: Duration,
    ) -> io::Result<Leitor> {
        let comum = Arc::new(Comum {
            parar: AtomicBool::new(false),
            fluxo: Mutex::new(None),
            sono: (Mutex::new(()), Condvar::new()),
            perdidos: AtomicU64::new(0),
        });
        let dele = Arc::clone(&comum);
        let fio = std::thread::Builder::new()
            .name("hypr-eventos".into())
            .spawn(move || {
                ler_para_sempre(&caminho, &curto, &apelido, &caixa, &dele, atraso_min)
            })?;
        Ok(Leitor {
            comum,
            fio: Some(fio),
        })
    }

    /// Eventos que não couberam na caixa desde o começo.
    pub fn perdidos(&self) -> u64 {
        self.comum.perdidos.load(Ordering::Relaxed)
    }

    /// Para a thread e espera ela sair.
    pub fn parar(&mut self) {
        self.comum.parar.store(true, Ordering::Relaxed);
        if let Some(fluxo) = self
            .comum
            .fluxo
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = fluxo.shutdown(std::net::Shutdown::Both);
        }
        {
            let _trava = self.comum.sono.0.lock().unwrap_or_else(|e| e.into_inner());
            self.comum.sono.1.notify_all();
        }
        if let Some(fio) = self.fio.take() {
            let _ = fio.join();
        }
    }
}

impl Drop for Leitor {
    fn drop(&mut self) {
        self.parar();
    }
}

/// Espera `quanto`, ou até o `parar`. `true` se mandaram parar.
fn dormir(comum: &Comum, quanto: Duration) -> bool {
    let prazo = Instant::now() + quanto;
    let mut trava = comum.sono.0.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        if comum.parar.load(Ordering::Relaxed) {
            return true;
        }
        let falta = prazo.saturating_duration_since(Instant::now());
        if falta.is_zero() {
            return false;
        }
        trava = comum
            .sono
            .1
            .wait_timeout(trava, falta)
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
}

/// Manda sem esperar; `false` se a caixa estava cheia (o evento se perdeu e
/// é contado).
fn mandar(caixa: &Caixa<EventoDesktop>, comum: &Comum, evento: EventoDesktop) -> bool {
    let foi = caixa.tentar(evento).is_ok();
    if !foi {
        comum.perdidos.fetch_add(1, Ordering::Relaxed);
    }
    foi
}

/// A entrega de uma ligação: lembra as repetições e se algo se perdeu.
#[derive(Debug, Default)]
struct Entrega {
    repetidos: Repetidos,
    /// Um evento não coube na caixa (o laço atrasado) desde a última
    /// entrega.
    perdeu: bool,
}

impl Entrega {
    /// Manda o evento de uma linha. Depois de uma perda, o laço vê a fonte
    /// cair e voltar antes do próximo evento (decisão 0061): o anel ganha um
    /// buraco no lugar da troca que se perdeu, e a memória das repetições
    /// recomeça, para a próxima ativação passar mesmo que seja a mesma janela
    /// de antes.
    fn linha(&mut self, caixa: &Caixa<EventoDesktop>, comum: &Comum, linha: Linha, parede_ms: u64) {
        let Some(evento) = self.repetidos.filtrar(linha, parede_ms) else {
            return;
        };
        if self.perdeu {
            if !(mandar(caixa, comum, EventoDesktop::Ligado(false))
                && mandar(caixa, comum, EventoDesktop::Ligado(true)))
            {
                self.repetidos = Repetidos::default();
                return;
            }
            self.perdeu = false;
            aviso!(
                "eventos do Hyprland: {} evento(s) não couberam na caixa (o laço atrasado); o \
                 anel ganhou um buraco",
                comum.perdidos.load(Ordering::Relaxed)
            );
        }
        if !mandar(caixa, comum, evento) {
            self.perdeu = true;
            self.repetidos = Repetidos::default();
        }
    }
}

fn ler_para_sempre(
    caminho: &std::path::Path,
    curto: &std::path::Path,
    apelido: &str,
    caixa: &Caixa<EventoDesktop>,
    comum: &Comum,
    atraso_min: Duration,
) {
    let mut atraso = atraso_min;
    let mut avisou_falha = false;
    while !comum.parar.load(Ordering::Relaxed) {
        let fluxo = match conectar_unix(caminho, curto, apelido) {
            Ok(fluxo) => fluxo,
            Err(e) => {
                if !avisou_falha {
                    aviso!(
                        "eventos do Hyprland: não consegui ligar ({}); tentando de novo",
                        e.kind()
                    );
                    avisou_falha = true;
                }
                if dormir(comum, atraso) {
                    return;
                }
                atraso = (atraso * 2).min(ATRASO_MAX);
                continue;
            }
        };
        avisou_falha = false;
        if let Ok(copia) = fluxo.try_clone() {
            *comum.fluxo.lock().unwrap_or_else(|e| e.into_inner()) = Some(copia);
        }
        if comum.parar.load(Ordering::Relaxed) {
            return;
        }
        info!("eventos do Hyprland: ligado ao socket de eventos");
        let contou = mandar(caixa, comum, EventoDesktop::Ligado(true));
        let inicio = Instant::now();
        let motivo = ler_linhas(fluxo, caixa, comum, contou);
        *comum.fluxo.lock().unwrap_or_else(|e| e.into_inner()) = None;
        if comum.parar.load(Ordering::Relaxed) {
            return;
        }
        mandar(caixa, comum, EventoDesktop::Ligado(false));
        if inicio.elapsed() >= VIDA_PARA_ZERAR {
            atraso = atraso_min;
        }
        aviso!(
            "eventos do Hyprland: a ligação caiu ({motivo}); nova tentativa em {} ms",
            atraso.as_millis()
        );
        if dormir(comum, atraso) {
            return;
        }
        atraso = (atraso * 2).min(ATRASO_MAX);
    }
}

/// Lê até o fim da conexão. `contou`: o laço soube que a fonte ligou (o
/// `Ligado(true)` coube na caixa). Devolve o motivo (sem nada da linha).
fn ler_linhas(
    fluxo: UnixStream,
    caixa: &Caixa<EventoDesktop>,
    comum: &Comum,
    contou: bool,
) -> String {
    let mut leitor = BufReader::with_capacity(8192, fluxo);
    let mut linha = Vec::with_capacity(256);
    let mut entrega = Entrega {
        perdeu: !contou,
        ..Entrega::default()
    };
    loop {
        linha.clear();
        match (&mut leitor)
            .take(LIMITE_LINHA)
            .read_until(b'\n', &mut linha)
        {
            Ok(0) => return "fim da conexão".into(),
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return format!("erro de leitura: {}", e.kind()),
        }
        if linha.last() != Some(&b'\n') {
            if linha.len() as u64 >= LIMITE_LINHA {
                // Linha grande demais: pula até o fim dela sem guardar.
                if let Err(e) = pular_ate_o_fim(&mut leitor) {
                    return format!("erro de leitura: {}", e.kind());
                }
                continue;
            }
            return "fim da conexão".into();
        }
        linha.pop();
        let Some(traduzida) = traduzir(&linha) else {
            continue;
        };
        entrega.linha(caixa, comum, traduzida, agora_parede_ms());
    }
}

fn pular_ate_o_fim(leitor: &mut BufReader<UnixStream>) -> io::Result<()> {
    loop {
        let (consumir, achou) = {
            let pedaco = leitor.fill_buf()?;
            if pedaco.is_empty() {
                return Ok(());
            }
            match pedaco.iter().position(|&b| b == b'\n') {
                Some(i) => (i + 1, true),
                None => (pedaco.len(), false),
            }
        };
        leitor.consume(consumir);
        if achou {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod testes {
    use std::io::Write;
    use std::os::unix::net::UnixListener;
    use std::sync::mpsc;

    use pet_core::plataforma::SemDespertador;

    use super::*;
    use crate::apoio::Temp;

    #[test]
    fn traduz_so_o_que_o_pet_usa() {
        let t = |s: &str| traduzir(s.as_bytes());
        assert_eq!(
            t("focusedmonv2>>HDMI-A-1,3"),
            Some(Linha::MonitorEmFoco("HDMI-A-1".into()))
        );
        assert_eq!(t("focusedmonv2>>FALLBACK,1"), None, "nunca o FALLBACK");
        assert_eq!(t("focusedmonv2>>eDP 1,1"), None, "nome estranho");
        assert_eq!(
            t("activewindowv2>>5bbf4e6128f0"),
            Some(Linha::Ativa(Some(Alca("5bbf4e6128f0".into()))))
        );
        assert_eq!(t("activewindowv2>>"), Some(Linha::Ativa(None)));
        assert_eq!(t("activewindowv2>>,"), Some(Linha::Ativa(None)));
        assert_eq!(t("activewindowv2>>0x5bbf"), None, "sem 0x nos eventos");
        assert_eq!(t("activewindowv2>>ZZZZ"), None);
        assert_eq!(
            t("activewindow>>foot,\u{2733} Pensando na vida"),
            Some(Linha::OlhandoClaude(true))
        );
        assert_eq!(
            t("activewindow>>foot,\u{25D1} Trabalhando, com vírgula"),
            Some(Linha::OlhandoClaude(true))
        );
        assert_eq!(
            t("activewindow>>firefox,SEGREDO-T"),
            Some(Linha::OlhandoClaude(false))
        );
        assert_eq!(t("activewindow>>,"), Some(Linha::OlhandoClaude(false)));
        assert_eq!(
            t("openwindow>>abc124,2,org.omarchy.screensaver,Screensaver"),
            Some(Linha::Abriu {
                janela: Alca("abc124".into()),
                protetor: true
            })
        );
        assert_eq!(
            t("openwindow>>abc125,2,foot,título, com vírgula"),
            Some(Linha::Abriu {
                janela: Alca("abc125".into()),
                protetor: false
            })
        );
        assert_eq!(
            t("closewindow>>abc124"),
            Some(Linha::Fechou(Alca("abc124".into())))
        );
        assert_eq!(t("monitoraddedv2>>1,HDMI-A-1,Dell"), Some(Linha::Monitores));
        assert_eq!(
            t("monitorremovedv2>>1,HDMI-A-1,Dell"),
            Some(Linha::Monitores)
        );
        for ignorada in [
            "windowtitlev2>>abc,SEGREDO",
            "workspacev2>>2,SEGREDO",
            "configreloaded>>",
            "screencast>>1,0",
            "sem separador",
            "",
        ] {
            assert_eq!(t(ignorada), None, "{ignorada}");
        }
        // Título com bytes que não são UTF-8 ou um glifo partido: não é o
        // Claude, e nada quebra.
        let mut quebrada = b"activewindow>>foot,".to_vec();
        quebrada.extend_from_slice(&[0xE2, 0x9C]);
        assert_eq!(traduzir(&quebrada), Some(Linha::OlhandoClaude(false)));
    }

    #[test]
    fn nada_do_titulo_ou_da_classe_sobra_no_que_sai() {
        let linhas = [
            "activewindow>>firefox,SEGREDO-T",
            "openwindow>>abc124,SEGREDO-area,SEGREDO-classe,SEGREDO-titulo",
            "windowtitlev2>>abc124,SEGREDO-novo",
            "focusedmonv2>>eDP-1,SEGREDO",
        ];
        let mut repetidos = Repetidos::default();
        for linha in linhas {
            let traduzida = traduzir(linha.as_bytes());
            let texto = format!("{traduzida:?}");
            assert!(!texto.to_lowercase().contains("segredo"), "{texto}");
            assert!(!texto.contains("firefox"), "{texto}");
            if let Some(t) = traduzida {
                let evento = format!("{:?}", repetidos.filtrar(t, 1));
                assert!(!evento.to_lowercase().contains("segredo"), "{evento}");
                assert!(!evento.contains("firefox"), "{evento}");
            }
        }
    }

    #[test]
    fn repetidos_so_passam_quando_mudam() {
        let mut r = Repetidos::default();
        let a = || Linha::Ativa(Some(Alca("a1".into())));
        assert!(r.filtrar(a(), 10).is_some());
        assert!(
            r.filtrar(a(), 20).is_none(),
            "o spinner repete o activewindowv2"
        );
        assert!(r.filtrar(Linha::OlhandoClaude(true), 21).is_some());
        assert!(r.filtrar(Linha::OlhandoClaude(true), 22).is_none());
        assert_eq!(
            r.filtrar(Linha::Ativa(None), 30),
            Some(EventoDesktop::JanelaAtiva {
                janela: None,
                parede_ms: 30
            })
        );
        assert!(r.filtrar(a(), 40).is_some(), "voltou para a mesma: é troca");
        // A janela ativa fechou: a próxima ativação dela (outra janela com o
        // mesmo endereço, um dia) passa.
        assert!(r.filtrar(Linha::Fechou(Alca("a1".into())), 50).is_some());
        assert!(r.filtrar(a(), 60).is_some());
    }

    /// Um socket2 de mentira: cada conexão recebe as linhas do canal.
    fn socket2_falso(caminho: &std::path::Path) -> mpsc::Sender<Vec<u8>> {
        let ouvinte = UnixListener::bind(caminho).unwrap();
        let (manda, recebe) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let Ok((mut fluxo, _)) = ouvinte.accept() else {
                return;
            };
            for pedaco in recebe {
                if pedaco.is_empty() {
                    // Pedaço vazio: derruba esta conexão e espera outra.
                    drop(fluxo);
                    match ouvinte.accept() {
                        Ok((novo, _)) => fluxo = novo,
                        Err(_) => return,
                    }
                    continue;
                }
                if fluxo.write_all(&pedaco).is_err() {
                    return;
                }
            }
        });
        manda
    }

    fn proximo(recebe: &mpsc::Receiver<EventoDesktop>) -> EventoDesktop {
        recebe
            .recv_timeout(Duration::from_secs(5))
            .expect("um evento do leitor")
    }

    #[test]
    fn le_traduz_reconecta_e_para() {
        let t = Temp::nova("socket2");
        let caminho = t.0.join(".socket2.sock");
        let escreve = socket2_falso(&caminho);
        let (caixa, recebe) = Caixa::nova(64, Arc::new(SemDespertador));
        let mut leitor = Leitor::iniciar_com(
            caminho,
            t.0.clone(),
            "teste".into(),
            caixa,
            Duration::from_millis(20),
        )
        .unwrap();
        assert_eq!(proximo(&recebe), EventoDesktop::Ligado(true));
        escreve
            .send(
                b"activewindow>>firefox,SEGREDO-T\nactivewindowv2>>abc123\nactivewindowv2>>abc123\nfocusedm"
                    .to_vec(),
            )
            .unwrap();
        escreve.send(b"onv2>>HDMI-A-1,3\n".to_vec()).unwrap();
        assert_eq!(proximo(&recebe), EventoDesktop::OlhandoClaude(false));
        let EventoDesktop::JanelaAtiva { janela, parede_ms } = proximo(&recebe) else {
            panic!("esperava a janela ativa");
        };
        assert_eq!(janela, Some(Alca("abc123".into())));
        assert!(parede_ms > 1_700_000_000_000, "hora de parede");
        assert_eq!(
            proximo(&recebe),
            EventoDesktop::MonitorEmFoco("HDMI-A-1".into()),
            "a repetida não passou e a linha partida juntou"
        );
        // Uma linha enorme é pulada sem derrubar a leitura.
        let mut grande = b"activewindow>>x,".to_vec();
        grande.extend(std::iter::repeat_n(b'a', 10_000));
        grande.extend_from_slice(b"\nclosewindow>>abc123\n");
        escreve.send(grande).unwrap();
        assert_eq!(
            proximo(&recebe),
            EventoDesktop::JanelaFechou(Alca("abc123".into()))
        );
        // O Hyprland derruba: o leitor conta, espera o backoff e liga de novo.
        escreve.send(Vec::new()).unwrap();
        assert_eq!(proximo(&recebe), EventoDesktop::Ligado(false));
        assert_eq!(proximo(&recebe), EventoDesktop::Ligado(true));
        escreve.send(b"activewindowv2>>abc123\n".to_vec()).unwrap();
        assert!(
            matches!(proximo(&recebe), EventoDesktop::JanelaAtiva { .. }),
            "a memória dos repetidos recomeça na ligação nova"
        );
        let antes = Instant::now();
        leitor.parar();
        assert!(antes.elapsed() < Duration::from_secs(2), "parar não espera");
        assert_eq!(leitor.perdidos(), 0);
    }

    #[test]
    fn caixa_cheia_vira_buraco_e_a_proxima_ativacao_passa() {
        // O laço atrasado não esvazia a caixa (3 vagas): a3 se perde, e o
        // a4 também (nem o aviso de que a fonte "caiu" coube).
        let t = Temp::nova("socket2-cheia");
        let caminho = t.0.join(".socket2.sock");
        let escreve = socket2_falso(&caminho);
        let (caixa, recebe) = Caixa::nova(3, Arc::new(SemDespertador));
        let mut leitor = Leitor::iniciar_com(
            caminho,
            t.0.clone(),
            "cheia".into(),
            caixa,
            Duration::from_millis(20),
        )
        .unwrap();
        escreve
            .send(
                b"activewindowv2>>a1\nactivewindowv2>>a2\nactivewindowv2>>a3\nactivewindowv2>>a4\n"
                    .to_vec(),
            )
            .unwrap();
        let limite = Instant::now() + Duration::from_secs(5);
        while leitor.perdidos() < 2 {
            assert!(Instant::now() < limite, "nada se perdeu");
            std::thread::sleep(Duration::from_millis(10));
        }
        let ativa = |a: &str| EventoDesktop::JanelaAtiva {
            janela: Some(Alca(a.into())),
            parede_ms: 0,
        };
        let sem_hora = |e: EventoDesktop| match e {
            EventoDesktop::JanelaAtiva { janela, .. } => EventoDesktop::JanelaAtiva {
                janela,
                parede_ms: 0,
            },
            outro => outro,
        };
        assert_eq!(proximo(&recebe), EventoDesktop::Ligado(true));
        assert_eq!(sem_hora(proximo(&recebe)), ativa("a1"));
        assert_eq!(sem_hora(proximo(&recebe)), ativa("a2"));
        // O laço esvaziou. A próxima linha (o a4 de novo, que o filtro das
        // repetidas seguraria) chega depois de a fonte "cair e voltar": o
        // anel ganha um buraco no lugar das trocas perdidas.
        escreve.send(b"activewindowv2>>a4\n".to_vec()).unwrap();
        assert_eq!(proximo(&recebe), EventoDesktop::Ligado(false));
        assert_eq!(proximo(&recebe), EventoDesktop::Ligado(true));
        assert_eq!(sem_hora(proximo(&recebe)), ativa("a4"));
        assert_eq!(leitor.perdidos(), 2);
        leitor.parar();
    }

    #[test]
    fn sem_socket_tenta_de_novo_e_para_na_hora() {
        let t = Temp::nova("socket2-ausente");
        let (caixa, recebe) = Caixa::nova(4, Arc::new(SemDespertador));
        let mut leitor = Leitor::iniciar_com(
            t.0.join("nao-existe.sock"),
            t.0.clone(),
            "ausente".into(),
            caixa,
            Duration::from_secs(30),
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(recebe.try_recv().is_err(), "nunca ligou");
        let antes = Instant::now();
        leitor.parar();
        assert!(
            antes.elapsed() < Duration::from_secs(2),
            "o parar acorda o backoff de 30 s"
        );
    }
}
