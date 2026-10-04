//! Sessão Wayland: uma conexão com o compositor, do handshake ao EOF, e a
//! camada OVERLAY do pet como [`Overlay`] (decisões 0004, 0005 e 0040).
//!
//! A sessão só existe enquanto o compositor responde; qualquer erro de
//! protocolo ou EOF derruba tudo e o laço volta a esperar (decisão 0007). Por
//! isso nada aqui tenta se recuperar da conexão sozinho: quem decide
//! reconectar é o laço do daemon. O que a sessão recupera sozinha é a própria
//! camada (`closed` → recria).
//!
//! A sessão não sabe o que o pet faz. Ela põe na tela a cena que o
//! [`pet_core::motor::Motor`] manda e conta o que aconteceu como
//! [`EventoOverlay`] (camada pronta, quadro mostrado, camada fechada); os
//! prazos dela (destruir depois de esconder, reservas de escala e de `enter`,
//! recriar depois de um `closed`) vencem pelo [`Overlay::vencer`], no relógio
//! do laço.
//!
//! A mesma sessão é o [`Desktop`] do Wayland (decisão 0043): a janela e a
//! ligação com o desktop usam a mesma conexão e o mesmo `wl_seat`, e nascem e
//! morrem juntas a cada reconexão. Juntas, são o [`Punho`] que o laço entrega
//! ao núcleo.

use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use pet_core::cena::Elemento;
use pet_core::geometria::{self, Ret};
use pet_core::plataforma::{
    Alca, Botao, CapDesktop, CapOverlay, Cursor, Desenho, Desktop, ErroFoco, EventoOverlay,
    EventoPonteiro, Fase, InfoOverlay, Monitor, Overlay, Punho, UltimoQuadro,
};
use pet_core::skin::Skin;

use smithay_client_toolkit as sctk;
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::globals::{GlobalList, registry_queue_init};
use smithay_client_toolkit::reexports::client::protocol::{
    wl_output, wl_pointer, wl_seat, wl_surface,
};
use smithay_client_toolkit::reexports::client::{
    Connection, Dispatch, EventQueue, QueueHandle, delegate_noop,
};
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::{
    self, WpFractionalScaleV1,
};
use smithay_client_toolkit::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::{
    Shape, WpCursorShapeDeviceV1,
};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::pointer::cursor_shape::CursorShapeManager;
use smithay_client_toolkit::seat::pointer::{
    BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, PointerEvent, PointerEventKind, PointerHandler,
};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_registry, registry_handlers};

use crate::saida;
use crate::sincronia;
use crate::superficie::{self, OrigemEscala, Prazo, Superficie};

/// Globais sem os quais não há camada para desenhar.
const OBRIGATORIOS: &[&str] = &[
    "wl_compositor",
    "wl_shm",
    "zwlr_layer_shell_v1",
    "wp_viewporter",
];
/// Globais que melhoram o resultado; a falta deles vira aviso.
const OPCIONAIS: &[&str] = &[
    "wp_fractional_scale_manager_v1",
    "wl_seat",
    "wp_cursor_shape_manager_v1",
];
/// Depois de um `closed`, espera o monitor assentar antes de recriar.
const RECRIAR_APOS_FECHAR: Duration = Duration::from_millis(250);
/// Ao esconder, a camada morre no frame callback do quadro transparente ou
/// depois disto, o que vier primeiro.
const ESPERA_DESTRUIR: Duration = Duration::from_millis(50);
/// Prazo para o compositor responder antes do handshake.
const LIMITE_HANDSHAKE: Duration = Duration::from_secs(3);
/// Prazo para o compositor confirmar o quadro transparente ao sair (o
/// `stop_grace_period` do compose é de 5 s).
const LIMITE_SAIDA: Duration = Duration::from_secs(1);

/// Um prazo interno da camada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tarefa {
    /// A camada que estava saindo morre (se ainda for a mesma geração).
    Destruir,
    /// Reserva da escala e do `enter` (se ainda for a mesma geração).
    Reserva(Prazo),
    /// Hora de recriar a camada depois de um `closed`.
    Recriar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Agendado {
    quando_ms: u64,
    /// A geração da camada a que o prazo se refere (`None`: qualquer).
    geracao: Option<u64>,
    tarefa: Tarefa,
}

