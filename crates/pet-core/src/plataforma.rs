//! O que o Motor pede a cada sistema (decisão 0040, T8.0).
//!
//! O núcleo é um só; por sistema mudam dois contratos pequenos:
//!
//! - [`Overlay`]: a janela do bicho (no Wayland, a camada OVERLAY do tamanho
//!   do monitor; no Windows, no macOS e no X11, uma janela pequena que anda);
//! - [`Desktop`]: a ligação com o ambiente (monitor e janela ativos, focar a
//!   janela de uma sessão, "não perturbe").
//!
//! Tudo aqui é tipo simples, sem nada de Wayland, Win32 ou AppKit: o laço de
//! cada sistema traduz os eventos dele para estes tipos e chama o
//! [`crate::motor::Motor`]. O tempo anda em milissegundos de um relógio
//! monotônico que o laço injeta (os testes usam um relógio falso).
//!
//! A [`Caixa`] é o canal das outras threads (a entrada HTTP) para o laço:
//! um `mpsc` limitado mais um [`Despertador`] do sistema (no Linux, o `Ping`
//! do calloop), no lugar do canal próprio do calloop.
//!
//! # Coordenadas: o palco
//!
//! Tudo o que o Motor troca com a janela está no **palco**: pixels do
//! dispositivo do monitor onde o pet está, com a origem no canto superior
//! esquerdo desse monitor (decisão 0044). A cena ([`Elemento`]), a célula do
//! pet, a área de toque pedida em [`Overlay::desenhar`] e os eventos do
//! ponteiro ([`EventoPonteiro`]) usam sempre o palco, seja qual for a janela.
//! Cada sistema converte para a janela dele:
//!
//! - a camada do Wayland cobre o monitor inteiro ([`CapOverlay::tela_inteira`]):
//!   o palco é o próprio buffer, e a área de toque vira coordenadas lógicas da
//!   superfície (dividida pela escala, arredondada para fora);
//! - uma janela pequena que anda (Win32, AppKit, X11; M8) subtrai a própria
//!   origem no palco ao desenhar e soma ao contar o ponteiro, e só mostra o
//!   pedaço da cena que cabe nela.
//!
//! A posição do monitor no desktop ([`Monitor::origem`]) fica com a janela,
//! que precisa dela para se posicionar; o Motor não vê o desktop inteiro.

use std::sync::{Arc, mpsc};

use crate::cena::Elemento;
use crate::geometria::Ret;
use crate::skin::Skin;

/// Um monitor pronto para desenhar. É o que a janela diz ao Motor quando fica
/// pronta ou muda de monitor ou de escala.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Monitor {
    /// Nome que o sistema dá (`eDP-1`), se ele disser.
    pub nome: Option<String>,
    /// Descrição que o sistema dá (fabricante e modelo), se ele disser: é por
    /// ela que o M4 guarda a posição do pet em cada monitor.
    pub descricao: Option<String>,
    /// Tamanho lógico do monitor.
    pub logico: (u32, u32),
    /// Pixels do dispositivo por pixel lógico (1.5 no eDP-1 do Renan).
    pub escala: f64,
    /// Canto superior esquerdo do monitor no desktop, em pixels lógicos, se o
    /// sistema disser (as janelas pequenas do M8 se posicionam por ele).
    pub origem: Option<(i32, i32)>,
    /// A área útil, no palco: o monitor sem a barra de tarefas, o Dock ou os
    /// painéis que reservam espaço. `None`: o monitor inteiro. A camada do
    /// Wayland ignora as zonas exclusivas (`exclusive_zone -1`) e o
    /// compositor não diz a área útil, então lá é sempre o monitor inteiro.
    pub area_util: Option<Ret>,
}

impl Monitor {
    /// Tamanho do buffer em pixels do dispositivo: `round(w*s) x round(h*s)`.
    pub fn buffer(&self) -> (i32, i32) {
        (
            (self.logico.0 as f64 * self.escala).round() as i32,
            (self.logico.1 as f64 * self.escala).round() as i32,
        )
    }

    /// O palco inteiro: o monitor em pixels do dispositivo, na origem.
    pub fn palco(&self) -> Ret {
        let (w, h) = self.buffer();
        Ret::novo(0, 0, w, h)
    }

