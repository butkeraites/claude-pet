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
//! - o **pipe de sinais** (SIGTERM/SIGINT) para encerrar sem atraso;
//! - o **canal de comandos** das outras threads: eventos dos hooks,
//!   `/v1/comando` e rotas de debug;
//! - o **prazo do cérebro** (acomodação do Stop, espera pelo Stop de um
//!   turno trocado, sessões que expiram), um timer só, rearmado depois de
//!   cada evento.
//!
//! O cérebro ([`Cerebro`]) mora aqui, no laço, e funciona com ou sem
//! compositor: sem tela, as reações só ficam no `/v1/estado`.
//!
//! Erro na conexão Wayland (EOF, erro de protocolo) sai do `dispatch` do
//! calloop: aí a sessão é derrubada inteira e o laço volta a esperar.

use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use signal_hook::consts::{SIGINT, SIGTERM};
use smithay_client_toolkit::reexports::calloop::channel::{self, Channel};
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{
    Interest, LoopHandle, Mode, PostAction, RegistrationToken,
};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;

use pet_core::aprovacao::Origem;
use pet_core::cerebro::{Agora, Cerebro, ConfigCerebro, Reacao};
use pet_core::config::ConfigEfetiva;
use pet_core::evento;
use pet_core::skin::Skin;

use crate::comando::{Comando, Recebido};
use crate::descoberta::{self, Espera, Reconexao};
use crate::estado::{Compartilhado, InfoSkin, Painel, Tela};
use crate::ingress::agora_desde_1970_ms;
use crate::personagem::{self, Escolha, Onde};
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
    /// O personagem: a skin aprovada (ou a de teste, em debug).
    skin: Option<Rc<Skin>>,
    /// Quem está na tela: (id, impressão digital, origem). Uma aprovação que
    /// não muda isso (aprovar de novo a mesma skin, revogar outra) não mexe
    /// na tela.
    na_tela: NaTela,
    /// Onde procurar skins e aprovações, para escolher de novo.
    onde: Onde,
    /// A skin configurada (`aparencia.skin`).
    configurada: String,
    /// `claude-pet.toml`, relido a cada aprovação: trocar `aparencia.skin`
    /// (o `zeca-contorno`, por exemplo) vale sem reiniciar o container.
    arquivo_config: Option<PathBuf>,
    /// O pet deve estar na tela (`/v1/comando` pode esconder e mostrar).
    visivel: bool,
    cerebro: Cerebro,
    /// Timer do próximo prazo do cérebro.
    prazo_cerebro: Option<RegistrationToken>,
    /// Origem do relógio monotônico do cérebro.
    inicio: Instant,
    pub parar: bool,
}

/// Quem está na tela: (id, impressão digital, origem).
type NaTela = Option<(String, Option<String>, Option<Origem>)>;

/// O relógio do cérebro para um evento: a hora em que o ingress o recebeu
/// (parede e monotônico desde `inicio`), não a hora do processamento. Se o
/// laço ficou parado (o handshake do Wayland, uma troca de personagem), os
/// eventos da fila entram na hora em que chegaram: o calloop despacha o
/// canal antes dos timers vencidos, e uma acomodação que um desses eventos
/// cancela não vence antes dele (decisão 0032).
fn relogio_da_chegada(inicio: Instant, recebido: &Recebido) -> Agora {
    Agora {
        parede_ms: recebido.recebido_ms,
        mono_ms: recebido
            .chegada
            .saturating_duration_since(inicio)
            .as_millis() as u64,
    }
}

/// A escolha nova é exatamente o personagem que já está na tela (aprovar de
/// novo a mesma skin, revogar a de outro id): nada a trocar.
fn mesma_tela(havia_skin: bool, antes: &NaTela, depois: &NaTela) -> bool {
    havia_skin && depois.is_some() && antes == depois
}

/// Uma sessão Wayland conectada.
struct Viva {
    sessao: wl::Sessao,
    fonte: RegistrationToken,
    assinatura: String,
    desde: Instant,
}