impl Agendado {
    /// O prazo ainda vale para a camada `atual`: um prazo de uma camada que
    /// já morreu (outra geração, ou nenhuma) se cala.
    fn vale_para(&self, atual: Option<u64>) -> bool {
        self.geracao.is_none() || self.geracao == atual
    }
}

/// Os prazos internos da camada, no relógio do laço (ms): destruir depois
/// de esconder, as reservas de escala e de `enter`, recriar depois de um
/// `closed`. Pura, para testar sem compositor; a geração de cada prazo é
/// conferida na hora de cumprir ([`Agendado::vale_para`]), porque cumprir um
/// pode matar a camada dos seguintes.
#[derive(Debug, Default)]
struct Agenda {
    itens: Vec<Agendado>,
}

impl Agenda {
    fn agendar(&mut self, quando_ms: u64, geracao: Option<u64>, tarefa: Tarefa) {
        self.itens.push(Agendado {
            quando_ms,
            geracao,
            tarefa,
        });
    }

    fn proximo(&self) -> Option<u64> {
        self.itens.iter().map(|a| a.quando_ms).min()
    }

    /// Tira os prazos vencidos até `agora_ms`, na ordem em que vencem (no
    /// empate, na ordem em que foram marcados, como os timers do calloop).
    fn vencidos(&mut self, agora_ms: u64) -> Vec<Agendado> {
        let (mut vencidos, resto): (Vec<Agendado>, Vec<Agendado>) = std::mem::take(&mut self.itens)
            .into_iter()
            .partition(|a| a.quando_ms <= agora_ms);
        self.itens = resto;
        vencidos.sort_by_key(|a| a.quando_ms);
        vencidos
    }

    fn limpar(&mut self) {
        self.itens.clear();
    }
}

/// Estado da fila de eventos Wayland.
pub struct Sessao {
    registro: RegistryState,
    saidas: OutputState,
    compositor: CompositorState,
    shm: Shm,
    camadas: LayerShell,
    fracional: Option<WpFractionalScaleManagerV1>,
    viewporter: WpViewporter,
    qh: QueueHandle<Sessao>,
    conexao: Connection,
    superficie: Option<Superficie>,
    /// Origem do relógio do laço: os prazos internos andam nele.
    inicio: Instant,
    agenda: Agenda,
    eventos: Vec<EventoOverlay>,
    assentos: SeatState,
    ponteiro: Option<wl_pointer::WlPointer>,
    cursores: Option<CursorShapeManager>,
    forma_do_cursor: Option<WpCursorShapeDeviceV1>,
    /// O serial do último `enter` do ponteiro na camada: o cursor-shape só
    /// aceita trocar o cursor com ele.
    serial_enter: Option<u32>,
}

/// Resultado do handshake: a conexão, a fila e o estado que ela despacha.
pub struct Conexao {
    pub conexao: Connection,
    pub fila: EventQueue<Sessao>,
    pub sessao: Sessao,
}

