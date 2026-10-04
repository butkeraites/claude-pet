//! O Motor: tudo o que o pet decide, sem saber em que sistema está
//! (decisão 0040, T8.0).
//!
//! Antes morava espalhado no laço do daemon (`laco.rs`) e na sessão Wayland
//! (`wl/mod.rs`). Agora é puro e testado com relógio falso e uma janela
//! falsa:
//!
//! - o **cérebro** ([`Cerebro`]) e o prazo dele;
//! - o **personagem** (a skin aprovada), o **pet** (animação) e o **palco**
//!   (D e posição no monitor);
//! - **mostrar e esconder** ([`passo_de_visibilidade`]);
//! - o **estresse** de debug e a contagem de **commits**;
//! - o **painel** do `/v1/estado` e o quadro esperado da checagem de nitidez;
//! - os **prazos** em milissegundos de um relógio monotônico que o laço de
//!   cada sistema injeta: o laço só acorda no [`Motor::proximo_prazo`] e
//!   entrega os eventos da janela ([`EventoOverlay`]).
//!
//! O que é do sistema (pixels no buffer, quadro em voo, região de input,
//! prazos internos da janela) fica atrás do trait [`Overlay`].
//!
//! Desde o M4, o Motor também arrasta e clica ([`arraste`], decisão 0048):
//! o ponteiro chega no palco, o pet anda em múltiplos de D e a área de toque
//! cresce para o palco inteiro só enquanto arrasta.

pub mod arraste;
pub mod balao;
mod desktop;
pub mod janelas;
mod pet;
mod poof;
pub mod posicoes;
mod ritmo;
pub mod viagem;

use std::rc::Rc;

use serde::Serialize;

use crate::animador;
use crate::cena::{self, Elemento};
use crate::cerebro::{Agora, Cerebro, ConfigCerebro, Reacao, Resumo};
use crate::confete::{Chuva, Grade};
use crate::evento::Evento;
use crate::geometria::{Ret, Tamanho};
use crate::plataforma::{
    Botao, Cursor, Desenho, EventoDesktop, EventoOverlay, EventoPonteiro, Fase, Monitor, Overlay,
    Passo, Punho, passo_de_visibilidade,
};
use crate::skin::Skin;

pub use arraste::{Arraste, Gesto};
pub use balao::Balao;
pub use desktop::{EstadoDesktop, PainelDesktop};
pub use pet::{Palco, Pet};
pub use posicoes::{Fracao, Posicoes};
pub use viagem::{Pouso, Seguir};

pub use ritmo::{Commits, Estresse, JANELA_COMMITS_MS};
use viagem::{Decisao, Fase as FaseViagem};

/// Confetes do teste de estresse, sempre com a mesma semente: medições
/// repetidas veem as mesmas trajetórias.
pub const CONFETES: usize = 40;
pub const SEMENTE_CONFETE: u64 = 7;
/// Lado de cada confete, em pixels de arte.
pub const LADO_CONFETE: i32 = 3;
/// O estado que o pet toca em laço enquanto é arrastado.
pub const ARRASTADO: &str = "dangle";
/// O estado que ele toca ao ser solto.
pub const SOLTO: &str = "land";
/// A risadinha do clique.
pub const RISADINHA: &str = "giggle";
/// O bocejo de quando a soneca começa e o despertar de quando ela acaba.
pub const BOCEJO: &str = "yawn";
pub const DESPERTAR: &str = "wake";
/// A soneca do botão direito (decisão 0053).
pub const SONECA_MS: u64 = 30 * 60 * 1000;

/// O que o pet publica para o `/v1/estado` (o laço publica; a entrada HTTP
/// só lê).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Painel {
    /// Nome do monitor da janela.
    pub monitor: Option<String>,
    /// Escala do monitor.
    pub escala: Option<f64>,
    /// Pixels do monitor por pixel de arte.
    pub d: Option<i32>,
    /// Célula do sprite no palco (pixels do monitor, relativa a ele) e
    /// recortada a ele (a foto e a checagem de nitidez recortam por aqui).
    pub sprite_disp: Option<Ret>,
    /// Região clicável nas coordenadas da janela (lógicas, no Wayland).
    pub regiao_entrada: Option<Ret>,
    /// O pet está desenhado na tela.
    pub visivel: bool,
    /// Teste de estresse rodando.
    pub estresse: bool,
    /// Commits no último minuto (orçamento: até 120 parado).
    pub commits_por_min: usize,
    pub commits_total: u64,
    /// Memória compartilhada dos buffers (o compositor guarda uma textura do
    /// mesmo tamanho, fora do processo).
    pub shm_bytes: usize,
    /// A reação tocando na tela agora (`nod`, `done_small`, …).
    pub reacao: Option<String>,
    /// O pet está sendo arrastado.
    pub arrastando: bool,
    /// A fase da viagem para o monitor em foco (`poof`, `saindo`,
    /// `chegando`, `entrando`), se houver uma.
    pub viagem: Option<&'static str>,
    /// As linhas do balão na tela, se houver um.
    pub balao: Option<Vec<String>>,
    /// Quanto falta da soneca (o botão direito), se o pet está cochilando.
    pub soneca_restante_s: Option<u64>,
    /// O desktop: a fonte dos eventos, o monitor em foco, a janela ativa (só
    /// o endereço) e o que a conexão sabe fazer.
    pub desktop: PainelDesktop,
}

/// O que um `tocar` do `/v1/comando` fez (decisão 0033).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tocou {
    /// Tocou na tela a tag `tag` da skin.
    NaTela { tag: String },
    /// A skin sabe tocar (`tag`), mas não há onde mostrar: sem compositor,
    /// ou o pet escondido.
    ForaDaTela { tag: String, motivo: &'static str },
    /// Nenhum personagem aprovado.
    SemPersonagem,
    /// A skin não tem estado, reserva nem tag com esse nome.
    Desconhecida { skin: String },
}