impl Laco {
    /// `config` é a da partida: dela saem a skin configurada e a config do
    /// cérebro; `arquivo_config` é de onde ela é relida a cada aprovação.
    pub fn novo(
        comp: Arc<Compartilhado>,
        handle: LoopHandle<'static, Laco>,
        base: PathBuf,
        curto: PathBuf,
        onde: Onde,
        arquivo_config: Option<PathBuf>,
        config: &ConfigEfetiva,
    ) -> Laco {
        let mut laco = Laco {
            comp,
            handle,
            base,
            curto,
            reconexao: Reconexao::default(),
            viva: None,
            descoberta: None,
            batimento: None,
            ultima_espera: None,
            skin: None,
            na_tela: None,
            onde,
            configurada: config.texto("aparencia.skin").to_owned(),
            arquivo_config,
            visivel: true,
            cerebro: Cerebro::novo(ConfigCerebro::de(&config.config())),
            prazo_cerebro: None,
            inicio: Instant::now(),
            parar: false,
        };
        laco.escolher_personagem();
        laco
    }

    /// Relê o `claude-pet.toml` (o mesmo caminho e as mesmas variáveis da
    /// partida): uma aprovação depois de trocar `aparencia.skin` já vale, e
    /// o cérebro passa a usar `sessoes.origens` e `celebracao.modo` do
    /// arquivo novo, como o `/v1/estado.config` mostra (decisão 0030).
    fn reler_config(&mut self) {
        let Some(arquivo) = &self.arquivo_config else {
            return;
        };
        let config = crate::daemon::carregar_config(arquivo);
        let skin = config.texto("aparencia.skin").to_owned();
        if skin != self.configurada {
            info!(
                "config: aparencia.skin mudou de «{}» para «{skin}»",
                self.configurada
            );
            self.configurada = skin;
        }
        let cerebro = ConfigCerebro::de(&config.config());
        self.comp.definir_config(config);
        if &cerebro != self.cerebro.config() {
            info!(
                "config: o cérebro passa a acompanhar as origens {} (celebração {:?})",
                cerebro.origens.join(", "),
                cerebro.modo
            );
            self.cerebro.reconfigurar(cerebro);
            self.depois_do_cerebro(Vec::new());
        }
    }

    /// Escolhe o personagem (decisões 0011 e 0026), publica no `/v1/estado`
    /// e, com o compositor conectado, troca na tela se mudou.
    pub fn escolher_personagem(&mut self) {
        let Escolha {
            skin,
            pedida,
            origem,
            sha256,
            avisos,
        } = personagem::escolher(&self.onde, &self.configurada);
        for aviso in &avisos {
            aviso!("personagem: {aviso}");
        }
        match &skin {
            Some(skin) => info!("personagem: skin «{}» ({})", skin.id, skin.nome),
            None => info!("sem personagem (pedida: «{pedida}»): o pet fica escondido"),
        }
        let na_tela = skin
            .as_ref()
            .map(|s| (s.id.clone(), sha256.clone(), origem));
        self.comp.definir_skin(InfoSkin {
            id: skin.as_ref().map(|s| s.id.clone()),
            pedida,
            origem,
            sha256,
            avisos,
        });
        if mesma_tela(self.skin.is_some(), &self.na_tela, &na_tela) {
            // A mesma skin, com a mesma impressão e da mesma origem: a tela
            // fica como está (sem recomeçar a animação).
            return;
        }
        self.na_tela = na_tela;
        let skin = skin.map(Rc::new);
        self.skin = skin.clone();
        if self.viva.is_some() {
            self.comp.definir_tela(if self.skin.is_some() {
                Tela::Ativa
            } else {
                Tela::SemPersonagem
            });
            let visivel = self.visivel;
            if let Some(sessao) = self.sessao_mut() {
                sessao.trocar_skin(skin, visivel);
            }
        }
    }

    /// O relógio do cérebro: parede (o mesmo do `ts` dos hooks) e
    /// monotônico desde a partida.
    fn agora(&self) -> Agora {
        Agora {
            parede_ms: agora_desde_1970_ms(),
            mono_ms: self.inicio.elapsed().as_millis() as u64,
        }
    }