/// Faz o handshake numa conexão já aberta (a da prova da descoberta).
/// `inicio` é a origem do relógio do laço.
pub fn conectar(fluxo: UnixStream, inicio: Instant) -> Result<Conexao, String> {
    let conexao = Connection::from_socket(fluxo).map_err(|e| format!("conexão Wayland: {e}"))?;
    // O registro abaixo faz um roundtrip sem prazo; antes, prova com prazo
    // que o compositor está lendo este socket.
    sincronia::sincronizar(&conexao, LIMITE_HANDSHAKE)?;
    let (globais, fila) =
        registry_queue_init::<Sessao>(&conexao).map_err(|e| format!("registro Wayland: {e}"))?;
    let faltando: Vec<&str> = OBRIGATORIOS
        .iter()
        .copied()
        .filter(|nome| !tem_global(&globais, nome))
        .collect();
    if !faltando.is_empty() {
        return Err(format!("o compositor não oferece {}", faltando.join(", ")));
    }
    for nome in OPCIONAIS {
        if !tem_global(&globais, nome) {
            aviso!("o compositor não oferece {nome}; seguindo sem");
        }
    }
    let qh = fila.handle();
    let falhou = |nome: &str, e: &dyn std::fmt::Display| format!("{nome}: {e}");
    let compositor =
        CompositorState::bind(&globais, &qh).map_err(|e| falhou("wl_compositor", &e))?;
    let shm = Shm::bind(&globais, &qh).map_err(|e| falhou("wl_shm", &e))?;
    let camadas = LayerShell::bind(&globais, &qh).map_err(|e| falhou("layer shell", &e))?;
    let viewporter = globais
        .bind::<WpViewporter, _, _>(&qh, 1..=1, ())
        .map_err(|e| falhou("wp_viewporter", &e))?;
    let fracional = globais
        .bind::<WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ())
        .ok();
    let assentos = SeatState::new(&globais, &qh);
    let cursores = CursorShapeManager::bind(&globais, &qh).ok();
    let sessao = Sessao {
        registro: RegistryState::new(&globais),
        saidas: OutputState::new(&globais, &qh),
        compositor,
        shm,
        camadas,
        fracional,
        viewporter,
        qh,
        conexao: conexao.clone(),
        superficie: None,
        inicio,
        agenda: Agenda::default(),
        eventos: Vec::new(),
        assentos,
        ponteiro: None,
        cursores,
        forma_do_cursor: None,
        serial_enter: None,
    };
    Ok(Conexao {
        conexao,
        fila,
        sessao,
    })
}

fn tem_global(globais: &GlobalList, interface: &str) -> bool {
    globais
        .contents()
        .with_list(|lista| lista.iter().any(|g| g.interface == interface))
}

/// Resumo de uma saída para log: nome, tamanho lógico e modo atual.
fn descrever_saida(saidas: &OutputState, saida: &wl_output::WlOutput) -> String {
    match saida::monitor(saidas, saida) {
        Some(m) => {
            let modo = m
                .modo
                .map(|(w, h)| format!("{w}x{h}"))
                .unwrap_or_else(|| "?".into());
            format!(
                "{} (lógico {}x{}, modo {modo})",
                m.nome, m.logico.0, m.logico.1
            )
        }
        None => "saída sem informação".into(),
    }
}

/// O botão do Linux (`input-event-codes.h`) como o Motor o vê.
fn botao(codigo: u32) -> Botao {
    match codigo {
        BTN_LEFT => Botao::Esquerdo,
        BTN_RIGHT => Botao::Direito,
        BTN_MIDDLE => Botao::Meio,
        outro => Botao::Outro(outro),
    }
}

impl Sessao {
    fn agora_ms(&self) -> u64 {
        self.inicio.elapsed().as_millis() as u64
    }

    /// Marca um prazo interno daqui a `daqui_a`.
    fn agendar(&mut self, daqui_a: Duration, geracao: Option<u64>, tarefa: Tarefa) {
        let quando_ms = self.agora_ms() + daqui_a.as_millis() as u64;
        self.agenda.agendar(quando_ms, geracao, tarefa);
    }

    fn geracao(&self) -> Option<u64> {
        self.superficie.as_ref().map(|s| s.geracao)
    }