    /// A área útil no palco (o monitor inteiro se o sistema não disser),
    /// sempre dentro dele.
    pub fn area_util(&self) -> Ret {
        let palco = self.palco();
        self.area_util
            .and_then(|area| area.intersecao(&palco))
            .unwrap_or(palco)
    }
}

/// O que a janela do bicho sabe fazer neste sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapOverlay {
    /// A janela cobre o monitor inteiro e o pet anda dentro do buffer (a
    /// camada do Wayland, decisão 0005); sem isto, é uma janela pequena que
    /// anda, e o que passa da célula do pet (o confete do estresse, os voos
    /// do M6) espera o palco transitório do M8.
    pub tela_inteira: bool,
    /// Só a área de toque recebe clique; o resto atravessa.
    pub regiao_de_toque: bool,
    /// Escala fracionária em pixels do dispositivo (nitidez D×D, decisão 0004).
    pub escala_fracionaria: bool,
    /// Cursor próprio por cima do pet.
    pub cursor: bool,
    /// O sistema avisa quando o quadro foi mostrado (frame callback): um
    /// quadro em voo de cada vez, e nenhum com a tela apagada (decisão 0018).
    pub ritmo_do_compositor: bool,
}

/// O que a ligação com o desktop sabe fazer neste sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapDesktop {
    /// Sabe qual monitor está em foco (para o pet seguir).
    pub segue_foco: bool,
    /// Sabe qual janela está ativa (só um id, nunca o título).
    pub janela_ativa: bool,
    /// Sabe levar o foco a uma janela (o terminal de uma sessão, M4).
    pub foca_janela: bool,
    /// Sabe se o "não perturbe" está ligado.
    pub nao_perturbe: bool,
}

/// Uma janela que o desktop sabe focar. Opaca: o endereço no Hyprland, um
/// HWND no Windows, um id de app no macOS. Nunca carrega o título.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Alca(pub String);

/// Por que o desktop não focou uma janela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroFoco {
    /// Este desktop (ainda) não sabe focar janelas.
    NaoSuportado,
    /// A janela não existe mais.
    JanelaSumiu,
    /// O sistema recusou (macOS e Windows podem negar a ativação).
    Recusado(String),
}

/// O que o desktop conta ao Motor (decisão 0043): o monitor em foco, a
/// janela ativa (só um id opaco, nunca o título), a proteção de tela. No
/// Hyprland vem do socket de eventos (`.socket2.sock`, numa thread que só
/// lê) e da própria conexão Wayland; nenhum evento carrega título, classe ou
/// nome de área de trabalho.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventoDesktop {
    /// A fonte dos eventos do desktop ligou (`true`) ou caiu (`false`): o que
    /// aconteceu enquanto ela estava fora não chegou.
    Ligado(bool),
    /// O monitor em foco mudou (o nome que o sistema dá, `eDP-1`).
    MonitorEmFoco(String),
    /// A janela ativa mudou (`None`: nenhuma, uma área de trabalho vazia),
    /// com a hora em que o desktop contou, em ms desde 1970 (o mesmo relógio
    /// do `ts` dos hooks).
    JanelaAtiva {
        janela: Option<Alca>,
        parede_ms: u64,
    },
    /// A janela que já estava ativa quando a conexão começou (a semente do
    /// anel de ativações): só vale se nada mais novo chegou.
    JanelaInicial { janela: Alca, parede_ms: u64 },
    /// O título da janela em foco começa com um glifo do Claude Code (✳, ◐
    /// ou ◑): o Renan está olhando um terminal do Claude. Do título só sai
    /// este booleano.
    OlhandoClaude(bool),
    /// Uma janela abriu; `protetor`: é a proteção de tela do sistema.
    JanelaAbriu { janela: Alca, protetor: bool },
    /// Uma janela fechou.
    JanelaFechou(Alca),
    /// Um monitor entrou ou saiu.
    Monitores,
    /// O Renan parou de mexer no teclado e no mouse há um tempo (`true`) ou
    /// voltou a mexer (`false`; também o estado de partida de uma conexão
    /// que sabe contar). Sem isto, não se sabe (decisão 0062).
    Ocioso(bool),
    /// A tela começou (`true`) ou parou (`false`) de ser compartilhada (no
    /// Hyprland, o `screencast` do socket2). Depois de 2 s compartilhando,
    /// os balões perdem os nomes dos projetos (decisão 0076).
    Compartilhando(bool),
}