    /// Publica o estado inicial do cérebro (sem sessões).
    pub fn iniciar_cerebro(&mut self) {
        info!(
            "cérebro: sessões de origem {}",
            self.cerebro.config().origens.join(", ")
        );
        self.depois_do_cerebro(Vec::new());
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

    /// Comandos das outras threads (eventos, `/v1/comando` com reações e
    /// aprovações, debug).
    pub fn instalar_comandos(&mut self, canal: Channel<Comando>) -> Result<(), String> {
        self.handle
            .insert_source(canal, |evento, _, laco| {
                if let channel::Event::Msg(comando) = evento {
                    laco.comando(comando);
                }
            })
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn comando(&mut self, comando: Comando) {
        match comando {
            Comando::Evento(recebido) => self.evento(*recebido),
            Comando::Tocar(reacao) => match self.sessao_mut().map(|s| s.tocar(&reacao)) {
                Some(true) => info!("tocar: «{reacao}»"),
                Some(false) => aviso!("tocar: sem personagem, ou a skin não tem «{reacao}»"),
                None => aviso!("tocar «{reacao}» sem compositor"),
            },
            Comando::Esconder | Comando::Mostrar => {
                self.visivel = matches!(comando, Comando::Mostrar);
                info!("{}", if self.visivel { "mostrar" } else { "esconder" });
                let visivel = self.visivel;
                if let Some(sessao) = self.sessao_mut() {
                    sessao.definir_visivel(visivel);
                }
            }
            Comando::Estresse { fps, segundos } => match self.sessao_mut() {
                Some(sessao) => sessao.estresse(fps, segundos),
                None => aviso!("debug: estresse pedido sem compositor"),
            },
            Comando::Quadro(resposta) => {
                let quadro = self.sessao_mut().and_then(|s| s.quadro_esperado());
                let _ = resposta.try_send(quadro);
            }
            Comando::RecarregarPersonagem(feito) => {
                info!("aprovação mudou: escolhendo o personagem de novo");
                self.reler_config();
                self.escolher_personagem();
                let _ = feito.try_send(());
            }
        }
    }

    /// Um evento do Claude Code vai para o cérebro, no relógio da chegada
    /// ([`relogio_da_chegada`]). No log só vão o nome do evento e o começo do
    /// id da sessão (decisão 0019).
    fn evento(&mut self, recebido: Recebido) {
        let ev = &recebido.evento;
        depurar!(
            "evento {} da sessão {}",
            ev.e,
            ev.sid.as_deref().map_or_else(|| "?".into(), evento::curto)
        );
        let agora = relogio_da_chegada(self.inicio, &recebido);
        let reacoes = self.cerebro.receber(ev, recebido.recebido_ms, agora);
        self.depois_do_cerebro(reacoes);
    }

    /// Toca as reações, publica o cérebro e rearma o prazo dele.
    fn depois_do_cerebro(&mut self, reacoes: Vec<Reacao>) {
        for reacao in &reacoes {
            self.reagir(reacao);
        }
        self.comp.publicar_cerebro(&self.cerebro.resumo());
        self.armar_prazo_cerebro();
    }

    fn reagir(&mut self, reacao: &Reacao) {
        info!(
            "reação {}{} da sessão {}{}",
            reacao.nome,
            reacao
                .nivel
                .map_or_else(String::new, |n| format!(" ({n:?})")),
            reacao.sid8,
            if reacao.teste { " (teste)" } else { "" }
        );
        match self.sessao_mut().map(|s| s.tocar(reacao.nome)) {
            Some(true) => {}
            Some(false) => depurar!("reação {} sem animação na tela", reacao.nome),
            None => depurar!("reação {} sem compositor", reacao.nome),
        }
    }

    fn armar_prazo_cerebro(&mut self) {
        if let Some(token) = self.prazo_cerebro.take() {
            self.handle.remove(token);
        }
        let Some(prazo) = self.cerebro.proximo_prazo() else {
            return;
        };
        let quando = self.inicio + Duration::from_millis(prazo);
        let inserido = self
            .handle
            .insert_source(Timer::from_deadline(quando), |_, _, laco| {
                // Este timer acaba aqui; o próximo é armado abaixo.
                laco.prazo_cerebro = None;
                let agora = laco.agora();
                let reacoes = laco.cerebro.tique(agora);
                laco.depois_do_cerebro(reacoes);
                TimeoutAction::Drop
            });
        match inserido {
            Ok(token) => self.prazo_cerebro = Some(token),
            Err(e) => erro!("não consegui armar o prazo do cérebro: {e}"),
        }
    }

    /// (Re)arma o batimento de 5 s. Chamado na partida e depois de um erro
    /// no `dispatch`, porque o calloop tira os timers vencidos da fila antes
    /// de despachar: se outra fonte falha no mesmo ciclo, o evento do timer
    /// se perde. Rearmar **não** bate: só o timer disparando prova que o
    /// laço anda. Um `dispatch` que falhasse sempre rearmaria sem nunca
    /// bater, e o vigia abortaria em 60 s (o Docker reinicia).
    pub fn armar_batimento(&mut self) {
        if let Some(token) = self.batimento.take() {
            self.handle.remove(token);
        }
        let timer = Timer::from_duration(BATIMENTO);
        match self.handle.insert_source(timer, |_, _, laco| {
            laco.comp.bater();
            if let Some(sessao) = laco.sessao_mut() {
                sessao.publicar();
            }
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
        let agora = Instant::now();
        let reconexao = &self.reconexao;
        let em_espera = |assinatura: &str| {
            (!reconexao.pode_tentar(assinatura, agora))
                .then(|| reconexao.espera_restante(assinatura, agora))
        };
        let instancia = match descoberta::procurar(&self.base, &self.curto, em_espera) {
            Ok(instancia) => instancia,
            Err(espera) => {
                self.registrar_espera(espera.descrever());
                return Some(match espera {
                    Espera::Recuo { restante } => {
                        restante.clamp(Duration::from_millis(100), INTERVALO_DESCOBERTA)
                    }
                    _ => INTERVALO_DESCOBERTA,
                });
            }
        };
        let assinatura = instancia.assinatura;
        let nome_wayland = instancia.nome_wayland;
        let conexao = match wl::conectar(
            instancia.wayland,
            self.handle.clone(),
            self.skin.clone(),
            Arc::clone(&self.comp),
        ) {
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
        self.comp.definir_tela(if self.skin.is_some() {
            Tela::Ativa
        } else {
            Tela::SemPersonagem
        });
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

    /// Fim do processo: esconde o pet (quadro transparente), destrói a
    /// camada e espera, com prazo, o compositor confirmar que processou tudo,
    /// para o fade de saída do Hyprland sair vazio.
    pub fn encerrar(&mut self) {
        if let Some(sessao) = self.sessao_mut() {
            sessao.encerrar();
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
        self.comp.publicar_painel(Painel::default());
        self.armar_descoberta(Duration::ZERO);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cerebro_conta_os_prazos_da_chegada_do_evento() {
        let inicio = Instant::now();
        let recebido = |chegada: Instant| Recebido {
            evento: pet_core::evento::Evento {
                e: "Stop".into(),
                ..Default::default()
            },
            recebido_ms: 1_790_000_000_000,
            chegada,
        };
        let agora = relogio_da_chegada(inicio, &recebido(inicio + Duration::from_millis(1_234)));
        assert_eq!(
            agora,
            Agora {
                parede_ms: 1_790_000_000_000,
                mono_ms: 1_234
            },
            "a hora da chegada, mesmo processado bem depois"
        );
        // Chegou antes de o laço existir (o ingress sobe primeiro): zero.
        let cedo = inicio
            .checked_sub(Duration::from_millis(5))
            .unwrap_or(inicio);
        assert_eq!(relogio_da_chegada(inicio, &recebido(cedo)).mono_ms, 0);
    }

    #[test]
    fn so_troca_a_tela_quando_muda_quem_esta_nela() {
        let zeca =
            |sha: &str, origem| Some(("zeca".to_owned(), Some(sha.to_owned()), Some(origem)));
        let a = zeca("aa", Origem::Imagem);
        assert!(mesma_tela(true, &a, &a), "aprovar de novo a mesma skin");
        assert!(
            !mesma_tela(true, &a, &zeca("bb", Origem::Imagem)),
            "skin nova"
        );
        assert!(
            !mesma_tela(true, &a, &zeca("aa", Origem::Snapshot)),
            "a da imagem quebrou: vai a cópia de /state"
        );
        assert!(!mesma_tela(true, &a, &None), "revogou: esconde");
        assert!(
            !mesma_tela(false, &None, &None),
            "nada antes, nada agora: publica de qualquer jeito"
        );
        assert!(
            !mesma_tela(false, &a, &a),
            "sem skin carregada ainda: troca"
        );
    }
}
