//! O núcleo do daemon, igual em todo sistema (decisão 0040).
//!
//! Junta o [`Motor`] (puro, no `pet-core`) com o que é do daemon: o
//! personagem escolhido pelas aprovações em disco, o config relido a cada
//! aprovação e o estado compartilhado com a entrada HTTP (`/v1/estado`). O
//! laço de cada sistema (no Linux, [`crate::laco`]) só traz os comandos da
//! caixa, a conexão com o sistema ([`Punho`]: a janela e o desktop juntos,
//! decisão 0043), os eventos do desktop e os prazos, e chama este núcleo.
//!
//! Funciona com ou sem janela: sem compositor, as reações só ficam no
//! `/v1/estado`.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use pet_core::cerebro::{Agora, ConfigCerebro, Reacao};
use pet_core::config::ConfigEfetiva;
use pet_core::evento;
use pet_core::motor::{Clicou, Motor, Posicoes, Tocou};
use pet_core::plataforma::{EventoDesktop, EventoOverlay, EventoPonteiro, Overlay, Punho};

use crate::comando::{Comando, Recebido};
use crate::estado::{Compartilhado, InfoSkin, Painel, Tela};
use crate::ingress::agora_desde_1970_ms;
use crate::personagem::{self, Escolha, NaTela, Onde, mesma_tela};

/// Quantas vezes por lote o núcleo esvazia os eventos da janela (um evento
/// pode gerar outro; uma janela com defeito não prende o laço).
const VOLTAS_DE_EVENTOS: usize = 8;
/// As posições do pet por monitor, em `/state` (decisão 0049).
pub const ARQUIVO_POSICOES: &str = "posicoes.json";

/// Lê as posições salvas; sem arquivo, ou com um que não serve, o pet
/// começa no canto padrão de cada monitor.
fn ler_posicoes(estado: &Path) -> Posicoes {
    let caminho = estado.join(ARQUIVO_POSICOES);
    match std::fs::read_to_string(&caminho) {
        Ok(texto) => match Posicoes::ler(&texto) {
            Ok(posicoes) => {
                info!("posições salvas: {} monitor(es)", posicoes.quantas());
                posicoes
            }
            Err(motivo) => {
                aviso!(
                    "posições salvas ignoradas ({}): {motivo}",
                    caminho.display()
                );
                Posicoes::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Posicoes::default(),
        Err(e) => {
            aviso!("não consegui ler {}: {e}", caminho.display());
            Posicoes::default()
        }
    }
}

/// Grava as posições de uma vez (arquivo temporário e `rename`).
fn gravar_posicoes(estado: &Path, json: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(estado)?;
    let temporario = estado.join(format!("{ARQUIVO_POSICOES}.tmp"));
    std::fs::write(&temporario, json)?;
    std::fs::rename(&temporario, estado.join(ARQUIVO_POSICOES))
}

/// Reempresta a conexão por um trecho. O `as_deref_mut` não serve: ele não
/// encurta o tempo de vida do `dyn Punho` dentro do `Option`.
fn punho<'a>(p: &'a mut Option<&mut dyn Punho>) -> Option<&'a mut dyn Punho> {
    match p {
        Some(p) => Some(&mut **p),
        None => None,
    }
}

/// A janela da conexão, por um trecho.
fn janela<'a>(p: &'a mut Option<&mut dyn Punho>) -> Option<&'a mut dyn Overlay> {
    match p {
        Some(p) => Some(p.janela()),
        None => None,
    }
}

pub struct Nucleo {
    pub comp: Arc<Compartilhado>,
    motor: Motor,
    /// Quem está na tela: (id, impressão digital, origem).
    na_tela: NaTela,
    /// Onde procurar skins e aprovações, para escolher de novo.
    onde: Onde,
    /// A skin configurada (`aparencia.skin`).
    configurada: String,
    /// A pasta do `bichinho.toml`, relido a cada aprovação: trocar
    /// `aparencia.skin` (o `zeca-contorno`, por exemplo) vale sem reiniciar o
    /// container.
    pasta_config: Option<PathBuf>,
    /// Origem do relógio monotônico do laço (os prazos em ms).
    inicio: Instant,
}

