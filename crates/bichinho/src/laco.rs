//! O laço do Linux (calloop) e a sessão Wayland (decisões 0007 e 0040).
//!
//! Tudo que mexe no Wayland roda nesta thread: descoberta, sessão, prazos.
//! O que o pet decide mora no [`Nucleo`] (e no Motor do `pet-core`); este
//! laço só traz as fontes:
//!
//! - **batimento** de 5 s, incondicional: alimenta o vigia e o `/saude`
//!   mesmo com a tela apagada (DPMS) ou em sono profundo, quando não há
//!   frame callback nem animação (revisão de viabilidade);
//! - **descoberta** a cada 2 s, só enquanto espera o compositor (o
//!   adaptador do Hyprland, [`pet_wayland::hyprland`]);
//! - a **conexão Wayland** ([`WaylandSource`]) enquanto há sessão;
//! - o **pipe de sinais** (SIGTERM/SIGINT) para encerrar sem atraso;
//! - a **caixa** ([`Caixa`]) das outras threads: eventos dos hooks,
//!   `/v1/comando` e rotas de debug, acordando o laço por um `Ping`;
//! - a **caixa do desktop**: os eventos do socket2 do Hyprland, lidos numa
//!   thread só para isso ([`Leitor`], decisão 0050), que nasce com a
//!   instância do Hyprland achada pela descoberta e morre com ela. Ela é
//!   esvaziada antes da caixa dos hooks: a troca de janela que veio antes
//!   de um prompt entra no anel antes dele;
//! - um **prazo** só, o mais próximo entre os do Motor (cérebro e animação)
//!   e os da janela, rearmado depois de cada lote.
//!
//! Erro na conexão Wayland (EOF, erro de protocolo) sai do `dispatch` do
//! calloop: aí a sessão é derrubada inteira e o laço volta a esperar.

use std::io::Read;
use std::net::TcpListener;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use pet_core::config::ConfigEfetiva;
use pet_core::plataforma::{Caixa, Despertador, EventoDesktop, Punho};
use pet_wayland::conexao::{self, Reconexao};
use pet_wayland::hyprland::eventos::Leitor;
use pet_wayland::hyprland::{self, Espera};
use pet_wayland::sessao::Sessao;
use signal_hook::consts::{SIGINT, SIGTERM};
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::ping::{self, Ping, PingSource};
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{
    EventLoop, Interest, LoopHandle, Mode, PostAction, RegistrationToken,
};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;

use crate::ambiente::Ambiente;
use crate::comando::{self, Comando};
use crate::estado::Compartilhado;
use crate::nucleo::Nucleo;
use crate::{daemon, vigia};

/// Batimento do laço principal (o vigia aborta com 60 s sem batimento).
pub const BATIMENTO: Duration = Duration::from_secs(5);
/// Intervalo da descoberta enquanto espera o compositor.
pub const INTERVALO_DESCOBERTA: Duration = Duration::from_secs(2);
/// Quantos comandos o laço tira da caixa de uma vez (o resto vem no próximo
/// toque do `Ping`).
const LOTE_DA_CAIXA: usize = 4 * comando::CAPACIDADE;
/// Eventos do desktop esperando o laço (o leitor só manda as mudanças: uma
/// troca de janela, de monitor ou de presença).
const CAPACIDADE_DESKTOP: usize = 1024;

/// O `Ping` do calloop acorda o laço quando chega algo na caixa.
struct Acordar(Ping);

impl Despertador for Acordar {
    fn despertar(&self) {
        self.0.ping();
    }
}