/// O sprite como deveria estar na tela: o RGBA exato, em pixels do monitor,
/// para a checagem de nitidez comparar com uma captura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuadroEsperado {
    pub monitor: String,
    /// Recorte em pixels do monitor (o `sprite_disp`).
    pub area: Ret,
    /// Canto da célula: origem da grade de blocos D×D.
    pub grade: (i32, i32),
    pub d: i32,
    /// Número do commit que pôs este quadro na tela.
    pub seq: u64,
    /// Há quanto tempo foi esse commit.
    pub idade_ms: u64,
    /// RGBA direto, `area.w * area.h * 4` bytes.
    pub rgba: Vec<u8>,
}

/// O pet inteiro, sem sistema. Os tempos são milissegundos do relógio
/// monotônico do laço (o mesmo do `mono_ms` do cérebro).
pub struct Motor {
    cerebro: Cerebro,
    /// O personagem: a skin aprovada (ou a de teste, em debug).
    skin: Option<Rc<Skin>>,
    /// O pet animado; só existe com uma janela (sessão com o compositor).
    pet: Option<Pet>,
    /// Onde o pet fica no palco do monitor atual (D e posição).
    palco: Option<Palco>,
    /// O pet deve estar na tela (`/v1/comando` esconde e mostra).
    visivel: bool,
    estresse: Option<Estresse>,
    commits: Commits,
    /// A próxima troca de quadro da animação.
    proximo_quadro: Option<u64>,
    /// O tamanho do pet na tela (`aparencia.tamanho`, decisão 0042).
    tamanho: Tamanho,
    /// O que os eventos do desktop contaram (decisão 0043).
    desktop: EstadoDesktop,
    /// Arrastar e clicar (decisão 0048).
    arraste: Arraste,
    /// Onde o Renan deixou o pet em cada monitor (decisão 0049).
    posicoes: Posicoes,
    /// As posições mudaram desde a última gravação.
    posicoes_novas: bool,
    /// A chave do monitor do palco de agora.
    chave_do_monitor: Option<String>,
    /// Seguir o monitor ativo (decisão 0051).
    seguir: Seguir,
    /// O balão na tela (decisão 0052).
    balao: Option<Balao>,
    /// Parede − monotônico, do último [`Agora`] que o Motor viu: a hora de
    /// parede de um instante do relógio do laço (o "há quanto tempo" do
    /// balão).
    deslocamento_parede: u64,
    /// Até quando o pet cochila (o botão direito; decisão 0053).
    soneca_ate: Option<u64>,
}

impl Motor {
    pub fn novo(config: ConfigCerebro) -> Motor {
        Motor {
            cerebro: Cerebro::novo(config),
            skin: None,
            pet: None,
            palco: None,
            visivel: true,
            estresse: None,
            commits: Commits::default(),
            proximo_quadro: None,
            tamanho: Tamanho::Normal,
            desktop: EstadoDesktop::default(),
            arraste: Arraste::default(),
            posicoes: Posicoes::default(),
            posicoes_novas: false,
            chave_do_monitor: None,
            seguir: Seguir::default(),
            balao: None,
            deslocamento_parede: 0,
            soneca_ate: None,
        }
    }

    /// Acerta o relógio de parede pelo de agora (o núcleo chama a cada lote).
    pub fn acertar_relogio(&mut self, agora: Agora) {
        self.deslocamento_parede = agora.parede_ms.saturating_sub(agora.mono_ms);
    }

    /// A hora de parede (ms desde 1970) de um instante do relógio do laço.
    fn parede(&self, mono_ms: u64) -> u64 {
        mono_ms + self.deslocamento_parede
    }

    // --- o balão (decisão 0052) ----------------------------------------------

    /// Mostra um balão com `linhas` em cima do pet (some sozinho).
    pub fn mostrar_balao(
        &mut self,
        ov: Option<&mut dyn Overlay>,
        linhas: Vec<String>,
        agora_ms: u64,
    ) {
        self.balao = Some(Balao::novo(linhas, agora_ms));
        if let Some(ov) = ov {
            self.desenhar(ov, agora_ms, false);
        }
    }

    /// O balão na tela agora.
    pub fn balao(&self, agora_ms: u64) -> Option<&Balao> {
        self.balao.as_ref().filter(|b| agora_ms < b.ate_ms)
    }

    /// As sessões abertas, uma por linha (projeto, estado e há quanto
    /// tempo), as reais antes das de teste, a mais recente primeiro.
    pub fn linhas_das_sessoes(&self, agora_ms: u64) -> Vec<String> {
        let agora = self.parede(agora_ms);
        let mut sessoes = self.cerebro.resumo().sessoes;
        sessoes.sort_by_key(|s| (s.teste, std::cmp::Reverse(s.ultimo_evento_ms)));
        if sessoes.is_empty() {
            return vec!["nenhuma sessão do Claude aberta".into()];
        }
        sessoes
            .iter()
            .map(|s| {
                balao::linha_da_sessao(
                    s.proj.as_deref(),
                    balao::estado(s),
                    s.estado_desde_ms,
                    agora,
                    s.teste,
                )
            })
            .collect()
    }

    // --- posições salvas (decisão 0049) --------------------------------------

    /// As posições lidas de `/state` na partida.
    pub fn definir_posicoes(&mut self, posicoes: Posicoes) {
        self.posicoes = posicoes;
        self.posicoes_novas = false;
    }

    pub fn posicoes(&self) -> &Posicoes {
        &self.posicoes
    }

    /// O arquivo das posições, se elas mudaram desde a última vez (quem
    /// grava é o núcleo do daemon).
    pub fn posicoes_para_gravar(&mut self) -> Option<String> {
        if !self.posicoes_novas {
            return None;
        }
        self.posicoes_novas = false;
        Some(self.posicoes.json())
    }

    /// Guarda onde o pet está agora, no monitor do palco.
    fn guardar_posicao(&mut self) {
        let (Some(palco), Some(pet), Some(chave)) = (
            self.palco,
            self.pet.as_ref(),
            self.chave_do_monitor.as_deref(),
        ) else {
            return;
        };
        let fracao = Fracao::da_celula(palco.x, palco.y, palco.d, &pet.skin().ancoras, palco.tela);
        if self.posicoes.de(chave) != Some(fracao) {
            self.posicoes.guardar(chave, fracao);
            self.posicoes_novas = true;
        }
    }