/// O que a ligação com o desktop mostra no `/v1/estado`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct InfoDesktop {
    /// Os protocolos do desktop ligados nesta conexão: os de focar janelas
    /// (no Wayland, `zwlr_foreign_toplevel_manager_v1` e
    /// `hyprland_toplevel_mapping_manager_v1`) e o que diz se o Renan está no
    /// teclado e no mouse (`ext_idle_notifier_v1`, decisão 0062).
    pub protocolos: Vec<String>,
    /// Janelas que ela sabe focar agora.
    pub janelas: usize,
}

/// A ligação com o ambiente: monitor e janela ativos, focar, discrição.
pub trait Desktop {
    fn capacidades(&self) -> CapDesktop;
    /// Leva o foco à janela `alvo` (o terminal de uma sessão).
    fn focar(&mut self, alvo: &Alca) -> Result<(), ErroFoco>;
    /// Os eventos acumulados desde a última vez, em ordem.
    fn eventos(&mut self) -> Vec<EventoDesktop> {
        Vec::new()
    }
    fn info(&self) -> InfoDesktop {
        InfoDesktop::default()
    }
    /// A janela ativa agora, pelo que a própria conexão sabe (no Wayland, o
    /// foreign-toplevel), se ela sabe: a semente do anel quando a fonte das
    /// trocas volta (decisão 0061).
    fn janela_ativa(&self) -> Option<Alca> {
        None
    }
}

/// O cursor por cima do pet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    /// A mão aberta: dá para pegar o pet.
    Pegar,
    /// A mão fechada: segurando o pet.
    Agarrar,
}

/// Uma conexão com o sistema: a janela do pet e a ligação com o desktop,
/// juntas (decisão 0043). No Wayland as duas usam a mesma conexão e o mesmo
/// `wl_seat`, e nascem e morrem juntas a cada reconexão; o Motor pede uma de
/// cada vez.
pub trait Punho {
    fn janela(&mut self) -> &mut dyn Overlay;
    fn desktop(&mut self) -> &mut dyn Desktop;
    /// Só para ler (o painel do `/v1/estado`, os prazos).
    fn ver_janela(&self) -> &dyn Overlay;
    fn ver_desktop(&self) -> &dyn Desktop;
}

/// Botão do ponteiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Botao {
    Esquerdo,
    Direito,
    Meio,
    Outro(u32),
}

/// O ponteiro sobre a janela do pet, em coordenadas do palco (pixels do
/// dispositivo do monitor; a janela converte as dela). As coordenadas nunca
/// vão para o log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventoPonteiro {
    Entrou { x: i32, y: i32 },
    Moveu { x: i32, y: i32 },
    Saiu,
    Apertou { botao: Botao, x: i32, y: i32 },
    Soltou { botao: Botao, x: i32, y: i32 },
}

/// Em que pé está a janela do pet, para decidir mostrar ou esconder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fase {
    /// Nenhuma janela.
    Ausente,
    /// Janela viva; `conteudo`: o sistema guarda pixels do pet nela.
    Viva { conteudo: bool },
    /// Escondendo: o quadro transparente já foi; a janela morre no próximo
    /// quadro mostrado ou num prazo curto.
    Saindo,
}

/// O que fazer para chegar à visibilidade pedida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Passo {
    Nada,
    /// Cria a janela (no Wayland, com output NULL: o monitor focado).
    Criar,
    /// Desiste de esconder: a mesma janela volta a desenhar o pet.
    Cancelar,
    /// Quadro transparente agora; destrói depois, para o fade de saída do
    /// compositor fotografar um quadro vazio (sem fantasma).
    ApagarEDestruir,
    /// Nada do pet no sistema (nunca desenhou): destrói já.
    Destruir,
}

/// Mostrar/esconder como função pura. Pedir para mostrar enquanto a janela
/// ainda está saindo **cancela** a saída: sem isso o pedido se perdia e a
/// janela morria logo depois, deixando o pet escondido (decisão 0018).
pub fn passo_de_visibilidade(fase: Fase, mostrar: bool) -> Passo {
    match (fase, mostrar) {
        (Fase::Ausente, true) => Passo::Criar,
        (Fase::Saindo, true) => Passo::Cancelar,
        (Fase::Viva { conteudo: true }, false) => Passo::ApagarEDestruir,
        (Fase::Viva { conteudo: false }, false) => Passo::Destruir,
        (Fase::Viva { .. }, true) | (Fase::Ausente | Fase::Saindo, false) => Passo::Nada,
    }
}