impl Nucleo {
    /// `config` é a da partida: dela saem a skin configurada e a config do
    /// cérebro; `pasta_config` é de onde ela é relida a cada aprovação.
    pub fn novo(
        comp: Arc<Compartilhado>,
        onde: Onde,
        pasta_config: Option<PathBuf>,
        config: &ConfigEfetiva,
        inicio: Instant,
    ) -> Nucleo {
        let mut motor = Motor::novo(ConfigCerebro::de(&config.config()));
        // O sorteio das micro-ações do pet (decisão 0082): a hora da partida
        // e o processo, para cada partida variar; o núcleo puro não lê o
        // relógio.
        motor.semear(agora_desde_1970_ms() ^ (u64::from(std::process::id()) << 32));
        motor.definir_tamanho(config.config().tamanho, None, 0);
        motor.definir_posicoes(ler_posicoes(&onde.estado));
        let mut nucleo = Nucleo {
            comp,
            motor,
            na_tela: None,
            onde,
            configurada: config.texto("aparencia.skin").to_owned(),
            pasta_config,
            inicio,
        };
        nucleo.escolher_personagem(None);
        nucleo
    }

    /// Origem do relógio do laço (os prazos em ms).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn inicio(&self) -> Instant {
        self.inicio
    }

    /// Milissegundos no relógio monotônico do laço.
    pub fn agora_ms(&self) -> u64 {
        self.inicio.elapsed().as_millis() as u64
    }

    /// O relógio do cérebro: parede (o mesmo do `ts` dos hooks) e
    /// monotônico desde a partida.
    fn agora(&self) -> Agora {
        Agora {
            parede_ms: agora_desde_1970_ms(),
            mono_ms: self.agora_ms(),
        }
    }

    /// Relê o `bichinho.toml` (a mesma pasta e as mesmas variáveis da
    /// partida): uma aprovação depois de trocar `aparencia.skin` já vale, e
    /// o cérebro passa a usar `sessoes.origens` e `celebracao.modo` do
    /// arquivo novo, como o `/v1/estado.config` mostra (decisão 0030).
    ///
    /// O `aparencia.tamanho` novo só é anotado: quem redesenha é a escolha
    /// do personagem que vem logo depois (uma skin nova já nasce no tamanho
    /// novo; uma revogada sai sem nenhum quadro nele), ou o
    /// [`Motor::redesenhar_palco`] se a skin ficou (decisão 0046). Devolve se
    /// o tamanho mudou.
    fn reler_config(&mut self, mut ov: Option<&mut dyn Punho>) -> bool {
        let Some(pasta) = &self.pasta_config else {
            return false;
        };
        let config = crate::daemon::carregar_config(&crate::ambiente::arquivo_config_em(pasta));
        let skin = config.texto("aparencia.skin").to_owned();
        if skin != self.configurada {
            info!(
                "config: aparencia.skin mudou de «{}» para «{skin}»",
                self.configurada
            );
            self.configurada = skin;
        }
        let cerebro = ConfigCerebro::de(&config.config());
        let tamanho = config.config().tamanho;
        self.comp.definir_config(config);
        let mudou_tamanho = tamanho != self.motor.tamanho();
        if mudou_tamanho {
            info!(
                "config: aparencia.tamanho mudou de «{}» para «{}»",
                self.motor.tamanho().nome(),
                tamanho.nome()
            );
            let agora = self.agora_ms();
            self.motor.definir_tamanho(tamanho, None, agora);
        }
        if &cerebro != self.motor.config_do_cerebro() {
            info!(
                "config: o cérebro passa a acompanhar as origens {} (celebração {:?})",
                cerebro.origens.join(", "),
                cerebro.modo
            );
            self.motor.reconfigurar_cerebro(cerebro);
            self.depois_do_cerebro(Vec::new(), punho(&mut ov));
        }
        mudou_tamanho
    }

    /// Escolhe o personagem (decisões 0011 e 0026), publica no `/v1/estado`
    /// e, com a janela aberta, troca na tela se mudou. Devolve se trocou
    /// (`false`: o mesmo personagem continua na tela, sem recomeçar).
    pub fn escolher_personagem(&mut self, ov: Option<&mut dyn Punho>) -> bool {
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
        if mesma_tela(self.motor.skin().is_some(), &self.na_tela, &na_tela) {
            // A mesma skin, com a mesma impressão e da mesma origem: a tela
            // fica como está (sem recomeçar a animação).
            return false;
        }
        self.na_tela = na_tela;
        let skin = skin.map(Rc::new);
        match ov {
            Some(ov) => {
                self.comp.definir_tela(if skin.is_some() {
                    Tela::Ativa
                } else {
                    Tela::SemPersonagem
                });
                let agora = self.agora_ms();
                self.motor.trocar_skin(ov.janela(), skin, agora);
            }
            None => self.motor.definir_skin(skin),
        }
        true
    }

    /// Publica o estado inicial do cérebro (sem sessões).
    pub fn iniciar_cerebro(&mut self) {
        info!(
            "cérebro: sessões de origem {}",
            self.motor.config_do_cerebro().origens.join(", ")
        );
        self.depois_do_cerebro(Vec::new(), None);
    }

    /// Um comando da caixa (eventos, `/v1/comando` com reações e
    /// aprovações, debug).
    pub fn comando(&mut self, comando: Comando, mut ov: Option<&mut dyn Punho>) {
        match comando {
            Comando::Evento(recebido) => self.evento(*recebido, ov),
            Comando::Tocar { reacao, resposta } => {
                let agora = self.agora_ms();
                let tocou = self.motor.tocar_comando(janela(&mut ov), &reacao, agora);
                match &tocou {
                    Tocou::NaTela { tag } => info!("tocar: «{reacao}» (tag {tag})"),
                    Tocou::ForaDaTela { tag, motivo } => {
                        aviso!("tocar: «{reacao}» (tag {tag}) fora da tela: {motivo}");
                    }
                    Tocou::SemPersonagem => aviso!("tocar: «{reacao}» sem personagem aprovado"),
                    Tocou::Desconhecida { skin } => {
                        aviso!("tocar: a skin «{skin}» não tem «{reacao}»");
                    }
                }
                let _ = resposta.try_send(tocou);
            }
            Comando::Clique { botao, resposta } => {
                let agora = self.agora_ms();
                let clicou = match punho(&mut ov) {
                    Some(p) => self.motor.clicar(p, botao, agora),
                    None => Clicou::Nada {
                        motivo: "sem compositor: o clique precisa da janela do pet",
                    },
                };
                self.publicar_avisos();
                let _ = resposta.try_send(clicou);
            }
            Comando::Esconder | Comando::Mostrar => {
                let visivel = matches!(comando, Comando::Mostrar);
                self.motor.definir_visivel(visivel);
                info!("{}", if visivel { "mostrar" } else { "esconder" });
                if let Some(ov) = janela(&mut ov) {
                    let agora = self.agora_ms();
                    self.motor.aplicar_visibilidade(ov, agora);
                }
            }
            Comando::Estresse { fps, segundos } => match janela(&mut ov) {
                Some(ov) => {
                    let agora = self.agora_ms();
                    self.motor.estresse(ov, fps, segundos, agora);
                }
                None => aviso!("debug: estresse pedido sem compositor"),
            },
            Comando::Quadro(resposta) => {
                let quadro = ov
                    .as_deref()
                    .and_then(|p| self.motor.quadro_esperado(p.ver_janela()));
                let _ = resposta.try_send(quadro);
            }
            Comando::RecarregarPersonagem(feito) => {
                info!("aprovação mudou: escolhendo o personagem de novo");
                let mudou_tamanho = self.reler_config(punho(&mut ov));
                let trocou = self.escolher_personagem(punho(&mut ov));
                // A mesma skin no tamanho novo: um quadro só, já no palco
                // novo (uma skin trocada ou revogada já cuidou da tela).
                if mudou_tamanho
                    && !trocou
                    && let Some(ov) = janela(&mut ov)
                {
                    let agora = self.agora_ms();
                    self.motor.redesenhar_palco(ov, agora);
                }
                let _ = feito.try_send(());
            }
        }
    }

    /// Um evento do desktop (decisão 0043): o monitor em foco, a janela
    /// ativa, a presença. Devolve se o `/v1/estado` precisa ser publicado.
    pub fn evento_desktop(
        &mut self,
        evento: &EventoDesktop,
        mut ov: Option<&mut dyn Punho>,
    ) -> bool {
        let agora = self.agora();
        let mut mudou = self.motor.evento_desktop(janela(&mut ov), evento, agora);
        // A fonte das trocas voltou: o que mudou enquanto ela estava fora não
        // chegou, e o anel fica num buraco até a próxima troca. A conexão sabe
        // a janela ativa de agora (no Wayland, o foreign-toplevel): ela é a
        // semente (decisão 0061).
        if *evento == EventoDesktop::Ligado(true)
            && let Some(ativa) = ov.as_deref().and_then(|p| p.ver_desktop().janela_ativa())
        {
            let semente = EventoDesktop::JanelaInicial {
                janela: ativa,
                parede_ms: agora.parede_ms,
            };
            mudou |= self.motor.evento_desktop(janela(&mut ov), &semente, agora);
        }
        // Uma janela que fechou tira a janela das sessões dela.
        if matches!(evento, EventoDesktop::JanelaFechou(_)) {
            self.motor.tirar_mudanca_do_cerebro();
            self.comp.publicar_cerebro(&self.motor.resumo());
        }
        // A janela que o clique focou ficou ativa: o aviso saiu.
        self.publicar_avisos() || mudou
    }

    /// Um aviso saiu fora de um evento do Claude (o clique, o terminal em
    /// foco; decisão 0057): o `/v1/estado.sessoes` sai de novo. Devolve se
    /// publicou.
    fn publicar_avisos(&mut self) -> bool {
        let mudou = self.motor.tirar_mudanca_do_cerebro();
        if mudou {
            self.comp.publicar_cerebro(&self.motor.resumo());
        }
        mudou
    }

    /// Um evento do Claude Code vai para o cérebro, no relógio da chegada.
    /// No log só vão o nome do evento e o começo do id da sessão (decisão
    /// 0019).
    fn evento(&mut self, recebido: Recebido, ov: Option<&mut dyn Punho>) {
        let ev = &recebido.evento;
        depurar!(
            "evento {} da sessão {}",
            ev.e,
            ev.sid.as_deref().map_or_else(|| "?".into(), evento::curto)
        );
        let agora = recebido.agora(self.inicio);
        let reacoes = self.motor.evento(ev, recebido.recebido_ms, agora);
        self.depois_do_cerebro(reacoes, ov);
    }

    /// Toca as reações e publica o cérebro.
    fn depois_do_cerebro(&mut self, reacoes: Vec<Reacao>, mut ov: Option<&mut dyn Punho>) {
        for reacao in &reacoes {
            self.reagir(reacao, punho(&mut ov));
        }
        self.motor.tirar_mudanca_do_cerebro();
        self.comp.publicar_cerebro(&self.motor.resumo());
    }

    fn reagir(&mut self, reacao: &Reacao, mut ov: Option<&mut dyn Punho>) {
        info!(
            "reação {}{} da sessão {}{}",
            reacao.nome,
            reacao
                .nivel
                .map_or_else(String::new, |n| format!(" ({n:?})")),
            reacao.sid8,
            if reacao.teste { " (teste)" } else { "" }
        );
        let com_janela = ov.is_some();
        let agora = self.agora_ms();
        if !self.motor.reagir(janela(&mut ov), reacao.nome, agora) {
            if com_janela {
                depurar!("reação {} sem animação na tela", reacao.nome);
            } else {
                depurar!("reação {} sem compositor", reacao.nome);
            }
        }
    }

    /// O próximo prazo, em ms no relógio do laço: o do Motor (cérebro e
    /// animação) ou o da janela.
    pub fn proximo_prazo(&self, ov: Option<&dyn Punho>) -> Option<u64> {
        [
            self.motor.proximo_prazo(),
            ov.and_then(|p| p.ver_janela().proximo_prazo()),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Venceu um prazo: os da janela, o do cérebro e o da animação, nessa
    /// ordem. Quem chama esvazia a caixa antes (decisão 0032). Sem conexão,
    /// os prazos do Motor vencem do mesmo jeito, sem desenhar: um prazo
    /// vencido que ficasse armado faria o laço girar (decisão 0059).
    pub fn vencer(&mut self, mut ov: Option<&mut dyn Punho>) {
        let agora = self.agora_ms();
        self.motor.acertar_relogio(self.agora());
        if let Some(ov) = janela(&mut ov) {
            ov.vencer(agora);
        }
        self.eventos_da_janela(punho(&mut ov));
        if self.motor.prazo_do_cerebro().is_some_and(|p| p <= agora) {
            let reacoes = self.motor.tique(self.agora());
            self.depois_do_cerebro(reacoes, punho(&mut ov));
        }
        match punho(&mut ov) {
            Some(ov) => self.motor.vencer(ov, agora),
            None => self.motor.vencer_sem_conexao(agora),
        }
        self.publicar_avisos();
        self.guardar_posicoes();
    }

    /// Grava as posições do pet em `/state` se elas mudaram (um arraste
    /// terminou).
    fn guardar_posicoes(&mut self) {
        if let Some(json) = self.motor.posicoes_para_gravar() {
            match gravar_posicoes(&self.onde.estado, &json) {
                Ok(()) => depurar!("posição do pet guardada"),
                Err(e) => aviso!("não consegui guardar a posição do pet: {e}"),
            }
        }
    }

    /// Os eventos da janela e do desktop da conexão, em ordem, até acabarem.
    /// Devolve se algum deles mudou o pet, a janela ou o desktop (o ponteiro
    /// sozinho não muda o painel): só aí o laço precisa publicar.
    pub fn eventos_da_janela(&mut self, ov: Option<&mut dyn Punho>) -> bool {
        let Some(ov) = ov else {
            return false;
        };
        self.motor.acertar_relogio(self.agora());
        let mut mudou = false;
        for _ in 0..VOLTAS_DE_EVENTOS {
            let eventos = ov.janela().eventos();
            let do_desktop = ov.desktop().eventos();
            if eventos.is_empty() && do_desktop.is_empty() {
                break;
            }
            let agora = self.agora_ms();
            for evento in eventos {
                // Mover o ponteiro não muda o painel; apertar e soltar (o
                // clique, o fim do arraste) mudam.
                mudou |= !matches!(
                    evento,
                    EventoOverlay::Ponteiro(
                        EventoPonteiro::Entrou { .. }
                            | EventoPonteiro::Moveu { .. }
                            | EventoPonteiro::Saiu
                    )
                );
                self.motor.evento_overlay(ov, evento, agora);
            }
            for evento in &do_desktop {
                mudou |= self.evento_desktop(evento, Some(&mut *ov));
            }
        }
        // O clique viu um aviso (a janela já estava ativa).
        mudou |= self.publicar_avisos();
        self.guardar_posicoes();
        mudou
    }

    /// Uma conexão nova: o compositor conectou. (Só o laço com janela usa: o
    /// do Linux; Windows e macOS no M8.)
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn conectou(&mut self, ov: &mut dyn Punho) {
        self.comp.definir_tela(if self.motor.skin().is_some() {
            Tela::Ativa
        } else {
            Tela::SemPersonagem
        });
        let agora = self.agora_ms();
        self.motor.conectou(agora);
        self.motor.aplicar_visibilidade(ov.janela(), agora);
    }

    /// A janela acabou com a sessão: volta a esperar o compositor.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn desconectou(&mut self) {
        self.motor.desconectou();
        self.comp.definir_tela(Tela::Aguardando);
        self.comp.publicar_painel(Painel::default());
    }

    /// Publica o painel no `/v1/estado`.
    pub fn publicar(&mut self, ov: Option<&dyn Punho>) {
        let agora = self.agora_ms();
        let painel = self.motor.painel(ov, agora);
        self.comp.publicar_painel(painel);
    }

    /// Fim do processo: esconde o pet (quadro transparente), larga a janela e
    /// espera, com prazo, o sistema confirmar que processou tudo, para o fade
    /// de saída do compositor sair vazio.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn encerrar(&mut self, ov: &mut dyn Punho) {
        let agora = self.agora_ms();
        self.motor.encerrar(ov.janela(), agora);
    }
}