    // --- cérebro -----------------------------------------------------------

    pub fn config_do_cerebro(&self) -> &ConfigCerebro {
        self.cerebro.config()
    }

    pub fn reconfigurar_cerebro(&mut self, config: ConfigCerebro) {
        self.cerebro.reconfigurar(config);
    }

    /// Um evento do Claude Code, no relógio da chegada (decisão 0032).
    pub fn evento(&mut self, ev: &Evento, recebido_ms: u64, agora: Agora) -> Vec<Reacao> {
        self.acertar_relogio(agora);
        self.cerebro.receber(ev, recebido_ms, agora)
    }

    /// O prazo do cérebro venceu (acomodação do Stop, sessões que expiram).
    pub fn tique(&mut self, agora: Agora) -> Vec<Reacao> {
        self.acertar_relogio(agora);
        self.cerebro.tique(agora)
    }

    pub fn prazo_do_cerebro(&self) -> Option<u64> {
        self.cerebro.proximo_prazo()
    }

    pub fn resumo(&self) -> Resumo {
        self.cerebro.resumo()
    }

    // --- personagem e janela -----------------------------------------------

    pub fn skin(&self) -> Option<&Rc<Skin>> {
        self.skin.as_ref()
    }

    /// Troca o personagem sem janela aberta (vale na próxima sessão).
    pub fn definir_skin(&mut self, skin: Option<Rc<Skin>>) {
        self.skin = skin;
    }

    pub fn tamanho(&self) -> Tamanho {
        self.tamanho
    }

    /// Troca o tamanho do pet (o config relido, decisão 0042). Com a janela
    /// pronta, o palco é refeito e o quadro novo redesenha a tela toda; sem,
    /// vale no próximo palco. A skin não muda.
    pub fn definir_tamanho(
        &mut self,
        tamanho: Tamanho,
        ov: Option<&mut dyn Overlay>,
        agora_ms: u64,
    ) {
        if tamanho == self.tamanho {
            return;
        }
        self.tamanho = tamanho;
        if let Some(ov) = ov {
            self.redesenhar_palco(ov, agora_ms);
        }
    }

    /// Refaz o palco com o tamanho de agora e redesenha a tela toda já, se o
    /// pet está num palco: o tamanho mudou e a skin não (decisão 0046).
    pub fn redesenhar_palco(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        if self.palco.is_some() {
            ov.esquecer_cena();
            self.refazer_palco(ov, agora_ms);
        }
    }

    /// O pet deve estar na tela (pedido do `/v1/comando`).
    pub fn visivel(&self) -> bool {
        self.visivel
    }

    pub fn definir_visivel(&mut self, visivel: bool) {
        self.visivel = visivel;
    }

    /// O pet deve aparecer agora: há personagem, ninguém mandou esconder e
    /// a proteção de tela não está na tela (decisão 0053).
    pub fn quer_mostrar(&self) -> bool {
        self.visivel && self.pet.is_some() && !self.desktop.protetor_ativo()
    }

    /// Uma janela nova (o compositor conectou): o pet recomeça a animação
    /// agora, sem palco até a janela ficar pronta.
    pub fn conectou(&mut self, agora_ms: u64) {
        self.pet = self.skin.clone().map(|skin| Pet::novo(skin, agora_ms));
        self.palco = None;
        self.arraste.cancelar();
        self.seguir.terminar();
        self.estresse = None;
        self.commits = Commits::default();
        self.proximo_quadro = None;
    }

    /// A janela acabou com a sessão (o compositor caiu).
    pub fn desconectou(&mut self) {
        self.pet = None;
        self.palco = None;
        self.arraste.cancelar();
        self.seguir.terminar();
        self.estresse = None;
        self.commits = Commits::default();
        self.proximo_quadro = None;
    }

    /// Leva a janela à visibilidade pedida. Numa viagem para outro monitor,
    /// quem cria a janela nova é a viagem; esconder no meio desiste dela.
    pub fn aplicar_visibilidade(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        match self.seguir.fase() {
            Some(FaseViagem::Entrando { .. }) | None => {}
            Some(_) if self.quer_mostrar() => return,
            Some(_) => {
                self.seguir.terminar();
            }
        }
        let passo = passo_de_visibilidade(ov.fase(), self.quer_mostrar());
        self.aplicar_passo(ov, passo, agora_ms);
    }

    fn aplicar_passo(&mut self, ov: &mut dyn Overlay, passo: Passo, agora_ms: u64) {
        match passo {
            Passo::Nada => {}
            Passo::Criar => {
                ov.criar();
                self.palco = None;
            }
            Passo::Cancelar => {
                ov.cancelar_saida();
                // Como esconder, mostrar é uma ordem e não animação: o pet
                // volta já, mesmo com um quadro em voo (que, com a tela
                // apagada, só seria mostrado quando ela acendesse).
                self.desenhar(ov, agora_ms, true);
            }
            Passo::ApagarEDestruir => {
                self.largar_o_arraste(agora_ms);
                self.proximo_quadro = None;
                self.estresse = None;
                let commit = match &self.pet {
                    Some(pet) => ov.apagar_e_destruir(pet.skin()),
                    None => {
                        ov.destruir();
                        false
                    }
                };
                if commit {
                    self.commits.contar(agora_ms);
                }
            }
            Passo::Destruir => {
                self.largar_o_arraste(agora_ms);
                self.proximo_quadro = None;
                self.estresse = None;
                ov.destruir();
            }
        }
    }