/// O que aconteceu num pedido de desenho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Desenho {
    /// Pixels novos foram para a tela (um commit).
    Enviado {
        retangulos: usize,
        area: i64,
    },
    /// Os pixels não mudaram, mas a área de toque sim: um commit só de
    /// estado.
    SoEstado,
    SemMudanca,
    /// Há um quadro em voo: desenha quando ele for mostrado
    /// ([`EventoOverlay::Redesenhar`]).
    Adiado,
}

/// O que a janela conta ao Motor, na ordem em que aconteceu.
#[derive(Debug, Clone, PartialEq)]
pub enum EventoOverlay {
    /// A janela ficou pronta para desenhar, ou mudou de monitor ou de
    /// escala: o palco (D e posição) é refeito com o [`Overlay::pronta`] de
    /// agora. O evento não carrega o monitor: dois na mesma leva (escala e
    /// `configure` juntos) desenhariam o primeiro com um palco que a janela
    /// já deixou para trás.
    Pronta,
    /// O quadro em voo foi mostrado e outro esperava: desenhe de novo.
    Redesenhar,
    /// O sistema fechou a janela (ou ela caiu num monitor que não serve): o
    /// palco acabou; uma nova vem com [`EventoOverlay::Recriar`].
    Sumiu,
    /// Hora de recriar a janela, se o pet deve aparecer.
    Recriar,
    /// A janela terminou de sair: escondida (quadro transparente mostrado,
    /// ou o prazo curto venceu) e destruída.
    Saiu,
    Ponteiro(EventoPonteiro),
}

/// O que a janela mostra no `/v1/estado`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfoOverlay {
    pub monitor: Option<String>,
    pub escala: Option<f64>,
    /// Área de toque pedida por último, nas coordenadas da janela no sistema
    /// dela (lógicas da superfície no Wayland): é o `regiao_entrada` do
    /// `/v1/estado`, que os scripts ao vivo comparam com o compositor.
    pub regiao: Option<Ret>,
    /// O pet está desenhado: o sistema guarda pixels dele e a janela não
    /// está saindo.
    pub visivel: bool,
    /// Memória compartilhada dos buffers.
    pub shm_bytes: usize,
}

/// O último quadro enviado, para a checagem de nitidez.
#[derive(Debug, Clone, PartialEq)]
pub struct UltimoQuadro {
    pub monitor: String,
    pub cena: Vec<Elemento>,
    /// Número do commit que pôs o quadro na tela.
    pub seq: u64,
    /// Há quanto tempo foi esse commit.
    pub idade_ms: u64,
}

/// A janela do bicho. Cada sistema implementa a sua; o Motor decide o que
/// desenhar e quando, e a janela só põe na tela.
pub trait Overlay {
    fn capacidades(&self) -> CapOverlay;
    fn fase(&self) -> Fase;
    /// O monitor onde a janela está pronta para desenhar, se estiver.
    fn pronta(&self) -> Option<Monitor>;
    /// Cria a janela no monitor ativo.
    fn criar(&mut self);
    /// Desiste de esconder: a janela fica.
    fn cancelar_saida(&mut self);
    /// Esconde sem fantasma: área de toque vazia, quadro transparente (com
    /// `skin`, a do último quadro, para o dano) e destruição logo depois.
    /// `true` se o quadro transparente fez um commit.
    fn apagar_e_destruir(&mut self, skin: &Skin) -> bool;
    /// Destrói já (nada do pet no sistema).
    fn destruir(&mut self);
    /// Desenha `cena` (se mudou) e pede a área de toque `toque` (`None`:
    /// nenhum clique é do pet), as duas no palco; a janela converte para as
    /// coordenadas dela. Com um quadro em voo, adia, a menos que `forcar`.
    fn desenhar(
        &mut self,
        cena: &[Elemento],
        skin: &Skin,
        toque: Option<Ret>,
        forcar: bool,
    ) -> Result<Desenho, String>;
    /// Esquece a cena desenhada: o próximo quadro redesenha tudo (troca de
    /// skin).
    fn esquecer_cena(&mut self);
    /// O cursor por cima do pet (pegar, ou agarrar enquanto segura). Uma
    /// janela sem cursor próprio ([`CapOverlay::cursor`]) ignora.
    fn cursor(&mut self, cursor: Cursor);
    fn info(&self) -> InfoOverlay;
    /// O último quadro na tela; `None` sem janela pronta, saindo ou sem
    /// quadro.
    fn ultimo_quadro(&self) -> Option<UltimoQuadro>;
    /// O próximo prazo interno da janela (ms no relógio do laço).
    fn proximo_prazo(&self) -> Option<u64>;
    /// Vence os prazos internos até `agora_ms`.
    fn vencer(&mut self, agora_ms: u64);
    /// Os eventos acumulados desde a última vez, em ordem.
    fn eventos(&mut self) -> Vec<EventoOverlay>;
    /// Fim do processo: larga o que sobrou e, com `confirmar`, espera (com
    /// prazo) o sistema confirmar que processou tudo.
    fn encerrar(&mut self, confirmar: bool);
}

