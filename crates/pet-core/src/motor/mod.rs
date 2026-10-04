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

mod pet;
mod ritmo;

use std::rc::Rc;

use serde::Serialize;

use crate::animador;
use crate::cena::{self, Elemento};
use crate::cerebro::{Agora, Cerebro, ConfigCerebro, Reacao, Resumo};
use crate::confete::{Chuva, Grade};
use crate::evento::Evento;
use crate::geometria::{Ret, Tamanho};
use crate::plataforma::{
    Desenho, EventoOverlay, Fase, Monitor, Overlay, Passo, passo_de_visibilidade,
};
use crate::skin::Skin;

pub use pet::{Palco, Pet};
pub use ritmo::{Commits, Estresse, JANELA_COMMITS_MS};

/// Confetes do teste de estresse, sempre com a mesma semente: medições
/// repetidas veem as mesmas trajetórias.
pub const CONFETES: usize = 40;
pub const SEMENTE_CONFETE: u64 = 7;
/// Lado de cada confete, em pixels de arte.
pub const LADO_CONFETE: i32 = 3;

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
        self.cerebro.receber(ev, recebido_ms, agora)
    }

    /// O prazo do cérebro venceu (acomodação do Stop, sessões que expiram).
    pub fn tique(&mut self, agora: Agora) -> Vec<Reacao> {
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

    /// O pet deve aparecer agora: há personagem e ninguém mandou esconder.
    pub fn quer_mostrar(&self) -> bool {
        self.visivel && self.pet.is_some()
    }

    /// Uma janela nova (o compositor conectou): o pet recomeça a animação
    /// agora, sem palco até a janela ficar pronta.
    pub fn conectou(&mut self, agora_ms: u64) {
        self.pet = self.skin.clone().map(|skin| Pet::novo(skin, agora_ms));
        self.palco = None;
        self.estresse = None;
        self.commits = Commits::default();
        self.proximo_quadro = None;
    }

    /// A janela acabou com a sessão (o compositor caiu).
    pub fn desconectou(&mut self) {
        self.pet = None;
        self.palco = None;
        self.estresse = None;
        self.commits = Commits::default();
        self.proximo_quadro = None;
    }

    /// Leva a janela à visibilidade pedida.
    pub fn aplicar_visibilidade(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
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

    /// Palco do pet num monitor pronto, com o log de onde ele ficou.
    fn montar_palco(&mut self, monitor: &Monitor) {
        let Some(pet) = &self.pet else {
            return;
        };
        let palco = pet.palco(monitor, self.tamanho);
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

    /// Um evento da janela.
    pub fn evento_overlay(&mut self, ov: &mut dyn Overlay, evento: EventoOverlay, agora_ms: u64) {
        match evento {
            // O palco sai do monitor de agora, não de quando o evento entrou
            // na fila: dois na mesma leva (escala e `configure` juntos) não
            // desenham com um palco velho no buffer novo.
            EventoOverlay::Pronta => {
                if self.pet.is_some()
                    && let Some(monitor) = ov.pronta()
                {
                    self.montar_palco(&monitor);
                    self.desenhar(ov, agora_ms, false);
                }
            }
            EventoOverlay::Redesenhar => self.desenhar(ov, agora_ms, false),
            EventoOverlay::Sumiu => {
                self.proximo_quadro = None;
                self.palco = None;
            }
            EventoOverlay::Recriar => {
                if self.quer_mostrar() && ov.fase() == Fase::Ausente {
                    ov.criar();
                    self.palco = None;
                }
            }
            // O painel é publicado depois de cada lote de eventos. O
            // ponteiro chega no M4 (arrastar e clicar; [`Motor::acerta_o_pet`]).
            EventoOverlay::Saiu | EventoOverlay::Ponteiro(_) => {}
        }
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
        let toque = pet.toque_no_palco(&palco);
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

    /// O próximo prazo do Motor (cérebro ou animação).
    pub fn proximo_prazo(&self) -> Option<u64> {
        [self.cerebro.proximo_prazo(), self.proximo_quadro]
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

    /// O painel do `/v1/estado`. Sem janela (sem compositor), o painel
    /// vazio.
    pub fn painel(&mut self, ov: Option<&dyn Overlay>, agora_ms: u64) -> Painel {
        let Some(ov) = ov else {
            return Painel::default();
        };
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
