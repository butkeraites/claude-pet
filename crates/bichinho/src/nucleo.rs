//! O núcleo do daemon, igual em todo sistema (decisão 0040).
//!
//! Junta o [`Motor`] (puro, no `pet-core`) com o que é do daemon: o
//! personagem escolhido pelas aprovações em disco, o config relido a cada
//! aprovação e o estado compartilhado com a entrada HTTP (`/v1/estado`). O
//! laço de cada sistema (no Linux, [`crate::laco`]) só traz os comandos da
//! caixa, a janela ([`Overlay`]) e os prazos, e chama este núcleo.
//!
//! Funciona com ou sem janela: sem compositor, as reações só ficam no
//! `/v1/estado`.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use pet_core::cerebro::{Agora, ConfigCerebro, Reacao};
use pet_core::config::ConfigEfetiva;
use pet_core::evento;
use pet_core::motor::{Motor, Tocou};
use pet_core::plataforma::Overlay;

use crate::comando::{Comando, Recebido};
use crate::estado::{Compartilhado, InfoSkin, Painel, Tela};
use crate::ingress::agora_desde_1970_ms;
use crate::personagem::{self, Escolha, NaTela, Onde, mesma_tela};

/// Quantas vezes por lote o núcleo esvazia os eventos da janela (um evento
/// pode gerar outro; uma janela com defeito não prende o laço).
const VOLTAS_DE_EVENTOS: usize = 8;