    /// Troca o personagem com a janela aberta (aprovação ou revogação,
    /// decisão 0026). Sem skin, o pet se esconde com o quadro transparente
    /// (desenhado com a skin velha). Com skin nova, a cena velha é esquecida
    /// e o palco (D e posição, que dependem da skin) é refeito: na hora, se a
    /// janela está pronta, ou quando ela ficar. O quadro novo redesenha a tela
    /// toda, sem fantasma da skin velha.
    pub fn trocar_skin(&mut self, ov: &mut dyn Overlay, skin: Option<Rc<Skin>>, agora_ms: u64) {
        match skin {
            None => {
                let passo = passo_de_visibilidade(ov.fase(), false);
                self.aplicar_passo(ov, passo, agora_ms);
                self.pet = None;
                self.skin = None;
            }
            Some(skin) => {
                self.proximo_quadro = None;
                self.estresse = None;
                self.skin = Some(Rc::clone(&skin));
                self.pet = Some(Pet::novo(skin, agora_ms));
                self.palco = None;
                ov.esquecer_cena();
                self.aplicar_visibilidade(ov, agora_ms);
                self.refazer_palco(ov, agora_ms);
            }
        }
    }

    /// Palco do pet num monitor pronto, na posição salva daquele monitor ou
    /// no canto inferior direito da área útil, com o log de onde ele ficou.
    fn montar_palco(&mut self, monitor: &Monitor) {
        let Some(pet) = &self.pet else {
            return;
        };
        let mut palco = pet.palco(monitor, self.tamanho);
        self.chave_do_monitor = posicoes::chave(monitor);
        if let Some(fracao) = self
            .chave_do_monitor
            .as_deref()
            .and_then(|c| self.posicoes.de(c))
        {
            let (x, y) = fracao.celula(palco.d, &pet.skin().ancoras, palco.tela);
            (palco.x, palco.y) = pet.prender(&palco, x, y);
        }
        info!(
            "pet «{}» com D={} (tamanho {}) e célula em ({}, {}) pixels do monitor",
            pet.skin().id,
            palco.d,
            self.tamanho.nome(),
            palco.x,
            palco.y
        );
        self.palco = Some(palco);
    }

    /// Refaz o palco numa janela que já está pronta (ela só avisa quando o
    /// monitor ou a escala mudam, não quando a skin muda) e desenha já, mesmo
    /// com um quadro em voo. Janela ainda não pronta ou saindo: nada; o palco
    /// vem com [`EventoOverlay::Pronta`].
    fn refazer_palco(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        if ov.fase() == Fase::Saindo {
            return;
        }
        let Some(monitor) = ov.pronta() else {
            return;
        };
        self.montar_palco(&monitor);
        self.desenhar(ov, agora_ms, true);
    }

    /// Um evento da janela. Recebe a conexão inteira ([`Punho`]): o clique
    /// do M4 foca janelas pelo desktop.
    pub fn evento_overlay(&mut self, punho: &mut dyn Punho, evento: EventoOverlay, agora_ms: u64) {
        let ov = punho.janela();
        match evento {
            // O palco sai do monitor de agora, não de quando o evento entrou
            // na fila: dois na mesma leva (escala e `configure` juntos) não
            // desenham com um palco velho no buffer novo.
            EventoOverlay::Pronta => {
                if self.pet.is_some()
                    && let Some(monitor) = ov.pronta()
                {
                    if self.seguir.fase() == Some(FaseViagem::Chegando) {
                        self.chegar(ov, &monitor, agora_ms);
                    } else {
                        self.montar_palco(&monitor);
                        self.desenhar(ov, agora_ms, false);
                        // A janela pode ter nascido longe do monitor em foco.
                        self.seguir.conferir_em(agora_ms);
                    }
                }
            }
            EventoOverlay::Redesenhar => self.desenhar(ov, agora_ms, false),
            EventoOverlay::Sumiu => {
                self.largar_o_arraste(agora_ms);
                self.proximo_quadro = None;
                self.palco = None;
                // O compositor fechou a janela no meio da viagem (o monitor
                // saiu): a nova vem com o `Recriar`.
                if self.seguir.em_viagem() {
                    self.seguir.mudar_fase(FaseViagem::Chegando);
                }
            }
            EventoOverlay::Recriar => {
                if self.quer_mostrar() && ov.fase() == Fase::Ausente {
                    ov.criar();
                    self.palco = None;
                }
            }
            EventoOverlay::Ponteiro(ponteiro) => self.ponteiro(punho, ponteiro, agora_ms),
            // A janela velha morreu no meio da viagem: a nova nasce no
            // monitor em foco (output NULL).
            EventoOverlay::Saiu => {
                if self.seguir.fase() == Some(FaseViagem::Saindo) {
                    if self.quer_mostrar() && ov.fase() == Fase::Ausente {
                        ov.criar();
                        self.palco = None;
                        self.seguir.mudar_fase(FaseViagem::Chegando);
                    } else {
                        self.seguir.terminar();
                    }
                }
            }
        }
    }

    // --- seguir o monitor ativo (decisão 0051) -------------------------------

    /// Começa a viagem para o monitor em foco: o poof de saída (se o pet está
    /// desenhado e a viagem não é rápida) ou a saída direta.
    fn comecar_viagem(&mut self, ov: &mut dyn Overlay, pouso: Option<Pouso>, agora_ms: u64) {
        self.largar_o_arraste(agora_ms);
        self.balao = None;
        let desenhado = ov.fase() == (Fase::Viva { conteudo: true }) && self.palco.is_some();
        let rapida = self.seguir.comecar(agora_ms, pouso, desenhado);
        info!(
            "indo para o monitor em foco ({}){}",
            self.seguir.alvo.as_deref().unwrap_or("?"),
            if rapida { ", sem poof" } else { "" }
        );
        if rapida {
            self.sair_para_viajar(ov, agora_ms);
        } else {
            self.desenhar(ov, agora_ms, true);
        }
    }

    /// O poof de saída acabou (ou não houve): quadro transparente e a janela
    /// morre; sem nada desenhado, morre já e a nova nasce em seguida.
    fn sair_para_viajar(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        self.seguir.mudar_fase(FaseViagem::Saindo);
        let passo = passo_de_visibilidade(ov.fase(), false);
        self.aplicar_passo(ov, passo, agora_ms);
        if ov.fase() == Fase::Ausente {
            if self.quer_mostrar() {
                ov.criar();
                self.palco = None;
                self.seguir.mudar_fase(FaseViagem::Chegando);
            } else {
                self.seguir.terminar();
            }
        }
    }