#[cfg(test)]
mod testes {
    use std::fs;
    use std::sync::mpsc;

    use pet_core::plataforma::Monitor;
    use pet_core::plataforma::falsa::JanelaFalsa;

    use super::*;
    use crate::aprovacao::{self, testes::Ambiente};
    use crate::personagem::Onde;

    fn edp() -> Monitor {
        Monitor {
            nome: Some("eDP-1".into()),
            logico: (1280, 800),
            escala: 1.5,
            ..Monitor::default()
        }
    }

    /// Um núcleo com o «zeca» (a xadrez com outro id) aprovado, na janela de
    /// mentira já pronta no eDP-1, com o primeiro quadro desenhado.
    fn ligado(a: &Ambiente, config: &std::path::Path) -> (Nucleo, JanelaFalsa) {
        aprovacao::aprovar("zeca", &a.sha(), &a.busca(), &a.estado).unwrap();
        let efetiva = crate::daemon::carregar_config(&crate::ambiente::arquivo_config_em(config));
        let comp = Arc::new(Compartilhado::novo(efetiva.clone(), false));
        let onde = Onde {
            busca: a.busca(),
            estado: a.estado.clone(),
            debug: false,
            debug_personagem: false,
        };
        let mut nucleo = Nucleo::novo(
            comp,
            onde,
            Some(config.to_owned()),
            &efetiva,
            Instant::now(),
        );
        let mut janela = JanelaFalsa::default();
        nucleo.conectou(&mut janela);
        janela.pronta = Some(edp());
        janela.eventos.push(EventoOverlay::Pronta);
        assert!(nucleo.eventos_da_janela(Some(&mut janela)));
        assert_eq!(janela.pedidos, vec!["criar", "quadro 1"]);
        (nucleo, janela)
    }