/// Acorda o laço de eventos do sistema quando chega algo na [`Caixa`].
pub trait Despertador: Send + Sync {
    fn despertar(&self);
}

/// Um despertador que não faz nada (testes, ou um laço que só faz `recv`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SemDespertador;

impl Despertador for SemDespertador {
    fn despertar(&self) {}
}

/// Por que a [`Caixa`] não aceitou.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErroCaixa {
    /// Cheia: o laço está atrasado (a entrada HTTP responde 503).
    Cheia,
    /// O laço acabou.
    Fechada,
}

/// O lado de quem manda: um `mpsc` limitado que acorda o laço a cada
/// mensagem (e também quando está cheio, para o laço esvaziar).
pub struct Caixa<T> {
    remetente: mpsc::SyncSender<T>,
    despertador: Arc<dyn Despertador>,
}

impl<T> Clone for Caixa<T> {
    fn clone(&self) -> Self {
        Caixa {
            remetente: self.remetente.clone(),
            despertador: Arc::clone(&self.despertador),
        }
    }
}

impl<T> std::fmt::Debug for Caixa<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Caixa")
    }
}

impl<T> Caixa<T> {
    /// Uma caixa com `capacidade` vagas e o receptor que o laço esvazia.
    pub fn nova(
        capacidade: usize,
        despertador: Arc<dyn Despertador>,
    ) -> (Caixa<T>, mpsc::Receiver<T>) {
        let (remetente, receptor) = mpsc::sync_channel(capacidade);
        (
            Caixa {
                remetente,
                despertador,
            },
            receptor,
        )
    }

    /// Manda sem esperar.
    pub fn tentar(&self, valor: T) -> Result<(), ErroCaixa> {
        match self.remetente.try_send(valor) {
            Ok(()) => {
                self.despertador.despertar();
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.despertador.despertar();
                Err(ErroCaixa::Cheia)
            }
            Err(mpsc::TrySendError::Disconnected(_)) => Err(ErroCaixa::Fechada),
        }
    }

    /// Manda, esperando vaga se estiver cheia. Nunca no próprio laço.
    pub fn mandar(&self, valor: T) -> Result<(), ErroCaixa> {
        match self.remetente.try_send(valor) {
            Ok(()) => {
                self.despertador.despertar();
                Ok(())
            }
            Err(mpsc::TrySendError::Full(valor)) => {
                self.despertador.despertar();
                self.remetente.send(valor).map_err(|_| ErroCaixa::Fechada)?;
                self.despertador.despertar();
                Ok(())
            }
            Err(mpsc::TrySendError::Disconnected(_)) => Err(ErroCaixa::Fechada),
        }
    }
}

/// Uma janela de mentira para os testes: a do Motor aqui, a do núcleo do
/// daemon com a feature `teste` (só nos testes) e a do `bichinho simular`
/// com a feature `simulacao` (só o subcomando offline a usa; decisão 0078).
#[cfg(any(test, feature = "teste", feature = "simulacao"))]
pub mod falsa {
    use super::*;
    use crate::geometria::para_logico_por_fora;

    /// Um desktop de mentira: as janelas que existem, as capacidades e os
    /// focos que o Motor pediu.
    #[derive(Debug, Clone)]
    pub struct DesktopFalso {
        pub capacidades: CapDesktop,
        /// As janelas que dá para focar.
        pub janelas: Vec<Alca>,
        /// Os focos pedidos, em ordem.
        pub focos: Vec<Alca>,
        /// O que o desktop conta ao Motor no próximo [`Desktop::eventos`].
        pub eventos: Vec<EventoDesktop>,
        /// A janela que a conexão diz estar ativa ([`Desktop::janela_ativa`]).
        pub ativa: Option<Alca>,
    }