    /// A janela nova ficou pronta: o palco na posição salva daquele monitor
    /// (ou no ponto onde o pet foi solto) e o poof de chegada.
    fn chegar(&mut self, ov: &mut dyn Overlay, monitor: &Monitor, agora_ms: u64) {
        self.montar_palco(monitor);
        let viagem = self.seguir.viagem;
        if let Some(pouso) = viagem.and_then(|v| v.pouso)
            && let Some(origem) = monitor.origem
        {
            let e = monitor.escala;
            let x = ((pouso.global.0 - pouso.pegada.0 - origem.0 as f64) * e).round() as i32;
            let y = ((pouso.global.1 - pouso.pegada.1 - origem.1 as f64) * e).round() as i32;
            self.mover_para((x, y));
            if let Some(pet) = self.pet.as_mut() {
                pet.tocar(SOLTO, agora_ms);
            }
            self.guardar_posicao();
        }
        if viagem.is_none_or(|v| v.rapida) {
            self.seguir.terminar();
            self.seguir.conferir_em(agora_ms);
        } else {
            self.seguir.mudar_fase(FaseViagem::Entrando {
                inicio_ms: agora_ms,
            });
        }
        self.desenhar(ov, agora_ms, false);
    }

    /// Os prazos da viagem: o fim dos poofs e a hora de conferir o foco.
    fn vencer_viagem(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        match self.seguir.fase() {
            Some(FaseViagem::Poof { inicio_ms }) if agora_ms >= inicio_ms + poof::DURACAO_MS => {
                self.sair_para_viajar(ov, agora_ms);
            }
            Some(FaseViagem::Entrando { inicio_ms })
                if agora_ms >= inicio_ms + poof::DURACAO_MS =>
            {
                self.seguir.terminar();
                self.desenhar(ov, agora_ms, false);
                self.seguir.conferir_em(agora_ms);
            }
            _ => {}
        }
        let monitor = ov.pronta().and_then(|m| m.nome);
        let livre = self.quer_mostrar()
            && !self.arraste.segurando()
            && matches!(ov.fase(), Fase::Viva { .. });
        if self.seguir.decidir(agora_ms, monitor.as_deref(), livre) == Decisao::Viajar {
            self.comecar_viagem(ov, None, agora_ms);
        }
    }

    /// O pouso no monitor onde o botão subiu, se ele subiu fora deste: o
    /// ponto no desktop (lógico) e a pegada, pela origem do monitor.
    fn pouso_fora(&self, ov: &dyn Overlay, x: i32, y: i32, pegada: (i32, i32)) -> Option<Pouso> {
        let palco = self.palco?;
        let fora = x < 0 || y < 0 || x >= palco.tela.0 || y >= palco.tela.1;
        if !fora {
            return None;
        }
        let monitor = ov.pronta()?;
        let origem = monitor.origem?;
        let e = monitor.escala;
        Some(Pouso {
            global: (
                origem.0 as f64 + x as f64 / e,
                origem.1 as f64 + y as f64 / e,
            ),
            pegada: (pegada.0 as f64 / e, pegada.1 as f64 / e),
        })
    }

    // --- arrastar e clicar (decisão 0048) ------------------------------------

    /// Um evento do ponteiro, no palco.
    fn ponteiro(&mut self, punho: &mut dyn Punho, evento: EventoPonteiro, agora_ms: u64) {
        let (Some(palco), true) = (self.palco, self.pet.is_some()) else {
            self.arraste.cancelar();
            return;
        };
        let no_corpo = match evento {
            EventoPonteiro::Apertou { x, y, .. } => self.acerta_o_pet(x, y),
            _ => false,
        };
        let limiar = (arraste::LIMIAR_LOGICO * palco.escala).round() as i32;
        let gesto = self.arraste.ponteiro(
            evento,
            agora_ms,
            (palco.x, palco.y),
            no_corpo,
            limiar,
            palco.d,
        );
        self.aplicar_gesto(punho, gesto, agora_ms);
    }

    fn aplicar_gesto(&mut self, punho: &mut dyn Punho, gesto: Gesto, agora_ms: u64) {
        match gesto {
            Gesto::Nada => {}
            Gesto::Apertou => punho.janela().cursor(Cursor::Agarrar),
            Gesto::Comecou => {
                self.balao = None;
                if let Some(pet) = self.pet.as_mut() {
                    pet.segurar(ARRASTADO, agora_ms);
                }
                self.ir_para_o_alvo();
                self.desenhar(punho.janela(), agora_ms, false);
            }
            Gesto::Moveu => {
                if self.ir_para_o_alvo() {
                    self.desenhar(punho.janela(), agora_ms, false);
                }
            }
            Gesto::Soltou {
                x,
                y,
                celula,
                pegada,
            } => {
                self.mover_para(celula);
                let ov = punho.janela();
                match self.pouso_fora(ov, x, y, pegada) {
                    // Solto fora do monitor: vai já para o monitor debaixo do
                    // ponteiro (o compositor já o focou), e pousa lá.
                    Some(pouso) => {
                        ov.cursor(Cursor::Pegar);
                        if let Some(pet) = self.pet.as_mut() {
                            pet.largar(agora_ms);
                        }
                        self.comecar_viagem(ov, Some(pouso), agora_ms);
                    }
                    None => self.pousar(ov, agora_ms),
                }
            }
            Gesto::Cancelou => self.pousar(punho.janela(), agora_ms),
            Gesto::Clique(botao) => {
                punho.janela().cursor(Cursor::Pegar);
                self.clique(punho, botao, agora_ms);
            }
        }
    }

    /// Leva a célula ao alvo do arraste (preso na área útil). `true` se ela
    /// mudou de lugar.
    fn ir_para_o_alvo(&mut self) -> bool {
        let Some(d) = self.palco.map(|p| p.d) else {
            return false;
        };
        match self.arraste.alvo(d) {
            Some(alvo) => self.mover_para(alvo),
            None => false,
        }
    }

