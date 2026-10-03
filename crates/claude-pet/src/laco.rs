//! Laço principal do daemon (calloop).
//!
//! Tudo que mexe no Wayland roda nesta thread: descoberta, sessão, timers.
//! Fontes do laço:
//!
//! - **batimento** de 5 s, incondicional: alimenta o vigia e o `/saude`
//!   mesmo com a tela apagada (DPMS) ou em sono profundo, quando não há
//!   frame callback nem animação (revisão de viabilidade);
//! - **descoberta** a cada 2 s, só enquanto espera o compositor;
//! - a **conexão Wayland** ([`WaylandSource`]) enquanto há sessão;
//! - o **pipe de sinais** (SIGTERM/SIGINT) para encerrar sem atraso.
//!
//! Erro na conexão Wayland (EOF, erro de protocolo) sai do `dispatch` do
//! calloop: aí a sessão é derrubada inteira e o laço volta a esperar.

use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use signal_hook::consts::{SIGINT, SIGTERM};
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{
    Interest, LoopHandle, Mode, PostAction, RegistrationToken,
};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;

use crate::descoberta::{self, Reconexao};
use crate::estado::{Compartilhado, Tela};
use crate::wl;

/// Batimento do laço principal (o vigia aborta com 60 s sem batimento).
pub const BATIMENTO: Duration = Duration::from_secs(5);
/// Intervalo da descoberta enquanto espera o compositor.
pub const INTERVALO_DESCOBERTA: Duration = Duration::from_secs(2);

pub struct Laco {
    pub comp: Arc<Compartilhado>,
    handle: LoopHandle<'static, Laco>,
    /// Runtime do usuário no host (`/host/run/user/<uid>`).
    base: PathBuf,
    /// `XDG_RUNTIME_DIR` privado, para symlinks curtos.
    curto: PathBuf,
    reconexao: Reconexao,
    viva: Option<Viva>,
    descoberta: Option<RegistrationToken>,
    batimento: Option<RegistrationToken>,
    /// Última razão de espera registrada no log (só loga quando muda).
    ultima_espera: Option<String>,
    /// O pet deve aparecer quando houver compositor.
    visivel: bool,
    pub parar: bool,
}

/// Uma sessão Wayland conectada.
struct Viva {
    sessao: wl::Sessao,
    fonte: RegistrationToken,
    assinatura: String,
    desde: Instant,
}

impl Laco {
    pub fn novo(
        comp: Arc<Compartilhado>,
        handle: LoopHandle<'static, Laco>,
        base: PathBuf,
        curto: PathBuf,
        visivel: bool,
    ) -> Laco {
        Laco {
            comp,
            handle,
            base,
            curto,
            reconexao: Reconexao::default(),
            viva: None,
            descoberta: None,
            batimento: None,
            ultima_espera: None,
            visivel,
            parar: false,
        }
    }

    /// A sessão Wayland, se conectada.
    pub fn sessao_mut(&mut self) -> Option<&mut wl::Sessao> {
        self.viva.as_mut().map(|viva| &mut viva.sessao)
    }

    /// SIGTERM e SIGINT acordam o laço por um pipe e pedem para parar.
    pub fn instalar_sinais(&mut self) -> std::io::Result<()> {
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

    /// (Re)arma o batimento de 5 s. Chamado na partida e depois de um erro
    /// no `dispatch`, porque um evento de timer pode se perder quando outra
    /// fonte falha no mesmo ciclo.
    pub fn armar_batimento(&mut self) {
        if let Some(token) = self.batimento.take() {
            self.handle.remove(token);
        }
        self.comp.bater();
        let timer = Timer::from_duration(BATIMENTO);
        match self.handle.insert_source(timer, |_, _, laco| {
            laco.comp.bater();
            TimeoutAction::ToDuration(BATIMENTO)
        }) {
            Ok(token) => self.batimento = Some(token),
            Err(e) => erro!("não consegui armar o batimento: {e}"),
        }
    }

    /// Agenda a próxima rodada de descoberta.
    pub fn armar_descoberta(&mut self, daqui_a: Duration) {
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
        let instancia = match descoberta::procurar(&self.base, &self.curto) {
            Ok(instancia) => instancia,
            Err(espera) => {
                self.registrar_espera(espera.descrever());
                return Some(INTERVALO_DESCOBERTA);
            }
        };
        let agora = Instant::now();
        if !self.reconexao.pode_tentar(&instancia.assinatura, agora) {
            let falta = self.reconexao.espera_restante(&instancia.assinatura, agora);
            self.registrar_espera(format!(
                "nova tentativa em {} daqui a {} s",
                instancia.nome_wayland,
                falta.as_secs_f32().ceil()
            ));
            return Some(falta.clamp(Duration::from_millis(100), INTERVALO_DESCOBERTA));
        }
        let assinatura = instancia.assinatura;
        let nome_wayland = instancia.nome_wayland;
        let conexao = match wl::conectar(instancia.wayland, self.handle.clone()) {
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
        let wl::Conexao {
            conexao,
            fila,
            sessao,
        } = conexao;
        let fonte = WaylandSource::new(conexao, fila);
        let token =
            match self
                .handle
                .insert_source(fonte, |_, fila, laco| match laco.viva.as_mut() {
                    Some(viva) => fila.dispatch_pending(&mut viva.sessao),
                    None => Ok(0),
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
        self.viva = Some(Viva {
            sessao,
            fonte: token,
            assinatura,
            desde: agora,
        });
        self.comp.definir_tela(Tela::Ativa);
        let visivel = self.visivel;
        if let Some(sessao) = self.sessao_mut() {
            sessao.definir_visivel(visivel);
        }
        None
    }

    fn registrar_espera(&mut self, motivo: String) {
        if self.ultima_espera.as_deref() != Some(motivo.as_str()) {
            info!("aguardando compositor: {motivo}");
            self.ultima_espera = Some(motivo);
        }
    }

    /// O `dispatch` do calloop falhou. A única fonte que falha é a conexão
    /// Wayland: derruba a sessão e volta a esperar.
    pub fn falha_no_laco(&mut self, erro: impl std::fmt::Display) {
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
        let atraso = self
            .reconexao
            .falhou(&viva.assinatura, viveu, Instant::now());
        aviso!(
            "{motivo}; sessão durou {} s; aguardando compositor (mesma instância só daqui a {} s)",
            viveu.as_secs(),
            atraso.as_secs()
        );
        self.comp.definir_tela(Tela::Aguardando);
        self.armar_descoberta(Duration::ZERO);
    }
}