    /// Recalcula tamanho, escala e monitor da camada; quando mudam, conta ao
    /// Motor ([`EventoOverlay::Pronta`]), que refaz o palco com o estado de
    /// agora ([`Overlay::pronta`]) e desenha.
    fn tentar_aprontar(&mut self) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        let Some(pronta) = superficie.resolver(&self.saidas) else {
            return;
        };
        let (largura, altura) = pronta.buffer();
        let origem = match pronta.origem {
            OrigemEscala::Fracionaria => "preferred_scale",
            OrigemEscala::Modo => "modo do monitor",
            OrigemEscala::Padrao => "padrão 1.0, sem informação do compositor",
        };
        info!(
            "camada pronta em {}: {}x{} lógicos, escala {} ({origem}), buffer {largura}x{altura}",
            pronta.monitor.as_deref().unwrap_or("monitor desconhecido"),
            pronta.logico.0,
            pronta.logico.1,
            pronta.escala
        );
        self.eventos.push(EventoOverlay::Pronta);
    }

    /// Larga a camada atual e agenda outra (output NULL: o monitor focado)
    /// para daqui a [`RECRIAR_APOS_FECHAR`], para o monitor assentar. Usado no
    /// `closed` e quando aparece um monitor de verdade para uma camada que
    /// caiu no FALLBACK. A camada largada aqui nunca tem pixels do pet (o
    /// compositor já a fechou, ou ela nunca desenhou).
    fn recriar_depois(&mut self) {
        self.superficie = None;
        self.eventos.push(EventoOverlay::Sumiu);
        self.agendar(RECRIAR_APOS_FECHAR, None, Tarefa::Recriar);
    }

    fn escala_preferida(&mut self, objeto: &WpFractionalScaleV1, escala_120: u32) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        if superficie.eh_fracional(objeto) {
            depurar!("preferred_scale {escala_120}/120");
            superficie.escala_preferida(escala_120);
            self.tentar_aprontar();
        }
    }

    fn executar(&mut self, agendado: Agendado) {
        if !agendado.vale_para(self.geracao()) {
            return;
        }
        match agendado.tarefa {
            Tarefa::Destruir => {
                if self.superficie.as_ref().is_some_and(|s| s.saindo) {
                    self.superficie = None;
                    self.eventos.push(EventoOverlay::Saiu);
                }
            }
            Tarefa::Reserva(prazo) => {
                if let Some(superficie) = self.superficie.as_mut() {
                    if superficie.pronta().is_none() {
                        depurar!("prazo de reserva vencido: {prazo:?}");
                    }
                    superficie.venceu(prazo);
                }
                self.tentar_aprontar();
            }
            Tarefa::Recriar => self.eventos.push(EventoOverlay::Recriar),
        }
    }
}

impl Overlay for Sessao {
    fn capacidades(&self) -> CapOverlay {
        CapOverlay {
            tela_inteira: true,
            regiao_de_toque: true,
            escala_fracionaria: self.fracional.is_some(),
            cursor: self.cursores.is_some(),
            ritmo_do_compositor: true,
        }
    }

    fn fase(&self) -> Fase {
        self.superficie
            .as_ref()
            .map_or(Fase::Ausente, Superficie::fase)
    }

    fn pronta(&self) -> Option<Monitor> {
        self.superficie
            .as_ref()
            .and_then(Superficie::pronta)
            .map(superficie::Pronta::monitor)
    }

    fn criar(&mut self) {
        let superficie = Superficie::criar(
            &self.compositor,
            &self.camadas,
            self.fracional.as_ref(),
            &self.viewporter,
            &self.qh,
        );
        depurar!("camada criada (geração {})", superficie.geracao);
        self.superficie = Some(superficie);
    }

    fn cancelar_saida(&mut self) {
        if let Some(superficie) = self.superficie.as_mut() {
            depurar!("mostrar durante a saída: a camada fica");
            superficie.saindo = false;
        }
    }

    /// Esconde uma camada com pixels do pet: região de input vazia, quadro
    /// transparente com commit e destruição no frame callback seguinte (ou
    /// em [`ESPERA_DESTRUIR`]). Funciona também logo depois de uma troca de
    /// escala, quando o buffer antigo já foi descartado: o quadro vazio vai
    /// num buffer novo, zerado, com dano na tela inteira.
    fn apagar_e_destruir(&mut self, skin: &Skin) -> bool {
        let Some(superficie) = self.superficie.as_mut() else {
            return false;
        };
        superficie.saindo = true;
        if let Err(e) = superficie.definir_regiao(None, &self.compositor) {
            aviso!("região de input ao esconder: {e}");
        }
        let geracao = superficie.geracao;
        let commit = match superficie.desenhar(&[], skin, &self.shm, &self.qh, true) {
            Ok(Desenho::Enviado { .. }) => true,
            Ok(_) => false,
            Err(e) => {
                aviso!("quadro transparente ao esconder: {e}");
                self.superficie = None;
                return false;
            }
        };
        self.agendar(ESPERA_DESTRUIR, Some(geracao), Tarefa::Destruir);
        commit
    }