    /// Põe a célula em `celula` (presa na área útil). `true` se mudou.
    fn mover_para(&mut self, celula: (i32, i32)) -> bool {
        let (Some(palco), Some(pet)) = (self.palco.as_mut(), self.pet.as_ref()) else {
            return false;
        };
        let (x, y) = pet.prender(palco, celula.0, celula.1);
        let mudou = (x, y) != (palco.x, palco.y);
        palco.x = x;
        palco.y = y;
        mudou
    }

    /// O arraste acabou (soltou ou o fail-safe): o cursor volta, o pet larga
    /// o laço, pousa, a área de toque volta ao corpo e a posição fica
    /// guardada para aquele monitor (decisão 0049).
    fn pousar(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        ov.cursor(Cursor::Pegar);
        if let Some(pet) = self.pet.as_mut() {
            pet.largar(agora_ms);
            pet.tocar(SOLTO, agora_ms);
        }
        self.guardar_posicao();
        self.desenhar(ov, agora_ms, false);
        // O foco pode ter mudado de monitor durante o arraste.
        self.seguir.conferir_em(agora_ms);
    }

    /// Esconder, a janela fechada ou o fim da conexão largam o arraste sem
    /// desenhar (a área de toque volta ao corpo no próximo quadro).
    fn largar_o_arraste(&mut self, agora_ms: u64) {
        if self.arraste.cancelar()
            && let Some(pet) = self.pet.as_mut()
        {
            pet.largar(agora_ms);
        }
    }

    /// Um clique no pet: o esquerdo dá a risadinha e mostra o balão com as
    /// sessões abertas (levar ao terminal de uma sessão chega na T4.10); o
    /// direito, a soneca (T4.7).
    fn clique(&mut self, punho: &mut dyn Punho, botao: Botao, agora_ms: u64) {
        match botao {
            Botao::Esquerdo => {
                self.tocar(Some(punho.janela()), RISADINHA, agora_ms);
                let linhas = self.linhas_das_sessoes(agora_ms);
                self.mostrar_balao(Some(punho.janela()), linhas, agora_ms);
            }
            Botao::Direito => self.alternar_soneca(punho.janela(), agora_ms),
            _ => {}
        }
    }

    // --- a soneca (decisão 0053) --------------------------------------------

    /// O botão direito: começa a soneca de 30 min (o bocejo e o selo "zZ")
    /// ou, se o pet já cochila, acorda (o despertar).
    pub fn alternar_soneca(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        if self.soneca(agora_ms).is_some() {
            self.soneca_ate = None;
            info!("soneca: acordou");
            self.tocar(Some(&mut *ov), DESPERTAR, agora_ms);
        } else {
            self.soneca_ate = Some(agora_ms + SONECA_MS);
            info!("soneca de {} min", SONECA_MS / 60_000);
            self.tocar(Some(&mut *ov), BOCEJO, agora_ms);
        }
        self.desenhar(ov, agora_ms, false);
    }

    /// Até quando o pet cochila, se cochila.
    pub fn soneca(&self, agora_ms: u64) -> Option<u64> {
        self.soneca_ate.filter(|&ate| agora_ms < ate)
    }

    /// Uma reação do cérebro: na soneca, só as pequenas (o pulinho e o que
    /// for maior viram o aceno discreto).
    pub fn reagir(&mut self, ov: Option<&mut dyn Overlay>, reacao: &str, agora_ms: u64) -> bool {
        let reacao = if self.soneca(agora_ms).is_some() && reacao != crate::cerebro::TCHAU {
            crate::cerebro::ACENO
        } else {
            reacao
        };
        self.tocar(ov, reacao, agora_ms)
    }

    /// O pet está sendo arrastado.
    pub fn arrastando(&self) -> bool {
        self.arraste.arrastando()
    }

    // --- desktop -------------------------------------------------------------

    /// Um evento do desktop (decisão 0043): o monitor em foco, a janela
    /// ativa, a presença. Devolve se o que o `/v1/estado` mostra mudou.
    pub fn evento_desktop(
        &mut self,
        ov: Option<&mut dyn Overlay>,
        evento: &EventoDesktop,
        agora: Agora,
    ) -> bool {
        self.acertar_relogio(agora);
        if let EventoDesktop::MonitorEmFoco(nome) = evento {
            self.seguir.foco(nome.clone(), agora.mono_ms);
        }
        let protetor_antes = self.desktop.protetor_ativo();
        let mudou = self.desktop.aplicar(evento, agora.parede_ms);
        // A proteção de tela do Omarchy abriu ou fechou: o pet sai e volta
        // (decisão 0053).
        if self.desktop.protetor_ativo() != protetor_antes {
            info!(
                "proteção de tela {}",
                if protetor_antes {
                    "fechou: o pet volta"
                } else {
                    "abriu: o pet se esconde"
                }
            );
            if let Some(ov) = ov {
                self.aplicar_visibilidade(ov, agora.mono_ms);
            }
        }
        mudou
    }

    /// O que os eventos do desktop contaram.
    pub fn desktop(&self) -> &EstadoDesktop {
        &self.desktop
    }

    // --- desenho -------------------------------------------------------------