    fn recarregar(nucleo: &mut Nucleo, janela: &mut JanelaFalsa) -> Vec<String> {
        let antes = janela.pedidos.len();
        let (feito, recebeu) = mpsc::sync_channel(1);
        nucleo.comando(Comando::RecarregarPersonagem(feito), Some(janela));
        assert_eq!(recebeu.try_recv(), Ok(()));
        janela.pedidos[antes..].to_vec()
    }

    #[test]
    fn eventos_do_desktop_da_conexao_chegam_ao_painel() {
        use pet_core::plataforma::Alca;
        let a = Ambiente::novo("nucleo-desktop");
        let config = a.raiz.join("config");
        fs::create_dir_all(&config).unwrap();
        let (mut nucleo, mut janela) = ligado(&a, &config);
        janela.desktop.janelas = vec![Alca("abc123".into())];
        janela.desktop.eventos.push(EventoDesktop::JanelaInicial {
            janela: Alca("abc123".into()),
            parede_ms: 1,
        });
        assert!(
            nucleo.eventos_da_janela(Some(&mut janela)),
            "o desktop mudou"
        );
        assert!(!nucleo.eventos_da_janela(Some(&mut janela)), "nada novo");
        nucleo.publicar(Some(&janela));
        let estado = nucleo.comp.estado_json();
        assert_eq!(estado["desktop"]["janela_ativa"], "abc123");
        assert_eq!(estado["desktop"]["foca_janelas"], true);
        assert_eq!(estado["desktop"]["janelas"], 1);
        // Um evento do desktop de fora da conexão (o socket2, no Hyprland).
        assert!(nucleo.evento_desktop(&EventoDesktop::Ligado(true), Some(&mut janela)));
        nucleo.publicar(Some(&janela));
        assert_eq!(nucleo.comp.estado_json()["desktop"]["eventos"], "ligado");
    }