/// O daemon no Linux: liga a entrada HTTP à caixa deste laço e roda até um
/// SIGTERM.
pub fn rodar(
    ambiente: Ambiente,
    config: ConfigEfetiva,
    comp: Arc<Compartilhado>,
    ouvinte: TcpListener,
) -> ExitCode {
    let (sino, fonte_do_sino) = match ping::make_ping() {
        Ok(par) => par,
        Err(e) => {
            erro!("não consegui criar o despertador do laço: {e}");
            return ExitCode::FAILURE;
        }
    };
    let (caixa, recebe) = Caixa::nova(comando::CAPACIDADE, Arc::new(Acordar(sino)));
    let (sino_do_desktop, fonte_do_desktop) = match ping::make_ping() {
        Ok(par) => par,
        Err(e) => {
            erro!("não consegui criar o despertador dos eventos do desktop: {e}");
            return ExitCode::FAILURE;
        }
    };
    let (caixa_do_desktop, recebe_do_desktop) =
        Caixa::nova(CAPACIDADE_DESKTOP, Arc::new(Acordar(sino_do_desktop)));
    if let Err(e) = daemon::iniciar_entrada(&ambiente, &comp, ouvinte, caixa) {
        erro!("{e}");
        return ExitCode::FAILURE;
    }
    vigia::iniciar(Arc::clone(&comp));

    let mut eventos: EventLoop<'static, Laco> = match EventLoop::try_new() {
        Ok(eventos) => eventos,
        Err(e) => {
            erro!("não consegui criar o laço de eventos: {e}");
            return ExitCode::FAILURE;
        }
    };
    let base = conexao::base_do_usuario(&ambiente.runtime_host);
    let curto = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let nucleo = Nucleo::novo(
        Arc::clone(&comp),
        ambiente.onde(),
        Some(ambiente.pasta_config.clone()),
        &config,
        Instant::now(),
    );
    let mut laco = Laco::novo(
        nucleo,
        eventos.handle(),
        base.clone(),
        curto,
        recebe,
        (caixa_do_desktop, recebe_do_desktop),
    );
    if let Err(e) = laco.instalar_sinais() {
        erro!("não consegui tratar SIGTERM/SIGINT: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = laco.instalar_caixa(fonte_do_sino) {
        erro!("não consegui ligar a caixa de comandos: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = laco.instalar_desktop(fonte_do_desktop) {
        erro!("não consegui ligar a caixa dos eventos do desktop: {e}");
        return ExitCode::FAILURE;
    }
    laco.armar_batimento();
    laco.armar_descoberta(Duration::ZERO);
    laco.nucleo.iniciar_cerebro();
    laco.assentar();

    info!(
        "bichinho {} escutando em {} (porta pública {}){}",
        pet_core::VERSAO,
        ambiente.escuta,
        ambiente.porta_publica,
        if ambiente.debug { ", modo debug" } else { "" }
    );
    depurar!(
        "config em {}, estado em {}",
        ambiente.arquivo_config().display(),
        ambiente.pasta_estado.display(),
    );
    info!("procurando o compositor em {}", base.display());

    while !laco.parar {
        if let Err(e) = eventos.dispatch(None, &mut laco) {
            laco.falha_no_laco(e);
        }
    }
    laco.encerrar();
    ExitCode::SUCCESS
}

pub struct Laco {
    nucleo: Nucleo,
    handle: LoopHandle<'static, Laco>,
    /// Runtime do usuário no host (`/host/run/user/<uid>`).
    base: PathBuf,
    /// `XDG_RUNTIME_DIR` privado, para symlinks curtos.
    curto: PathBuf,
    reconexao: Reconexao,
    viva: Option<Viva>,
    descoberta: Option<RegistrationToken>,
    batimento: Option<RegistrationToken>,
    /// O timer do próximo prazo (Motor ou janela) e para quando ele está
    /// armado (ms no relógio do laço).
    prazo: Option<(u64, RegistrationToken)>,
    /// Última razão de espera registrada no log (só loga quando muda).
    ultima_espera: Option<String>,
    caixa: mpsc::Receiver<Comando>,
    /// Os eventos do desktop e a caixa que o leitor recebe.
    desktop: mpsc::Receiver<EventoDesktop>,
    caixa_do_desktop: Caixa<EventoDesktop>,
    /// O leitor do socket2 da instância achada (assinatura e leitor).
    leitor: Option<(String, Leitor)>,
    pub parar: bool,
}

/// Uma sessão Wayland conectada.
struct Viva {
    sessao: Sessao,
    fonte: RegistrationToken,
    assinatura: String,
    desde: Instant,
}

impl Laco {
    fn novo(
        nucleo: Nucleo,
        handle: LoopHandle<'static, Laco>,
        base: PathBuf,
        curto: PathBuf,
        caixa: mpsc::Receiver<Comando>,
        (caixa_do_desktop, desktop): (Caixa<EventoDesktop>, mpsc::Receiver<EventoDesktop>),
    ) -> Laco {
        Laco {
            nucleo,
            handle,
            base,
            curto,
            reconexao: Reconexao::default(),
            viva: None,
            descoberta: None,
            batimento: None,
            prazo: None,
            ultima_espera: None,
            caixa,
            desktop,
            caixa_do_desktop,
            leitor: None,
            parar: false,
        }
    }

    /// Depois de cada lote (caixa, prazo, conexão): os eventos da janela, o
    /// painel e o próximo prazo.
    fn assentar(&mut self) {
        self.assentar_com(true);
    }

    /// Depois de uma leva do Wayland, que chega a cada frame callback e a
    /// cada movimento do ponteiro sobre o pet (o calloop chama de novo até a
    /// fila esvaziar): o painel só sai se um evento mudou o pet ou a janela,
    /// e o prazo só é rearmado se mudou. O batimento de 5 s republica de todo
    /// jeito.
    fn assentar_wayland(&mut self) {
        self.assentar_com(false);
    }

    fn assentar_com(&mut self, publicar_sempre: bool) {
        let janela = self
            .viva
            .as_mut()
            .map(|viva| &mut viva.sessao as &mut dyn Punho);
        let mudou = self.nucleo.eventos_da_janela(janela);
        if publicar_sempre || mudou {
            let janela = self.viva.as_ref().map(|viva| &viva.sessao as &dyn Punho);
            self.nucleo.publicar(janela);
        }
        self.armar_prazo();
    }

    /// SIGTERM e SIGINT acordam o laço por um pipe e pedem para parar.
    fn instalar_sinais(&mut self) -> std::io::Result<()> {
        let (leitura, escrita) = UnixStream::pair()?;
        leitura.set_nonblocking(true)?;
        for sinal in [SIGTERM, SIGINT] {
            signal_hook::low_level::pipe::register(sinal, escrita.try_clone()?)?;
        }
        let fonte = Generic::new(leitura, Interest::READ, Mode::Level);
        self.handle
            .insert_source(fonte, |_, leitura, laco| {
                let mut lixo = [0u8; 64];
                while matches!((&**leitura).read(&mut lixo), Ok(n) if n > 0) {}
                info!("encerrando: sinal recebido");
                laco.parar = true;
                Ok(PostAction::Continue)
            })
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(())
    }

    /// A caixa das outras threads (eventos, `/v1/comando` com reações e
    /// aprovações, debug): o `Ping` acorda o laço e ele a esvazia.
    fn instalar_caixa(&mut self, fonte: PingSource) -> Result<(), String> {
        self.handle
            .insert_source(fonte, |_, _, laco| {
                laco.esvaziar_caixa();
                laco.assentar();
            })
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Os eventos do desktop (o socket2), na ordem em que chegaram.
    fn instalar_desktop(&mut self, fonte: PingSource) -> Result<(), String> {
        self.handle
            .insert_source(fonte, |_, _, laco| {
                laco.esvaziar_desktop();
                laco.assentar();
            })
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn esvaziar_desktop(&mut self) {
        for _ in 0..CAPACIDADE_DESKTOP {
            let Ok(evento) = self.desktop.try_recv() else {
                return;
            };
            let punho = self
                .viva
                .as_mut()
                .map(|viva| &mut viva.sessao as &mut dyn Punho);
            self.nucleo.evento_desktop(&evento, punho);
        }
    }

    /// O leitor do socket2 da instância `assinatura`: o mesmo se ela não
    /// mudou; um novo (e o velho parado) se mudou.
    fn garantir_leitor(&mut self, assinatura: &str, eventos: &std::path::Path) {
        if self.leitor.as_ref().is_some_and(|(a, _)| a == assinatura) {
            return;
        }
        self.parar_leitor();
        match Leitor::iniciar(
            eventos.to_owned(),
            self.curto.clone(),
            format!("eventos-{assinatura}"),
            self.caixa_do_desktop.clone(),
        ) {
            Ok(leitor) => self.leitor = Some((assinatura.to_owned(), leitor)),
            Err(e) => aviso!("não consegui iniciar o leitor dos eventos do Hyprland: {e}"),
        }
    }

    /// Para o leitor do socket2 (não há instância do Hyprland).
    fn parar_leitor(&mut self) {
        if let Some((_, mut leitor)) = self.leitor.take() {
            leitor.parar();
        }
    }

    fn esvaziar_caixa(&mut self) {
        // As trocas de janela que chegaram antes entram no anel antes dos
        // eventos dos hooks (o prompt é casado com a janela ativa na hora).
        self.esvaziar_desktop();
        for _ in 0..LOTE_DA_CAIXA {
            let Ok(comando) = self.caixa.try_recv() else {
                return;
            };
            let janela = self
                .viva
                .as_mut()
                .map(|viva| &mut viva.sessao as &mut dyn Punho);
            self.nucleo.comando(comando, janela);
        }
    }

    /// (Re)arma o prazo mais próximo: o do Motor (cérebro e animação) ou o
    /// da janela. O mesmo prazo já armado fica como está (tirar e pôr de
    /// novo um timer que o calloop já separou para esta volta o atrasaria
    /// uma volta). Quando vence, os comandos que já estão na caixa entram
    /// antes: chegaram antes do prazo, e um deles pode cancelar uma
    /// acomodação (decisão 0032).
    fn armar_prazo(&mut self) {
        let janela = self.viva.as_ref().map(|viva| &viva.sessao as &dyn Punho);
        let proximo = self.nucleo.proximo_prazo(janela);
        if proximo.is_some() && proximo == self.prazo.as_ref().map(|(armado, _)| *armado) {
            return;
        }
        if let Some((_, token)) = self.prazo.take() {
            self.handle.remove(token);
        }
        let Some(prazo) = proximo else {
            return;
        };
        let quando = self.nucleo.inicio() + Duration::from_millis(prazo);
        let inserido = self
            .handle
            .insert_source(Timer::from_deadline(quando), |_, _, laco| {
                // Este timer acaba aqui; o próximo é armado no assentar.
                laco.prazo = None;
                laco.esvaziar_caixa();
                let janela = laco
                    .viva
                    .as_mut()
                    .map(|viva| &mut viva.sessao as &mut dyn Punho);
                laco.nucleo.vencer(janela);
                laco.assentar();
                TimeoutAction::Drop
            });
        match inserido {
            Ok(token) => self.prazo = Some((prazo, token)),
            Err(e) => erro!("não consegui armar o prazo: {e}"),
        }
    }

    /// (Re)arma o batimento de 5 s. Chamado na partida e depois de um erro
    /// no `dispatch`, porque o calloop tira os timers vencidos da fila antes
    /// de despachar: se outra fonte falha no mesmo ciclo, o evento do timer
    /// se perde. Rearmar **não** bate: só o timer disparando prova que o
    /// laço anda. Um `dispatch` que falhasse sempre rearmaria sem nunca
    /// bater, e o vigia abortaria em 60 s (o Docker reinicia).
    fn armar_batimento(&mut self) {
        if let Some(token) = self.batimento.take() {
            self.handle.remove(token);
        }
        let timer = Timer::from_duration(BATIMENTO);
        match self.handle.insert_source(timer, |_, _, laco| {
            laco.nucleo.comp.bater();
            if laco.viva.is_some() {
                let janela = laco.viva.as_ref().map(|viva| &viva.sessao as &dyn Punho);
                laco.nucleo.publicar(janela);
            }
            TimeoutAction::ToDuration(BATIMENTO)
        }) {
            Ok(token) => self.batimento = Some(token),
            Err(e) => erro!("não consegui armar o batimento: {e}"),
        }
    }

    /// Agenda a próxima rodada de descoberta.
    fn armar_descoberta(&mut self, daqui_a: Duration) {
        if let Some(token) = self.descoberta.take() {
            self.handle.remove(token);
        }
        let timer = Timer::from_duration(daqui_a);
        match self
            .handle
            .insert_source(timer, |_, _, laco| match laco.descobrir() {
                Some(proxima) => TimeoutAction::ToDuration(proxima),
                None => {
                    laco.descoberta = None;
                    TimeoutAction::Drop
                }
            }) {
            Ok(token) => self.descoberta = Some(token),
            Err(e) => erro!("não consegui armar a descoberta: {e}"),
        }
    }

    /// Uma rodada de descoberta. Devolve quando tentar de novo, ou `None` se
    /// conectou.
    fn descobrir(&mut self) -> Option<Duration> {
        if self.viva.is_some() {
            return None;
        }
        let agora = Instant::now();
        let reconexao = &self.reconexao;
        let em_espera = |assinatura: &str| {
            (!reconexao.pode_tentar(assinatura, agora))
                .then(|| reconexao.espera_restante(assinatura, agora))
        };
        let instancia = match hyprland::procurar(&self.base, &self.curto, em_espera) {
            Ok(instancia) => instancia,
            Err(espera) => {
                if matches!(espera, Espera::SemRuntime | Espera::SemHyprland) {
                    self.parar_leitor();
                }
                self.registrar_espera(espera.descrever());
                return Some(match espera {
                    Espera::Recuo { restante } => {
                        restante.clamp(Duration::from_millis(100), INTERVALO_DESCOBERTA)
                    }
                    _ => INTERVALO_DESCOBERTA,
                });
            }
        };
        // O socket2 da instância é lido desde já, mesmo se a conexão Wayland
        // falhar e tiver de esperar o backoff.
        self.garantir_leitor(&instancia.assinatura, &instancia.eventos);
        let assinatura = instancia.assinatura;
        let nome_wayland = instancia.nome_wayland;
        let conexao = match pet_wayland::conectar(instancia.wayland, self.nucleo.inicio()) {
            Ok(conexao) => conexao,
            Err(motivo) => {
                let atraso = self.reconexao.falhou(&assinatura, Duration::ZERO, agora);
                aviso!(
                    "falha ao conectar em {nome_wayland}: {motivo}; nova tentativa em {} s",
                    atraso.as_secs()
                );
                return Some(atraso.min(INTERVALO_DESCOBERTA));
            }
        };
        let pet_wayland::Conexao {
            conexao,
            fila,
            sessao,
        } = conexao;
        let fonte = WaylandSource::new(conexao, fila);
        let token = match self.handle.insert_source(fonte, |_, fila, laco| {
            let resultado = match laco.viva.as_mut() {
                Some(viva) => fila.dispatch_pending(&mut viva.sessao),
                None => Ok(0),
            };
            laco.assentar_wayland();
            resultado
        }) {
            Ok(token) => token,
            Err(e) => {
                erro!("não consegui registrar a conexão Wayland no laço: {e}");
                self.reconexao.falhou(&assinatura, Duration::ZERO, agora);
                return Some(INTERVALO_DESCOBERTA);
            }
        };
        info!("conectado ao compositor ({nome_wayland}, instância {assinatura})");
        self.ultima_espera = None;
        let viva = self.viva.insert(Viva {
            sessao,
            fonte: token,
            assinatura,
            desde: agora,
        });
        self.nucleo.conectou(&mut viva.sessao);
        self.assentar();
        None
    }

    fn registrar_espera(&mut self, motivo: String) {
        if self.ultima_espera.as_deref() != Some(motivo.as_str()) {
            info!("aguardando compositor: {motivo}");
            self.ultima_espera = Some(motivo);
        }
    }

    /// Fim do processo: esconde o pet (quadro transparente), destrói a
    /// camada e espera, com prazo, o compositor confirmar que processou tudo,
    /// para o fade de saída do Hyprland sair vazio.
    fn encerrar(&mut self) {
        if let Some(viva) = self.viva.as_mut() {
            self.nucleo.encerrar(&mut viva.sessao);
        }
    }

    /// O `dispatch` do calloop falhou. A única fonte que falha é a conexão
    /// Wayland: derruba a sessão e volta a esperar.
    fn falha_no_laco(&mut self, erro: impl std::fmt::Display) {
        if self.viva.is_some() {
            self.derrubar(&format!("conexão Wayland caiu: {erro}"));
        } else {
            aviso!("erro no laço principal sem sessão: {erro}");
            std::thread::sleep(Duration::from_millis(100));
        }
        self.armar_batimento();
    }

    /// Desmonta a sessão Wayland e volta a esperar o compositor.
    fn derrubar(&mut self, motivo: &str) {
        let Some(viva) = self.viva.take() else {
            return;
        };
        self.handle.remove(viva.fonte);
        let viveu = viva.desde.elapsed();
        drop(viva.sessao);
        self.nucleo.desconectou();
        let atraso = self
            .reconexao
            .falhou(&viva.assinatura, viveu, Instant::now());
        aviso!(
            "{motivo}; sessão durou {} s; aguardando compositor (mesma instância só daqui a {} s)",
            viveu.as_secs(),
            atraso.as_secs()
        );
        self.armar_descoberta(Duration::ZERO);
        self.armar_prazo();
    }
}