    /// Desenha o quadro de agora (se a janela está viva e algo mudou) e
    /// marca a próxima troca. Com um quadro em voo o desenho espera por ele e
    /// nenhum prazo é marcado: é o [`EventoOverlay::Redesenhar`] que chama de
    /// novo, já com o quadro do instante em que chegar. Com a tela apagada ele
    /// não chega, e o pet fica sem commit nenhum até ela acender (decisão
    /// 0018). `forcar` faz o commit mesmo com um quadro em voo.
    pub fn desenhar(&mut self, ov: &mut dyn Overlay, agora_ms: u64, forcar: bool) {
        let Some(palco) = self.palco else {
            return;
        };
        let Some(pet) = self.pet.as_ref() else {
            return;
        };
        if !matches!(ov.fase(), Fase::Viva { .. }) {
            return;
        }
        let (mut cena, mut proxima) = pet.cena(&palco, agora_ms);
        // O poof da viagem (decisão 0051): a nuvem em volta do corpo, e o pet
        // some na saída e aparece na chegada.
        let poof = match self.seguir.fase() {
            Some(FaseViagem::Poof { inicio_ms }) => Some((inicio_ms, false)),
            Some(FaseViagem::Entrando { inicio_ms }) => Some((inicio_ms, true)),
            _ => None,
        };
        if let Some((inicio_ms, entrando)) = poof {
            match poof::passo(inicio_ms, agora_ms) {
                Some((passo, prazo)) => {
                    if !poof::mostra_o_pet(passo, entrando) {
                        cena.clear();
                    }
                    if let Some(corpo) = pet.toque_no_palco(&palco) {
                        cena.extend(poof::nuvem(corpo, palco.d, passo, entrando));
                    }
                    proxima = Some(proxima.map_or(prazo, |p| p.min(prazo)));
                }
                None if !entrando => cena.clear(),
                None => {}
            }
        }
        // O selo "zZ" da soneca, parado, enquanto ela durar.
        if let Some(ate) = self.soneca(agora_ms)
            && let Some(corpo) = pet.toque_no_palco(&palco)
            && poof.is_none()
        {
            cena.extend(balao::selo_zz(corpo, palco.area, balao::dt(palco.d)));
            proxima = Some(proxima.map_or(ate, |p| p.min(ate)));
        }
        // O balão em cima do corpo, até o prazo dele.
        if let Some(balao) = self.balao.as_ref().filter(|b| agora_ms < b.ate_ms)
            && let Some(corpo) = pet.toque_no_palco(&palco)
        {
            cena.extend(balao::elementos(
                &balao.linhas,
                corpo,
                palco.area,
                balao::dt(palco.d),
            ));
            proxima = Some(proxima.map_or(balao.ate_ms, |p| p.min(balao.ate_ms)));
        }
        if self.estresse.as_ref().is_some_and(|e| agora_ms >= e.fim_ms) {
            self.estresse = None;
            info!("debug: estresse acabou");
        }
        if let Some(estresse) = self.estresse.as_mut() {
            estresse.avancar(agora_ms);
            cena.extend(estresse.chuva.elementos());
            let prazo = estresse.proximo_prazo();
            proxima = Some(proxima.map_or(prazo, |p| p.min(prazo)));
        }
        // Arrastando, a área de toque é o palco inteiro: o arraste continua
        // até numa área de trabalho vazia, onde o compositor pode perder a
        // pegada implícita. Ao soltar, volta ao corpo.
        let toque = if self.arraste.arrastando() {
            Some(Ret::novo(0, 0, palco.tela.0, palco.tela.1))
        } else {
            pet.toque_no_palco(&palco)
        };
        match ov.desenhar(&cena, pet.skin(), toque, forcar) {
            Ok(Desenho::Enviado { retangulos, area }) => {
                self.commits.contar(agora_ms);
                if self.estresse.is_none() {
                    depurar!("quadro enviado: {retangulos} retângulo(s) de dano, {area} px");
                }
            }
            Ok(Desenho::SoEstado) => self.commits.contar(agora_ms),
            // O quadro em voo chama `desenhar` de novo quando for mostrado.
            Ok(Desenho::Adiado) => return,
            Ok(Desenho::SemMudanca) => {}
            Err(e) => aviso!("desenho: {e}"),
        }
        if let Some(proxima) = proxima {
            self.proximo_quadro = Some(proxima);
        }
    }

    /// A próxima troca de quadro da animação, se houver uma marcada.
    pub fn prazo_da_animacao(&self) -> Option<u64> {
        self.proximo_quadro
    }

    /// O prazo da animação venceu: desenha o quadro de agora.
    pub fn vencer_animacao(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        if self.proximo_quadro.is_some_and(|p| p <= agora_ms) {
            self.proximo_quadro = None;
            self.desenhar(ov, agora_ms, false);
        }
    }

    /// Os prazos do Motor que vencem com a conexão: o arraste (segurar e o
    /// fail-safe), a viagem para o monitor em foco e a animação.
    pub fn vencer(&mut self, punho: &mut dyn Punho, agora_ms: u64) {
        let gesto = self.arraste.vencer(agora_ms);
        self.aplicar_gesto(punho, gesto, agora_ms);
        self.vencer_viagem(punho.janela(), agora_ms);
        if self.balao.as_ref().is_some_and(|b| agora_ms >= b.ate_ms) {
            self.balao = None;
            self.desenhar(punho.janela(), agora_ms, false);
        }
        if self.soneca_ate.is_some_and(|ate| agora_ms >= ate) {
            self.soneca_ate = None;
            info!("soneca acabou");
            self.desenhar(punho.janela(), agora_ms, false);
        }
        self.vencer_animacao(punho.janela(), agora_ms);
    }

    /// O próximo prazo do Motor (cérebro, animação, arraste ou viagem).
    pub fn proximo_prazo(&self) -> Option<u64> {
        [
            self.cerebro.proximo_prazo(),
            self.proximo_quadro,
            self.arraste.prazo(),
            self.seguir.prazo(),
            self.balao.as_ref().map(|b| b.ate_ms),
            self.soneca_ate,
        ]
        .into_iter()
        .flatten()
        .min()
    }

    // --- reações -------------------------------------------------------------

    /// Toca uma reação uma vez e volta à pose (o cérebro e o `tocar`).
    /// `false` sem pet (sem janela ou sem personagem) ou quando a skin não
    /// sabe tocar a reação. Com o pet escondido a animação corre no relógio
    /// sem desenhar nada.
    pub fn tocar(&mut self, ov: Option<&mut dyn Overlay>, reacao: &str, agora_ms: u64) -> bool {
        let Some(pet) = self.pet.as_mut() else {
            return false;
        };
        if !pet.tocar(reacao, agora_ms) {
            return false;
        }
        if let Some(ov) = ov {
            self.desenhar(ov, agora_ms, false);
        }
        true
    }