    fn destruir(&mut self) {
        self.superficie = None;
    }

    /// A cena e o toque chegam no palco; o palco é o buffer da camada (ela
    /// cobre o monitor), e o toque vira a região de input em coordenadas
    /// lógicas da superfície, arredondada para fora.
    fn desenhar(
        &mut self,
        cena: &[Elemento],
        skin: &Skin,
        toque: Option<Ret>,
        forcar: bool,
    ) -> Result<Desenho, String> {
        let Some(superficie) = self.superficie.as_mut() else {
            return Err("sem camada".into());
        };
        let Some(escala) = superficie.pronta().map(|p| p.escala) else {
            return Err("camada ainda não está pronta".into());
        };
        let regiao = toque.map(|t| geometria::para_logico_por_fora(t, escala));
        let mudou_regiao = superficie
            .definir_regiao(regiao, &self.compositor)
            .unwrap_or_else(|e| {
                aviso!("região de input: {e}");
                false
            });
        match superficie.desenhar(cena, skin, &self.shm, &self.qh, forcar)? {
            Desenho::SemMudanca if mudou_regiao => {
                superficie.commit_de_estado();
                Ok(Desenho::SoEstado)
            }
            outro => Ok(outro),
        }
    }

    fn esquecer_cena(&mut self) {
        if let Some(superficie) = self.superficie.as_mut() {
            superficie.esquecer_cena();
        }
    }

    /// `grab` e `grabbing` do cursor-shape-v1, com o serial do último
    /// `enter` (sem ele, ou sem o protocolo, o cursor fica como está).
    fn cursor(&mut self, cursor: Cursor) {
        let (Some(forma), Some(serial)) = (&self.forma_do_cursor, self.serial_enter) else {
            return;
        };
        forma.set_shape(
            serial,
            match cursor {
                Cursor::Pegar => Shape::Grab,
                Cursor::Agarrar => Shape::Grabbing,
            },
        );
    }

    fn info(&self) -> InfoOverlay {
        let superficie = self.superficie.as_ref();
        let pronta = superficie.and_then(Superficie::pronta);
        InfoOverlay {
            monitor: pronta.and_then(|p| p.monitor.clone()),
            escala: pronta.map(|p| p.escala),
            regiao: superficie.and_then(Superficie::regiao),
            visivel: superficie.is_some_and(|s| s.tem_conteudo() && !s.saindo),
            shm_bytes: superficie.map_or(0, Superficie::bytes_shm),
        }
    }

    fn ultimo_quadro(&self) -> Option<UltimoQuadro> {
        let superficie = self.superficie.as_ref()?;
        if superficie.saindo {
            return None;
        }
        let pronta = superficie.pronta()?;
        let cena = superficie.cena_atual()?.to_vec();
        let (seq, idade) = superficie.ultimo_quadro();
        Some(UltimoQuadro {
            monitor: pronta.monitor.clone().unwrap_or_default(),
            cena,
            seq,
            idade_ms: idade.map_or(0, |t| t.as_millis() as u64),
        })
    }

    fn proximo_prazo(&self) -> Option<u64> {
        self.agenda.proximo()
    }

    fn vencer(&mut self, agora_ms: u64) {
        for agendado in self.agenda.vencidos(agora_ms) {
            self.executar(agendado);
        }
    }

    fn eventos(&mut self) -> Vec<EventoOverlay> {
        std::mem::take(&mut self.eventos)
    }