    #[test]
    fn a_fonte_das_trocas_que_volta_ressemeia_o_anel_pela_conexao() {
        use pet_core::plataforma::Alca;
        let a = Ambiente::novo("nucleo-semente");
        let config = a.raiz.join("config");
        fs::create_dir_all(&config).unwrap();
        let (mut nucleo, mut janela) = ligado(&a, &config);
        nucleo.evento_desktop(&EventoDesktop::Ligado(true), Some(&mut janela));
        let ativa = EventoDesktop::JanelaAtiva {
            janela: Some(Alca("f00d01".into())),
            parede_ms: 1,
        };
        nucleo.evento_desktop(&ativa, Some(&mut janela));
        // O socket2 cai: o anel ganha um buraco (o que vier no meio não chega).
        nucleo.evento_desktop(&EventoDesktop::Ligado(false), Some(&mut janela));
        nucleo.publicar(Some(&janela));
        assert_eq!(
            nucleo.comp.estado_json()["desktop"]["anel"][0]["tipo"],
            "buraco"
        );
        // Ele volta, e o foreign-toplevel da conexão diz que a ativa agora é o
        // foot2: ela é a semente, sem esperar o Renan trocar de janela.
        janela.desktop.ativa = Some(Alca("f00d02".into()));
        assert!(nucleo.evento_desktop(&EventoDesktop::Ligado(true), Some(&mut janela)));
        nucleo.publicar(Some(&janela));
        let estado = nucleo.comp.estado_json();
        let anel = &estado["desktop"]["anel"];
        assert_eq!(
            (&anel[0]["tipo"], &anel[0]["janela"]),
            (&"semente".into(), &"f00d02".into())
        );
        assert_eq!(estado["desktop"]["janela_ativa"], "f00d02");
    }