    /// `tocar` do `/v1/comando`: a tag que a skin toca para a reação e se
    /// ela apareceu (decisão 0033). Escondido, não toca.
    pub fn tocar_comando(
        &mut self,
        ov: Option<&mut dyn Overlay>,
        reacao: &str,
        agora_ms: u64,
    ) -> Tocou {
        let Some(skin) = self.skin.clone() else {
            return Tocou::SemPersonagem;
        };
        let Some(tag) = animador::tag_da_reacao(&skin, reacao)
            .and_then(|i| skin.tags.get(i))
            .map(|t| t.nome.clone())
        else {
            return Tocou::Desconhecida {
                skin: skin.id.clone(),
            };
        };
        if !self.visivel {
            return Tocou::ForaDaTela {
                tag,
                motivo: "o pet está escondido (bin/pet mostrar)",
            };
        }
        let Some(ov) = ov else {
            return Tocou::ForaDaTela {
                tag,
                motivo: "sem compositor",
            };
        };
        if self.tocar(Some(ov), reacao, agora_ms) {
            Tocou::NaTela { tag }
        } else {
            Tocou::ForaDaTela {
                tag,
                motivo: "a janela do pet ainda não tem o personagem",
            }
        }
    }

    /// Confete pela tela inteira por `segundos`, a `fps` quadros por
    /// segundo; depois volta ao repouso. O confete anda na grade de arte do
    /// pet (múltiplos de D a partir do canto da célula).
    pub fn estresse(&mut self, ov: &mut dyn Overlay, fps: u32, segundos: u32, agora_ms: u64) {
        let Some(palco) = self.palco else {
            aviso!("debug: estresse pedido com o pet fora da tela");
            return;
        };
        // Numa janela pequena (M8) o confete pela tela inteira pede o palco
        // transitório, que ainda não existe.
        if !ov.capacidades().tela_inteira {
            aviso!(
                "debug: estresse pede uma janela do tamanho do monitor (o palco transitório chega no M8)"
            );
            return;
        }
        let grade = Grade {
            x: palco.x,
            y: palco.y,
            d: palco.d,
        };
        self.estresse = Some(Estresse {
            chuva: Chuva::nova(CONFETES, palco.tela, grade, LADO_CONFETE, SEMENTE_CONFETE),
            fps: fps.max(1) as u64,
            inicio_ms: agora_ms,
            passos: 0,
            fim_ms: agora_ms + segundos as u64 * 1000,
        });
        info!("debug: estresse com {CONFETES} confetes a {fps} fps por {segundos} s");
        self.desenhar(ov, agora_ms, false);
    }

    /// O ponto (x, y) do palco cai no corpo do pet (o clique e o arraste do
    /// M4 começam aqui, com o [`crate::plataforma::EventoPonteiro`] já no
    /// palco). `false` sem pet ou sem palco.
    pub fn acerta_o_pet(&self, x: i32, y: i32) -> bool {
        match (&self.pet, &self.palco) {
            (Some(pet), Some(palco)) => pet.acerta(palco, x, y),
            _ => false,
        }
    }

    /// Fim do processo: esconde o pet (quadro transparente) e larga a
    /// janela, esperando o sistema confirmar se havia uma.
    pub fn encerrar(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        let tinha = ov.fase() != Fase::Ausente;
        let passo = passo_de_visibilidade(ov.fase(), false);
        self.aplicar_passo(ov, passo, agora_ms);
        self.proximo_quadro = None;
        ov.encerrar(tinha);
    }

    // --- o que vai para fora -------------------------------------------------

    /// O painel do `/v1/estado`. Sem conexão (sem compositor), o painel
    /// vazio, só com o que os eventos do desktop contaram.
    pub fn painel(&mut self, punho: Option<&dyn Punho>, agora_ms: u64) -> Painel {
        let Some(punho) = punho else {
            return Painel {
                desktop: self.desktop.painel(Default::default(), Default::default()),
                ..Painel::default()
            };
        };
        let ov = punho.ver_janela();
        let desktop = punho.ver_desktop();
        let painel_desktop = self.desktop.painel(desktop.capacidades(), desktop.info());
        let info = ov.info();
        // Pelo relógio: com a tela apagada o último quadro do estresse pode
        // nunca ser desenhado, mas o estresse acabou do mesmo jeito.
        let estresse = self.estresse.as_ref().is_some_and(|e| agora_ms < e.fim_ms);
        let (d, sprite_disp) = match (&self.palco, &self.pet) {
            (Some(palco), Some(pet)) if info.visivel => (Some(palco.d), pet.sprite_disp(palco)),
            _ => (None, None),
        };
        Painel {
            monitor: info.monitor,
            escala: info.escala,
            d,
            sprite_disp,
            regiao_entrada: info.regiao,
            visivel: info.visivel,
            estresse,
            commits_por_min: self.commits.por_minuto(agora_ms),
            commits_total: self.commits.total,
            shm_bytes: info.shm_bytes,
            reacao: self
                .pet
                .as_ref()
                .and_then(|pet| pet.reacao(agora_ms))
                .map(str::to_owned),
            arrastando: self.arraste.arrastando(),
            viagem: self.seguir.fase().map(FaseViagem::nome),
            balao: self.balao(agora_ms).map(|b| b.linhas.clone()),
            soneca_restante_s: self
                .soneca(agora_ms)
                .map(|ate| (ate - agora_ms).div_ceil(1000)),
            desktop: painel_desktop,
        }
    }

    /// O sprite como está na tela agora, para a checagem de nitidez.
    pub fn quadro_esperado(&self, ov: &dyn Overlay) -> Option<QuadroEsperado> {
        let ultimo = ov.ultimo_quadro()?;
        let pet = self.pet.as_ref()?;
        let palco = self.palco.as_ref()?;
        let sprite = ultimo
            .cena
            .iter()
            .find(|e| matches!(e, Elemento::Sprite { .. }))?;
        let Elemento::Sprite { x, y, d, .. } = *sprite else {
            return None;
        };
        let area = pet.sprite_disp(palco)?;
        Some(QuadroEsperado {
            monitor: ultimo.monitor,
            area,
            grade: (x, y),
            d,
            seq: ultimo.seq,
            idade_ms: ultimo.idade_ms,
            rgba: cena::rgba_do_sprite(pet.skin(), sprite, area),
        })
    }
}

#[cfg(test)]
mod testes;