    impl Default for DesktopFalso {
        /// Como o Hyprland com o foreign-toplevel: segue o foco e foca.
        fn default() -> DesktopFalso {
            DesktopFalso {
                capacidades: CapDesktop {
                    segue_foco: true,
                    janela_ativa: true,
                    foca_janela: true,
                    nao_perturbe: false,
                },
                janelas: Vec::new(),
                focos: Vec::new(),
                eventos: Vec::new(),
                ativa: None,
            }
        }
    }

    impl Desktop for DesktopFalso {
        fn capacidades(&self) -> CapDesktop {
            self.capacidades
        }

        fn focar(&mut self, alvo: &Alca) -> Result<(), ErroFoco> {
            if !self.capacidades.foca_janela {
                return Err(ErroFoco::NaoSuportado);
            }
            if !self.janelas.contains(alvo) {
                return Err(ErroFoco::JanelaSumiu);
            }
            self.focos.push(alvo.clone());
            Ok(())
        }

        fn eventos(&mut self) -> Vec<EventoDesktop> {
            std::mem::take(&mut self.eventos)
        }

        fn info(&self) -> InfoDesktop {
            InfoDesktop {
                protocolos: vec!["falso".into()],
                janelas: self.janelas.len(),
            }
        }

        fn janela_ativa(&self) -> Option<Alca> {
            self.ativa.clone()
        }
    }

    /// Guarda o que o Motor pediu e imita a camada do Wayland: um quadro em
    /// voo por vez e a área de toque (pedida no palco) convertida para
    /// coordenadas lógicas no [`Overlay::info`]. Com
    /// [`JanelaFalsa::pequena`], imita uma janela pequena que anda (M8).
    /// Também é um [`Punho`], com um [`DesktopFalso`] junto.
    pub struct JanelaFalsa {
        pub capacidades: CapOverlay,
        pub fase: Option<Fase>,
        pub pronta: Option<Monitor>,
        pub em_voo: bool,
        pub cena: Option<Vec<Elemento>>,
        /// A área de toque pedida por último, no palco.
        pub toque: Option<Ret>,
        pub seq: u64,
        /// O cursor pedido por último.
        pub cursor: Option<Cursor>,
        /// O que o Motor pediu, em ordem (`criar`, `quadro 3`, `apagar com
        /// _teste`, …).
        pub pedidos: Vec<String>,
        /// O que a janela conta ao Motor no próximo [`Overlay::eventos`].
        pub eventos: Vec<EventoOverlay>,
        /// O desktop da mesma conexão.
        pub desktop: DesktopFalso,
    }

    impl Default for JanelaFalsa {
        /// Como a camada do Wayland: cobre o monitor e sabe tudo.
        fn default() -> JanelaFalsa {
            JanelaFalsa {
                capacidades: CapOverlay {
                    tela_inteira: true,
                    regiao_de_toque: true,
                    escala_fracionaria: true,
                    cursor: true,
                    ritmo_do_compositor: true,
                },
                fase: None,
                pronta: None,
                em_voo: false,
                cena: None,
                toque: None,
                seq: 0,
                cursor: None,
                pedidos: Vec::new(),
                eventos: Vec::new(),
                desktop: DesktopFalso::default(),
            }
        }
    }

    impl JanelaFalsa {
        /// Uma janela pequena que anda (Win32, AppKit, X11; M8): não cobre o
        /// monitor.
        pub fn pequena() -> JanelaFalsa {
            JanelaFalsa {
                capacidades: CapOverlay {
                    tela_inteira: false,
                    ..JanelaFalsa::default().capacidades
                },
                ..JanelaFalsa::default()
            }
        }

        pub fn fase_atual(&self) -> Fase {
            self.fase.unwrap_or(Fase::Ausente)
        }

        /// O compositor mostrou o quadro em voo.
        pub fn mostrou(&mut self) {
            self.em_voo = false;
        }

        /// Quantos quadros foram para a tela.
        pub fn quadros(&self) -> usize {
            self.pedidos
                .iter()
                .filter(|p| p.starts_with("quadro"))
                .count()
        }
    }