    #[test]
    fn o_clique_foca_o_terminal_e_o_aviso_visto_sai_do_estado() {
        use pet_core::evento::Evento;
        use pet_core::plataforma::{Alca, Botao};
        let a = Ambiente::novo("nucleo-clique");
        let config = a.raiz.join("config");
        fs::create_dir_all(&config).unwrap();
        let (mut nucleo, mut janela) = ligado(&a, &config);
        let clicar = |nucleo: &mut Nucleo, janela: Option<&mut JanelaFalsa>| {
            let (resposta, recebeu) = mpsc::sync_channel(1);
            nucleo.comando(
                Comando::Clique {
                    botao: Botao::Esquerdo,
                    resposta,
                },
                janela.map(|j| j as &mut dyn Punho),
            );
            recebeu.try_recv().expect("o clique responde na hora")
        };
        // Sem compositor, o clique não faz nada.
        assert!(matches!(clicar(&mut nucleo, None), Clicou::Nada { .. }));
        // O socket2 conta o foot ativo; a sessão pede permissão nele.
        let agora = agora_desde_1970_ms();
        let ativa = |janela: &str, parede_ms: u64| EventoDesktop::JanelaAtiva {
            janela: Some(Alca(janela.into())),
            parede_ms,
        };
        nucleo.evento_desktop(&EventoDesktop::Ligado(true), Some(&mut janela));
        nucleo.evento_desktop(&ativa("f00d01", agora - 5_000), Some(&mut janela));
        for (e, ts) in [
            ("UserPromptSubmit", agora - 4_000),
            ("PermissionRequest", agora - 3_000),
        ] {
            let evento = Evento {
                e: e.into(),
                sid: Some("0123456789abcdef".into()),
                turno: Some("p1".into()),
                ent: Some("cli".into()),
                proj: Some("api".into()),
                tool: Some("Bash".into()),
                ts: Some(ts),
                ..Evento::default()
            };
            let recebido = Recebido {
                evento,
                recebido_ms: ts,
                chegada: Instant::now(),
            };
            nucleo.comando(Comando::Evento(Box::new(recebido)), Some(&mut janela));
        }
        let estado = nucleo.comp.estado_json();
        assert_eq!(estado["sessoes"][0]["aviso"]["tipo"], "esperando");
        assert_eq!(estado["sessoes"][0]["janela"]["endereco"], "f00d01");
        // Outra janela ativa: o clique foca o foot e espera a confirmação.
        nucleo.evento_desktop(&ativa("f00d02", agora - 1_000), Some(&mut janela));
        janela.desktop.janelas = vec![Alca("f00d01".into())];
        janela.mostrou();
        assert!(matches!(
            clicar(&mut nucleo, Some(&mut janela)),
            Clicou::Focou {
                confirmado: false,
                ..
            }
        ));
        assert_eq!(janela.desktop.focos, vec![Alca("f00d01".into())]);
        nucleo.publicar(Some(&janela));
        assert_eq!(nucleo.comp.estado_json()["focando"], "f00d01");
        // O socket2 conta a troca: o aviso sai do /v1/estado na hora.
        assert!(nucleo.evento_desktop(&ativa("f00d01", agora), Some(&mut janela)));
        let estado = nucleo.comp.estado_json();
        assert!(
            estado["sessoes"][0]["aviso"].is_null(),
            "{}",
            estado["sessoes"]
        );
        // Sem aviso, o clique mostra as sessões.
        assert_eq!(
            clicar(&mut nucleo, Some(&mut janela)),
            Clicou::Lista { sessoes: 1 }
        );
    }