/// Reempresta a janela por um trecho. O `as_deref_mut` não serve: ele não
/// encurta o tempo de vida do `dyn Overlay` dentro do `Option`.
fn janela<'a>(ov: &'a mut Option<&mut dyn Overlay>) -> Option<&'a mut dyn Overlay> {
    match ov {
        Some(ov) => Some(&mut **ov),
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
        let mut nucleo = Nucleo {
            comp,
            motor: Motor::novo(ConfigCerebro::de(&config.config())),
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
    fn reler_config(&mut self, ov: Option<&mut dyn Overlay>) {
        let Some(pasta) = &self.pasta_config else {
            return;
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
        self.comp.definir_config(config);
        if &cerebro != self.motor.config_do_cerebro() {
            info!(
                "config: o cérebro passa a acompanhar as origens {} (celebração {:?})",
                cerebro.origens.join(", "),
                cerebro.modo
            );
            self.motor.reconfigurar_cerebro(cerebro);
            self.depois_do_cerebro(Vec::new(), ov);
        }
    }

    /// Escolhe o personagem (decisões 0011 e 0026), publica no `/v1/estado`
    /// e, com a janela aberta, troca na tela se mudou.
    pub fn escolher_personagem(&mut self, ov: Option<&mut dyn Overlay>) {
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
            return;
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
                self.motor.trocar_skin(ov, skin, agora);
            }
            None => self.motor.definir_skin(skin),
        }
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
    pub fn comando(&mut self, comando: Comando, mut ov: Option<&mut dyn Overlay>) {
        match comando {
            Comando::Evento(recebido) => self.evento(*recebido, ov),
            Comando::Tocar { reacao, resposta } => {
                let agora = self.agora_ms();
                let tocou = self.motor.tocar_comando(ov, &reacao, agora);
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
            Comando::Esconder | Comando::Mostrar => {
                let visivel = matches!(comando, Comando::Mostrar);
                self.motor.definir_visivel(visivel);
                info!("{}", if visivel { "mostrar" } else { "esconder" });
                if let Some(ov) = ov {
                    let agora = self.agora_ms();
                    self.motor.aplicar_visibilidade(ov, agora);
                }
            }
            Comando::Estresse { fps, segundos } => match ov {
                Some(ov) => {
                    let agora = self.agora_ms();
                    self.motor.estresse(ov, fps, segundos, agora);
                }
                None => aviso!("debug: estresse pedido sem compositor"),
            },
            Comando::Quadro(resposta) => {
                let quadro = ov.as_deref().and_then(|ov| self.motor.quadro_esperado(ov));
                let _ = resposta.try_send(quadro);
            }
            Comando::RecarregarPersonagem(feito) => {
                info!("aprovação mudou: escolhendo o personagem de novo");
                self.reler_config(janela(&mut ov));
                self.escolher_personagem(ov);
                let _ = feito.try_send(());
            }
        }
    }

    /// Um evento do Claude Code vai para o cérebro, no relógio da chegada.
    /// No log só vão o nome do evento e o começo do id da sessão (decisão
    /// 0019).
    fn evento(&mut self, recebido: Recebido, ov: Option<&mut dyn Overlay>) {
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
    fn depois_do_cerebro(&mut self, reacoes: Vec<Reacao>, mut ov: Option<&mut dyn Overlay>) {
        for reacao in &reacoes {
            self.reagir(reacao, janela(&mut ov));
        }
        self.comp.publicar_cerebro(&self.motor.resumo());
    }

    fn reagir(&mut self, reacao: &Reacao, ov: Option<&mut dyn Overlay>) {
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
        if !self.motor.tocar(ov, reacao.nome, agora) {
            if com_janela {
                depurar!("reação {} sem animação na tela", reacao.nome);
            } else {
                depurar!("reação {} sem compositor", reacao.nome);
            }
        }
    }

    /// O próximo prazo, em ms no relógio do laço: o do Motor (cérebro e
    /// animação) ou o da janela.
    pub fn proximo_prazo(&self, ov: Option<&dyn Overlay>) -> Option<u64> {
        [
            self.motor.proximo_prazo(),
            ov.and_then(|ov| ov.proximo_prazo()),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Venceu um prazo: os da janela, o do cérebro e o da animação, nessa
    /// ordem. Quem chama esvazia a caixa antes (decisão 0032).
    pub fn vencer(&mut self, mut ov: Option<&mut dyn Overlay>) {
        let agora = self.agora_ms();
        if let Some(ov) = janela(&mut ov) {
            ov.vencer(agora);
        }
        self.eventos_da_janela(janela(&mut ov));
        if self.motor.prazo_do_cerebro().is_some_and(|p| p <= agora) {
            let reacoes = self.motor.tique(self.agora());
            self.depois_do_cerebro(reacoes, janela(&mut ov));
        }
        if let Some(ov) = ov {
            self.motor.vencer_animacao(ov, agora);
        }
    }

    /// Os eventos da janela, em ordem, até acabarem.
    pub fn eventos_da_janela(&mut self, ov: Option<&mut dyn Overlay>) {
        let Some(ov) = ov else {
            return;
        };
        for _ in 0..VOLTAS_DE_EVENTOS {
            let eventos = ov.eventos();
            if eventos.is_empty() {
                return;
            }
            let agora = self.agora_ms();
            for evento in eventos {
                self.motor.evento_overlay(ov, evento, agora);
            }
        }
    }

    /// Uma janela nova: o compositor conectou. (Só o laço com janela usa: o
    /// do Linux; Windows e macOS no M8.)
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn conectou(&mut self, ov: &mut dyn Overlay) {
        self.comp.definir_tela(if self.motor.skin().is_some() {
            Tela::Ativa
        } else {
            Tela::SemPersonagem
        });
        let agora = self.agora_ms();
        self.motor.conectou(agora);
        self.motor.aplicar_visibilidade(ov, agora);
    }

    /// A janela acabou com a sessão: volta a esperar o compositor.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn desconectou(&mut self) {
        self.motor.desconectou();
        self.comp.definir_tela(Tela::Aguardando);
        self.comp.publicar_painel(Painel::default());
    }

    /// Publica o painel no `/v1/estado`.
    pub fn publicar(&mut self, ov: Option<&dyn Overlay>) {
        let agora = self.agora_ms();
        let painel = self.motor.painel(ov, agora);
        self.comp.publicar_painel(painel);
    }

    /// Fim do processo: esconde o pet (quadro transparente), larga a janela e
    /// espera, com prazo, o sistema confirmar que processou tudo, para o fade
    /// de saída do compositor sair vazio.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn encerrar(&mut self, ov: &mut dyn Overlay) {
        let agora = self.agora_ms();
        self.motor.encerrar(ov, agora);
    }
}