    impl Overlay for JanelaFalsa {
        fn capacidades(&self) -> CapOverlay {
            self.capacidades
        }

        fn fase(&self) -> Fase {
            self.fase_atual()
        }

        fn pronta(&self) -> Option<Monitor> {
            self.pronta.clone()
        }

        fn criar(&mut self) {
            self.pedidos.push("criar".into());
            self.fase = Some(Fase::Viva { conteudo: false });
            self.cena = None;
            self.em_voo = false;
        }

        fn cancelar_saida(&mut self) {
            self.pedidos.push("cancelar".into());
            self.fase = Some(Fase::Viva { conteudo: true });
        }

        fn apagar_e_destruir(&mut self, skin: &Skin) -> bool {
            self.pedidos.push(format!("apagar com {}", skin.id));
            self.fase = Some(Fase::Saindo);
            self.cena = Some(Vec::new());
            self.toque = None;
            true
        }

        fn destruir(&mut self) {
            self.pedidos.push("destruir".into());
            self.fase = None;
            self.cena = None;
        }

        fn desenhar(
            &mut self,
            cena: &[Elemento],
            _skin: &Skin,
            toque: Option<Ret>,
            forcar: bool,
        ) -> Result<Desenho, String> {
            if self.pronta.is_none() {
                return Err("janela ainda não está pronta".into());
            }
            let mudou_toque = self.toque != toque;
            self.toque = toque;
            if self.cena.as_deref() == Some(cena) {
                return Ok(if mudou_toque {
                    Desenho::SoEstado
                } else {
                    Desenho::SemMudanca
                });
            }
            if self.em_voo && !forcar {
                return Ok(Desenho::Adiado);
            }
            self.cena = Some(cena.to_vec());
            self.em_voo = true;
            self.seq += 1;
            self.fase = Some(Fase::Viva {
                conteudo: !cena.is_empty(),
            });
            self.pedidos.push(format!("quadro {}", self.seq));
            Ok(Desenho::Enviado {
                retangulos: 1,
                area: 0,
            })
        }

        fn esquecer_cena(&mut self) {
            self.pedidos.push("esquecer".into());
            self.cena = None;
        }

        fn cursor(&mut self, cursor: Cursor) {
            self.cursor = Some(cursor);
        }

        fn info(&self) -> InfoOverlay {
            let escala = self.pronta.as_ref().map(|m| m.escala);
            InfoOverlay {
                monitor: self.pronta.as_ref().and_then(|m| m.nome.clone()),
                escala,
                regiao: self
                    .toque
                    .map(|t| para_logico_por_fora(t, escala.unwrap_or(1.0))),
                visivel: self.fase_atual() == Fase::Viva { conteudo: true },
                shm_bytes: 9_216_000,
            }
        }

        fn ultimo_quadro(&self) -> Option<UltimoQuadro> {
            if !matches!(self.fase_atual(), Fase::Viva { .. }) {
                return None;
            }
            let pronta = self.pronta.as_ref()?;
            Some(UltimoQuadro {
                monitor: pronta.nome.clone().unwrap_or_default(),
                cena: self.cena.clone()?,
                seq: self.seq,
                idade_ms: 0,
            })
        }

        fn proximo_prazo(&self) -> Option<u64> {
            None
        }

        fn vencer(&mut self, _: u64) {}

        fn eventos(&mut self) -> Vec<EventoOverlay> {
            std::mem::take(&mut self.eventos)
        }

        fn encerrar(&mut self, confirmar: bool) {
            self.pedidos.push(format!("encerrar {confirmar}"));
            self.fase = None;
        }
    }

    impl Punho for JanelaFalsa {
        fn janela(&mut self) -> &mut dyn Overlay {
            self
        }

        fn desktop(&mut self) -> &mut dyn Desktop {
            &mut self.desktop
        }

        fn ver_janela(&self) -> &dyn Overlay {
            self
        }

        fn ver_desktop(&self) -> &dyn Desktop {
            &self.desktop
        }
    }
}