    #[test]
    fn a_posicao_arrastada_fica_em_state_e_volta_noutro_nucleo() {
        use pet_core::plataforma::{Botao, EventoPonteiro};
        let a = Ambiente::novo("nucleo-posicao");
        let config = a.raiz.join("config");
        fs::create_dir_all(&config).unwrap();
        let (mut nucleo, mut janela) = ligado(&a, &config);
        let corpo = janela.toque.expect("toque no corpo");
        let (x, y) = (corpo.x + corpo.w / 2, corpo.y + corpo.h / 2);
        let antes = nucleo.motor.painel(Some(&janela), 0).sprite_disp.unwrap();
        for evento in [
            EventoPonteiro::Apertou {
                botao: Botao::Esquerdo,
                x,
                y,
            },
            EventoPonteiro::Moveu {
                x: x - 400,
                y: y - 300,
            },
            EventoPonteiro::Soltou {
                botao: Botao::Esquerdo,
                x: x - 400,
                y: y - 300,
            },
        ] {
            janela.mostrou();
            janela.eventos.push(EventoOverlay::Ponteiro(evento));
            nucleo.eventos_da_janela(Some(&mut janela));
        }
        let depois = nucleo.motor.painel(Some(&janela), 0).sprite_disp.unwrap();
        assert_ne!((depois.x, depois.y), (antes.x, antes.y), "arrastou");
        let arquivo = a.estado.join(ARQUIVO_POSICOES);
        let gravado = fs::read_to_string(&arquivo).expect("posições gravadas");
        assert!(gravado.contains("\"chave\": \"nome:eDP-1\""), "{gravado}");
        // Outro núcleo (o pet reiniciou) com o mesmo /state: o pet volta para
        // onde foi deixado, não para o canto.
        let (mut nucleo2, janela2) = ligado(&a, &config);
        let de_novo = nucleo2.motor.painel(Some(&janela2), 0).sprite_disp.unwrap();
        assert_eq!((de_novo.x, de_novo.y), (depois.x, depois.y));
        // Um arquivo quebrado não derruba nada: o canto padrão.
        fs::write(&arquivo, "quebrado").unwrap();
        let (mut nucleo3, janela3) = ligado(&a, &config);
        let padrao = nucleo3.motor.painel(Some(&janela3), 0).sprite_disp.unwrap();
        assert_eq!((padrao.x, padrao.y), (antes.x, antes.y));
    }

