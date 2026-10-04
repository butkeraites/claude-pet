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

use std::sync::{Arc, mpsc};

use crate::cena::Elemento;
use crate::geometria::Ret;
use crate::skin::Skin;

/// Um monitor pronto para desenhar: nome, tamanho lógico e escala. É o que a
/// janela diz ao Motor quando fica pronta ou muda de monitor ou de escala.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// Nome que o sistema dá (`eDP-1`), se ele disser.
    pub nome: Option<String>,
    /// Tamanho lógico da área de desenho (no Wayland, o do monitor).
    pub logico: (u32, u32),
    /// Pixels do dispositivo por pixel lógico (1.5 no eDP-1 do Renan).
    pub escala: f64,
}

impl Monitor {
    /// Tamanho do buffer em pixels do dispositivo: `round(w*s) x round(h*s)`.
    pub fn buffer(&self) -> (i32, i32) {
        (
            (self.logico.0 as f64 * self.escala).round() as i32,
            (self.logico.1 as f64 * self.escala).round() as i32,
        )
    }
}

/// O que a janela do bicho sabe fazer neste sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapOverlay {
    /// A janela cobre o monitor inteiro e o pet anda dentro do buffer (a
    /// camada do Wayland, decisão 0005); sem isto, é uma janela pequena que
    /// anda, com um palco transitório para voo e confete (M8).
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// A ligação com o ambiente: monitor e janela ativos, focar, discrição.
pub trait Desktop {
    fn capacidades(&self) -> CapDesktop;
    /// Leva o foco à janela `alvo` (o terminal de uma sessão).
    fn focar(&mut self, alvo: &Alca) -> Result<(), ErroFoco>;
}

/// Botão do ponteiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Botao {
    Esquerdo,
    Direito,
    Meio,
    Outro(u32),
}

/// O ponteiro sobre a janela do pet, em pixels do dispositivo relativos a
/// ela. As coordenadas nunca vão para o log.
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
    /// escala: o palco (D e posição) é refeito.
    Pronta(Monitor),
    /// O quadro em voo foi mostrado e outro esperava: desenhe de novo.
    Redesenhar,
    /// O sistema fechou a janela (ou ela caiu num monitor que não serve): o
    /// palco acabou; uma nova vem com [`EventoOverlay::Recriar`].
    Sumiu,
    /// Hora de recriar a janela, se o pet deve aparecer.
    Recriar,
    /// Algo do painel mudou (a janela terminou de sair).
    Mudou,
    Ponteiro(EventoPonteiro),
}

/// O que a janela mostra no `/v1/estado`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfoOverlay {
    pub monitor: Option<String>,
    pub escala: Option<f64>,
    /// Área de toque pedida por último, em coordenadas lógicas da janela.
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
    /// Desenha `cena` (se mudou) e pede a área de toque `regiao`, em
    /// coordenadas lógicas da janela. Com um quadro em voo, adia, a menos que
    /// `forcar`.
    fn desenhar(
        &mut self,
        cena: &[Elemento],
        skin: &Skin,
        regiao: Option<Ret>,
        forcar: bool,
    ) -> Result<Desenho, String>;
    /// Esquece a cena desenhada: o próximo quadro redesenha tudo (troca de
    /// skin).
    fn esquecer_cena(&mut self);
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
            nome: None,
            logico: (2560, 1440),
            escala: 1.5,
        };
        assert_eq!(m.buffer(), (3840, 2160));
        let q = Monitor {
            logico: (1093, 615),
            escala: 1.75,
            ..m
        };
        assert_eq!(q.buffer(), (1913, 1076));
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