    /// Antes de sair do processo: larga a camada e, se havia uma, faz uma ida
    /// e volta com prazo, para garantir que o compositor processou o quadro
    /// transparente e a destruição antes de o socket fechar (o
    /// libwayland-server descarta o que não leu quando o cliente desliga).
    fn encerrar(&mut self, confirmar: bool) {
        self.superficie = None;
        self.agenda.limpar();
        if !confirmar {
            return;
        }
        match sincronia::sincronizar(&self.conexao, LIMITE_SAIDA) {
            Ok(()) => depurar!("o compositor processou o quadro transparente e a destruição"),
            Err(e) => aviso!("saindo sem confirmação do compositor: {e}"),
        }
    }
}

/// O desktop do Wayland nesta conexão. Focar janelas pelo foreign-toplevel
/// chega na T4.9 (decisão 0039); por enquanto, nenhuma capacidade.
impl Desktop for Sessao {
    fn capacidades(&self) -> CapDesktop {
        CapDesktop::default()
    }

    fn focar(&mut self, _: &Alca) -> Result<(), ErroFoco> {
        Err(ErroFoco::NaoSuportado)
    }
}

/// A janela e o desktop da mesma conexão (decisão 0043).
impl Punho for Sessao {
    fn janela(&mut self) -> &mut dyn Overlay {
        self
    }

    fn desktop(&mut self) -> &mut dyn Desktop {
        self
    }

    fn ver_janela(&self) -> &dyn Overlay {
        self
    }

    fn ver_desktop(&self) -> &dyn Desktop {
        self
    }
}

impl CompositorHandler for Sessao {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        fator: i32,
    ) {
        // A escala inteira (ceil(1.5) = 2) não serve para pixel art; quem
        // manda é a fracionária.
        depurar!("escala inteira sugerida: {fator} (ignorada)");
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        if !superficie.eh(surface) {
            return;
        }
        superficie.quadro_mostrado();
        if superficie.saindo {
            self.superficie = None;
            self.eventos.push(EventoOverlay::Saiu);
        } else if superficie.pendente {
            self.eventos.push(EventoOverlay::Redesenhar);
        }
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        saida: &wl_output::WlOutput,
    ) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        if superficie.eh(surface) {
            depurar!("enter em {}", descrever_saida(&self.saidas, saida));
            superficie.entrou(saida);
            self.tentar_aprontar();
        }
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for Sessao {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, camada: &LayerSurface) {
        let nossa = self
            .superficie
            .as_ref()
            .is_some_and(|s| &s.camada == camada);
        if !nossa {
            return;
        }
        info!(
            "a camada foi fechada pelo compositor; recriando em {} ms",
            RECRIAR_APOS_FECHAR.as_millis()
        );
        self.recriar_depois();
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        camada: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let Some(superficie) = self.superficie.as_mut() else {
            return;
        };
        if &superficie.camada != camada {
            return;
        }
        let (largura, altura) = configure.new_size;
        depurar!("configure {largura}x{altura}");
        match superficie.configurar(largura, altura, &self.compositor, &self.shm) {
            Ok(true) => {
                let geracao = superficie.geracao;
                self.agendar(
                    superficie::PRAZO_ESCALA,
                    Some(geracao),
                    Tarefa::Reserva(Prazo::Escala),
                );
                self.agendar(
                    superficie::PRAZO_ENTER,
                    Some(geracao),
                    Tarefa::Reserva(Prazo::Enter),
                );
            }
            Ok(false) => self.tentar_aprontar(),
            Err(motivo) => aviso!("configure da camada: {motivo}"),
        }
    }
}

impl ShmHandler for Sessao {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl OutputHandler for Sessao {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.saidas
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, saida: wl_output::WlOutput) {
        info!("monitor: {}", descrever_saida(&self.saidas, &saida));
        let utilizavel = saida::monitor(&self.saidas, &saida).is_some_and(|m| m.utilizavel());
        let sem_casa = self.superficie.as_ref().is_some_and(Superficie::sem_casa);
        if utilizavel && sem_casa {
            info!(
                "apareceu um monitor de verdade; recriando a camada em {} ms",
                RECRIAR_APOS_FECHAR.as_millis()
            );
            self.recriar_depois();
        }
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, saida: wl_output::WlOutput) {
        depurar!("monitor mudou: {}", descrever_saida(&self.saidas, &saida));
    }