    #[test]
    fn revogar_com_tamanho_novo_so_esconde() {
        // O config pede outro tamanho e a aprovação some na mesma hora: o pet
        // sai com o quadro transparente, sem um quadro antes no tamanho novo
        // com a skin revogada.
        let a = Ambiente::novo("nucleo-revoga");
        let config = a.raiz.join("config");
        fs::create_dir_all(&config).unwrap();
        let (mut nucleo, mut janela) = ligado(&a, &config);
        fs::write(
            config.join("bichinho.toml"),
            "[aparencia]\ntamanho = \"pequeno\"\n",
        )
        .unwrap();
        aprovacao::revogar("zeca", &a.estado).unwrap();
        assert_eq!(
            recarregar(&mut nucleo, &mut janela),
            vec!["apagar com zeca"]
        );
        assert_eq!(nucleo.comp.tela(), Tela::SemPersonagem);
    }

    #[test]
    fn mesma_skin_com_tamanho_novo_redesenha_uma_vez() {
        let a = Ambiente::novo("nucleo-tamanho");
        let config = a.raiz.join("config");
        fs::create_dir_all(&config).unwrap();
        let (mut nucleo, mut janela) = ligado(&a, &config);
        let d_antes = nucleo.motor.painel(Some(&janela), 0).d;
        fs::write(
            config.join("bichinho.toml"),
            "[aparencia]\ntamanho = \"grande\"\n",
        )
        .unwrap();
        janela.em_voo = true; // um quadro em voo não segura a troca
        assert_eq!(
            recarregar(&mut nucleo, &mut janela),
            vec!["esquecer", "quadro 2"],
            "um quadro só, forçado, no palco novo"
        );
        let d_depois = nucleo.motor.painel(Some(&janela), 0).d;
        assert!(d_depois > d_antes, "{d_antes:?} → {d_depois:?}");
        // De novo, sem mudar nada: a tela fica como está.
        assert!(recarregar(&mut nucleo, &mut janela).is_empty());
    }
}