#[cfg(test)]
mod testes {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn mostrar_e_esconder_por_fase() {
        use Passo::*;
        let viva = |conteudo| Fase::Viva { conteudo };
        assert_eq!(passo_de_visibilidade(Fase::Ausente, true), Criar);
        assert_eq!(passo_de_visibilidade(Fase::Ausente, false), Nada);
        assert_eq!(passo_de_visibilidade(viva(true), true), Nada);
        assert_eq!(passo_de_visibilidade(viva(false), true), Nada);
        assert_eq!(passo_de_visibilidade(viva(true), false), ApagarEDestruir);
        assert_eq!(passo_de_visibilidade(viva(false), false), Destruir);
        assert_eq!(passo_de_visibilidade(Fase::Saindo, false), Nada);
    }

    #[test]
    fn mostrar_logo_depois_de_esconder_cancela_a_saida() {
        // A corrida da revisão do M1: esconder e mostrar em seguida (no mesmo
        // lote do canal ou antes do frame callback). Antes, o pedido de
        // mostrar não fazia nada e a camada morria em seguida: pet escondido.
        let fase = Fase::Viva { conteudo: true };
        assert_eq!(passo_de_visibilidade(fase, false), Passo::ApagarEDestruir);
        assert_eq!(passo_de_visibilidade(Fase::Saindo, true), Passo::Cancelar);
    }

    #[test]
    fn buffer_arredonda_no_4k_e_em_escala_quebrada() {
        let m = Monitor {
            logico: (2560, 1440),
            escala: 1.5,
            ..Monitor::default()
        };
        assert_eq!(m.buffer(), (3840, 2160));
        let q = Monitor {
            logico: (1093, 615),
            escala: 1.75,
            ..m
        };
        assert_eq!(q.buffer(), (1913, 1076));
    }

    #[test]
    fn area_util_e_o_monitor_inteiro_ou_a_parte_dele_que_o_sistema_diz() {
        let edp = Monitor {
            logico: (1280, 800),
            escala: 1.5,
            ..Monitor::default()
        };
        assert_eq!(edp.palco(), Ret::novo(0, 0, 1920, 1200));
        assert_eq!(edp.area_util(), edp.palco(), "sem área útil: o monitor");
        // Uma barra de tarefas de 48 lógicos embaixo (72 pixels a 1,5).
        let com_barra = Monitor {
            area_util: Some(Ret::novo(0, 0, 1920, 1128)),
            ..edp.clone()
        };
        assert_eq!(com_barra.area_util(), Ret::novo(0, 0, 1920, 1128));
        // O que passa do monitor é cortado; fora dele, vale o monitor.
        let torta = Monitor {
            area_util: Some(Ret::novo(-10, 0, 5000, 600)),
            ..edp.clone()
        };
        assert_eq!(torta.area_util(), Ret::novo(0, 0, 1920, 600));
        let fora = Monitor {
            area_util: Some(Ret::novo(3000, 0, 10, 10)),
            ..edp
        };
        assert_eq!(fora.area_util(), fora.palco());
    }

    #[derive(Default)]
    struct Contador(AtomicUsize);

    impl Despertador for Contador {
        fn despertar(&self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn caixa_acorda_o_laco_a_cada_mensagem_e_quando_enche() {
        let contador = Arc::new(Contador::default());
        let (caixa, receptor) = Caixa::nova(2, Arc::clone(&contador) as Arc<dyn Despertador>);
        assert_eq!(caixa.tentar(1), Ok(()));
        assert_eq!(caixa.clone().tentar(2), Ok(()));
        assert_eq!(caixa.tentar(3), Err(ErroCaixa::Cheia));
        assert_eq!(contador.0.load(Ordering::Relaxed), 3, "cheia também acorda");
        assert_eq!(receptor.try_recv(), Ok(1));
        assert_eq!(caixa.mandar(4), Ok(()));
        assert_eq!(receptor.try_iter().collect::<Vec<_>>(), vec![2, 4]);
        drop(receptor);
        assert_eq!(caixa.tentar(5), Err(ErroCaixa::Fechada));
        assert_eq!(caixa.mandar(6), Err(ErroCaixa::Fechada));
    }

    #[test]
    fn mandar_espera_vaga_quando_cheia() {
        let (caixa, receptor) = Caixa::nova(1, Arc::new(SemDespertador));
        caixa.tentar(1).unwrap();
        let outra = caixa.clone();
        let fio = std::thread::spawn(move || outra.mandar(2));
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(receptor.recv(), Ok(1));
        assert_eq!(fio.join().unwrap(), Ok(()));
        assert_eq!(receptor.recv(), Ok(2));
    }
}