    fn output_destroyed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        saida: wl_output::WlOutput,
    ) {
        info!("monitor saiu: {}", descrever_saida(&self.saidas, &saida));
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for Sessao {
    fn event(
        sessao: &mut Self,
        objeto: &WpFractionalScaleV1,
        evento: wp_fractional_scale_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = evento {
            sessao.escala_preferida(objeto, scale);
        }
    }
}

// Sem eventos: o gerente da escala fracionária, o viewporter e o viewport.
delegate_noop!(Sessao: WpFractionalScaleManagerV1);
delegate_noop!(Sessao: WpViewporter);
delegate_noop!(Sessao: WpViewport);

impl ProvidesRegistryState for Sessao {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registro
    }
    registry_handlers![OutputState, SeatState];
}

delegate_registry!(Sessao);
sctk::delegate_dispatch2!(Sessao);

impl SeatHandler for Sessao {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.assentos
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        assento: wl_seat::WlSeat,
        capacidade: Capability,
    ) {
        if capacidade != Capability::Pointer || self.ponteiro.is_some() {
            return;
        }
        match self.assentos.get_pointer(qh, &assento) {
            Ok(ponteiro) => {
                self.forma_do_cursor = self
                    .cursores
                    .as_ref()
                    .map(|c| c.get_shape_device(&ponteiro, qh));
                self.ponteiro = Some(ponteiro);
            }
            Err(e) => aviso!("ponteiro: {e}"),
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capacidade: Capability,
    ) {
        if capacidade == Capability::Pointer {
            if let Some(forma) = self.forma_do_cursor.take() {
                forma.destroy();
            }
            if let Some(ponteiro) = self.ponteiro.take() {
                ponteiro.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for Sessao {
    /// O ponteiro troca o cursor para "pegar" em cima do pet e vai ao Motor
    /// no palco: a camada cobre o monitor, então é a posição na superfície
    /// vezes a escala (o arraste e o clique chegam no M4). Movimentos
    /// seguidos viram um só, o último: passar o mouse por cima do pet não
    /// enche a fila. Coordenadas nunca vão para o log.
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        eventos: &[PointerEvent],
    ) {
        for evento in eventos {
            let Some(superficie) = self.superficie.as_ref() else {
                return;
            };
            if !superficie.eh(&evento.surface) {
                continue;
            }
            let escala = superficie.pronta().map_or(1.0, |p| p.escala);
            let x = (evento.position.0 * escala).round() as i32;
            let y = (evento.position.1 * escala).round() as i32;
            let ponteiro = match &evento.kind {
                PointerEventKind::Enter { serial } => {
                    self.serial_enter = Some(*serial);
                    if let Some(forma) = &self.forma_do_cursor {
                        forma.set_shape(*serial, Shape::Grab);
                    }
                    EventoPonteiro::Entrou { x, y }
                }
                PointerEventKind::Leave { .. } => EventoPonteiro::Saiu,
                PointerEventKind::Motion { .. } => EventoPonteiro::Moveu { x, y },
                PointerEventKind::Press { button, .. } => EventoPonteiro::Apertou {
                    botao: botao(*button),
                    x,
                    y,
                },
                PointerEventKind::Release { button, .. } => EventoPonteiro::Soltou {
                    botao: botao(*button),
                    x,
                    y,
                },
                PointerEventKind::Axis { .. } => continue,
            };
            empilhar_ponteiro(&mut self.eventos, ponteiro);
        }
    }
}

/// Põe um evento do ponteiro na fila; um `Moveu` logo depois de outro o
/// substitui.
fn empilhar_ponteiro(eventos: &mut Vec<EventoOverlay>, ponteiro: EventoPonteiro) {
    if let (
        EventoPonteiro::Moveu { .. },
        Some(EventoOverlay::Ponteiro(ultimo @ EventoPonteiro::Moveu { .. })),
    ) = (ponteiro, eventos.last_mut())
    {
        *ultimo = ponteiro;
        return;
    }
    eventos.push(EventoOverlay::Ponteiro(ponteiro));
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn agenda_vence_em_ordem_e_guarda_o_resto() {
        let mut agenda = Agenda::default();
        assert_eq!(agenda.proximo(), None);
        agenda.agendar(250, None, Tarefa::Recriar);
        agenda.agendar(50, Some(7), Tarefa::Destruir);
        agenda.agendar(500, Some(7), Tarefa::Reserva(Prazo::Enter));
        agenda.agendar(200, Some(7), Tarefa::Reserva(Prazo::Escala));
        assert_eq!(agenda.proximo(), Some(50));
        assert!(agenda.vencidos(49).is_empty(), "nada antes do prazo");
        let vencidos: Vec<Tarefa> = agenda.vencidos(250).iter().map(|a| a.tarefa).collect();
        assert_eq!(
            vencidos,
            vec![
                Tarefa::Destruir,
                Tarefa::Reserva(Prazo::Escala),
                Tarefa::Recriar
            ],
            "na ordem em que vencem, não na em que foram marcados"
        );
        assert_eq!(agenda.proximo(), Some(500), "o resto fica");
        // Empate: na ordem em que foram marcados.
        agenda.agendar(500, None, Tarefa::Recriar);
        let vencidos: Vec<Tarefa> = agenda.vencidos(1_000).iter().map(|a| a.tarefa).collect();
        assert_eq!(
            vencidos,
            vec![Tarefa::Reserva(Prazo::Enter), Tarefa::Recriar]
        );
        agenda.agendar(10, None, Tarefa::Recriar);
        agenda.limpar();
        assert_eq!(agenda.proximo(), None);
    }

    #[test]
    fn prazo_de_camada_morta_se_cala() {
        let destruir = Agendado {
            quando_ms: 50,
            geracao: Some(7),
            tarefa: Tarefa::Destruir,
        };
        assert!(destruir.vale_para(Some(7)));
        assert!(!destruir.vale_para(Some(8)), "outra camada");
        assert!(!destruir.vale_para(None), "camada nenhuma");
        let recriar = Agendado {
            geracao: None,
            tarefa: Tarefa::Recriar,
            ..destruir
        };
        assert!(recriar.vale_para(None) && recriar.vale_para(Some(8)));
    }

    #[test]
    fn movimentos_seguidos_do_ponteiro_viram_um() {
        let mut fila = Vec::new();
        empilhar_ponteiro(&mut fila, EventoPonteiro::Entrou { x: 1, y: 1 });
        for x in 2..50 {
            empilhar_ponteiro(&mut fila, EventoPonteiro::Moveu { x, y: 1 });
        }
        empilhar_ponteiro(
            &mut fila,
            EventoPonteiro::Apertou {
                botao: Botao::Esquerdo,
                x: 49,
                y: 1,
            },
        );
        empilhar_ponteiro(&mut fila, EventoPonteiro::Moveu { x: 60, y: 2 });
        empilhar_ponteiro(&mut fila, EventoPonteiro::Moveu { x: 61, y: 2 });
        assert_eq!(
            fila,
            vec![
                EventoOverlay::Ponteiro(EventoPonteiro::Entrou { x: 1, y: 1 }),
                EventoOverlay::Ponteiro(EventoPonteiro::Moveu { x: 49, y: 1 }),
                EventoOverlay::Ponteiro(EventoPonteiro::Apertou {
                    botao: Botao::Esquerdo,
                    x: 49,
                    y: 1
                }),
                EventoOverlay::Ponteiro(EventoPonteiro::Moveu { x: 61, y: 2 }),
            ]
        );
    }

    #[test]
    fn botoes_do_linux() {
        assert_eq!(botao(BTN_LEFT), Botao::Esquerdo);
        assert_eq!(botao(BTN_RIGHT), Botao::Direito);
        assert_eq!(botao(BTN_MIDDLE), Botao::Meio);
        assert_eq!(botao(0x113), Botao::Outro(0x113));
    }
}
