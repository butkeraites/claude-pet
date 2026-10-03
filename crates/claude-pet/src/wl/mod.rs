//! Sessão Wayland: uma conexão com o compositor, do handshake ao EOF.
//!
//! Wayland em Rust puro (smithay-client-toolkit 0.21 sem xkbcommon,
//! wayland-client sem libwayland), decisão 0004. A sessão só existe
//! enquanto o compositor responde; qualquer erro de protocolo ou EOF derruba
//! tudo e o laço volta a esperar (decisão 0007). Por isso nada aqui tenta se
//! recuperar da conexão sozinho: quem decide reconectar é o [`crate::laco`].
//! O que a sessão recupera sozinha é a própria camada (`closed` → recria).

pub mod saida;
pub mod shm;
pub mod sincronia;
pub mod superficie;

use std::collections::VecDeque;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use pet_core::cena::{self, Elemento};
use pet_core::confete::{Chuva, Grade};
use pet_core::skin::Skin;

use smithay_client_toolkit as sctk;
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::{LoopHandle, RegistrationToken};
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
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
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_registry, registry_handlers};

use crate::comando::QuadroEsperado;
use crate::estado::{Compartilhado, Painel};
use crate::laco::Laco;
use crate::pet::{Palco, Pet};
use superficie::{Desenho, Fase, OrigemEscala, Passo, Prazo, Superficie};

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
/// Janela da contagem de commits.
const JANELA_COMMITS: Duration = Duration::from_secs(60);
/// Prazo para o compositor responder antes do handshake.
const LIMITE_HANDSHAKE: Duration = Duration::from_secs(3);
/// Prazo para o compositor confirmar o quadro transparente ao sair (o
/// `stop_grace_period` do compose é de 5 s).
const LIMITE_SAIDA: Duration = Duration::from_secs(1);
/// Confetes do teste de estresse, sempre com a mesma semente: medições
/// repetidas veem as mesmas trajetórias.
const CONFETES: usize = 40;
const SEMENTE_CONFETE: u64 = 7;
/// Lado de cada confete, em pixels de arte.
const LADO_CONFETE: i32 = 3;

/// Teste de estresse em curso (`/v1/debug/estresse`). Os prazos ficam no
/// mesmo relógio inteiro em milissegundos do animador: o timer dispara no
/// prazo ou depois, e o passo devido sempre acontece (sem laço ocupado por
/// um prazo arredondado para baixo).
struct Estresse {
    chuva: Chuva,
    fps: u64,
    inicio_ms: u64,
    /// Passos dados (o passo `n` vence em `inicio + ⌈n·1000/fps⌉`).
    passos: u64,
    fim_ms: u64,
}

impl Estresse {
    fn prazo_do_passo(&self, n: u64) -> u64 {
        self.inicio_ms + (n * 1000).div_ceil(self.fps)
    }

    /// Dá um passo se algum venceu. Passos atrasados não viram rajada: o
    /// confete só pula para o passo de agora.
    fn avancar(&mut self, agora_ms: u64) {
        let devidos = agora_ms.saturating_sub(self.inicio_ms) * self.fps / 1000;
        if devidos > self.passos {
            self.chuva.passo();
            self.passos = devidos;
        }
    }

    fn proximo_prazo(&self) -> u64 {
        self.prazo_do_passo(self.passos + 1).min(self.fim_ms)
    }
}

/// Commits Wayland: o total e os do último minuto (orçamento da decisão
/// 0005: média de até 2/s parado, 0 dormindo, rajadas de até 30 fps).
#[derive(Debug, Default)]
pub struct Commits {
    recentes: VecDeque<Instant>,
    pub total: u64,
}

impl Commits {
    pub fn contar(&mut self, agora: Instant) {
        self.total += 1;
        self.recentes.push_back(agora);
        self.podar(agora);
    }

    pub fn por_minuto(&mut self, agora: Instant) -> usize {
        self.podar(agora);
        self.recentes.len()
    }

    fn podar(&mut self, agora: Instant) {
        while let Some(&t) = self.recentes.front() {
            if agora.saturating_duration_since(t) <= JANELA_COMMITS {
                break;
            }
            self.recentes.pop_front();
        }
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
    handle: LoopHandle<'static, Laco>,
    conexao: Connection,
    superficie: Option<Superficie>,
    /// O pet deve aparecer (há personagem e ninguém mandou esconder).
    visivel: bool,
    pet: Option<Pet>,
    /// Onde o pet fica na superfície atual (D e posição).
    palco: Option<Palco>,
    /// Relógio da animação: os quadros contam a partir daqui.
    inicio: Instant,
    /// Timer da próxima troca de quadro.
    relogio: Option<RegistrationToken>,
    commits: Commits,
    comp: Arc<Compartilhado>,
    assentos: SeatState,
    ponteiro: Option<wl_pointer::WlPointer>,
    cursores: Option<CursorShapeManager>,
    forma_do_cursor: Option<WpCursorShapeDeviceV1>,
    estresse: Option<Estresse>,
}

/// Resultado do handshake: a conexão, a fila e o estado que ela despacha.
pub struct Conexao {
    pub conexao: Connection,
    pub fila: EventQueue<Sessao>,
    pub sessao: Sessao,
}

/// Faz o handshake numa conexão já aberta (a da prova da descoberta).
pub fn conectar(
    fluxo: UnixStream,
    handle: LoopHandle<'static, Laco>,
    skin: Option<Rc<Skin>>,
    comp: Arc<Compartilhado>,
) -> Result<Conexao, String> {
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
        handle,
        conexao: conexao.clone(),
        superficie: None,
        visivel: false,
        pet: skin.map(|skin| Pet::novo(skin, 0)),
        palco: None,
        inicio: Instant::now(),
        relogio: None,
        commits: Commits::default(),
        comp,
        assentos,
        ponteiro: None,
        cursores,
        forma_do_cursor: None,
        estresse: None,
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

impl Sessao {
    fn agora_ms(&self) -> u64 {
        self.inicio.elapsed().as_millis() as u64
    }

    /// Publica o que a sessão sabe no `/v1/estado`.
    pub fn publicar(&mut self) {
        let agora_ms = self.agora_ms();
        // Pelo relógio: com a tela apagada o último quadro do estresse pode
        // nunca ser desenhado, mas o estresse acabou do mesmo jeito.
        let estresse = self.estresse.as_ref().is_some_and(|e| agora_ms < e.fim_ms);
        let superficie = self.superficie.as_ref();
        let pronta = superficie.and_then(Superficie::pronta);
        let visivel = superficie.is_some_and(|s| s.tem_conteudo() && !s.saindo);
        let (d, sprite_disp) = match (&self.palco, &self.pet) {
            (Some(palco), Some(pet)) if visivel => (Some(palco.d), pet.sprite_disp(palco)),
            _ => (None, None),
        };
        let painel = Painel {
            monitor: pronta.and_then(|p| p.monitor.clone()),
            escala: pronta.map(|p| p.escala),
            d,
            sprite_disp,
            regiao_entrada: superficie.and_then(Superficie::regiao),
            visivel,
            estresse,
            commits_por_min: self.commits.por_minuto(Instant::now()),
            commits_total: self.commits.total,
            shm_bytes: superficie.map_or(0, Superficie::bytes_shm),
        };
        self.comp.publicar_painel(painel);
    }

    /// O sprite como está na tela agora, para a checagem de nitidez.
    pub fn quadro_esperado(&self) -> Option<QuadroEsperado> {
        let superficie = self.superficie.as_ref()?;
        if superficie.saindo {
            return None;
        }
        let pronta = superficie.pronta()?;
        let pet = self.pet.as_ref()?;
        let palco = self.palco.as_ref()?;
        let sprite = superficie
            .cena_atual()?
            .iter()
            .find(|e| matches!(e, Elemento::Sprite { .. }))?;
        let Elemento::Sprite { x, y, d, .. } = *sprite else {
            return None;
        };
        let area = pet.sprite_disp(palco)?;
        let (seq, idade) = superficie.ultimo_quadro();
        Some(QuadroEsperado {
            monitor: pronta.monitor.clone().unwrap_or_default(),
            area,
            grade: (x, y),
            d,
            seq,
            idade_ms: idade.map_or(0, |t| t.as_millis() as u64),
            rgba: cena::rgba_do_sprite(pet.skin(), sprite, area),
        })
    }

    /// Confete pela tela inteira por `segundos`, a `fps` quadros por
    /// segundo; depois volta ao repouso. O confete anda na grade de arte do
    /// pet (múltiplos de D a partir do canto da célula).
    pub fn estresse(&mut self, fps: u32, segundos: u32) {
        let Some(palco) = self.palco else {
            aviso!("debug: estresse pedido com o pet fora da tela");
            return;
        };
        let agora_ms = self.agora_ms();
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
        self.desenhar();
        // Com um frame callback pendente nada foi desenhado ainda: publica
        // assim mesmo, para o /v1/estado já mostrar o estresse.
        self.publicar();
    }

    fn contar_commit(&mut self) {
        self.commits.contar(Instant::now());
        self.publicar();
    }

    /// Liga ou desliga o pet na tela ([`superficie::passo_de_visibilidade`]).
    /// Ligar cria a camada (no monitor focado) ou cancela uma saída em
    /// curso; desligar desenha transparente e só então destrói, para o fade
    /// de saída do Hyprland não guardar um quadro fantasma.
    pub fn definir_visivel(&mut self, visivel: bool) {
        self.visivel = visivel && self.pet.is_some();
        let fase = self
            .superficie
            .as_ref()
            .map_or(Fase::Ausente, Superficie::fase);
        match superficie::passo_de_visibilidade(fase, self.visivel) {
            Passo::Nada => {}
            Passo::Criar => self.criar_superficie(),
            Passo::Cancelar => {
                if let Some(superficie) = self.superficie.as_mut() {
                    depurar!("mostrar durante a saída: a camada fica");
                    superficie.saindo = false;
                }
                // Como esconder, mostrar é uma ordem e não animação: o pet
                // volta já, mesmo com um frame callback pendente (que, com a
                // tela apagada, só chegaria quando ela acendesse).
                self.desenhar_com(true);
            }
            Passo::ApagarEDestruir => self.apagar_e_destruir(),
            Passo::Destruir => {
                self.cancelar_relogio();
                self.estresse = None;
                self.superficie = None;
            }
        }
        self.publicar();
    }

    /// Troca o personagem na tela (aprovação ou revogação, decisão 0026).
    /// Sem skin, o pet se esconde com o quadro transparente (desenhado com a
    /// skin velha); com skin nova, a cena velha é esquecida e o próximo
    /// quadro redesenha a tela toda.
    pub fn trocar_skin(&mut self, skin: Option<Rc<Skin>>, visivel: bool) {
        match skin {
            None => {
                self.definir_visivel(false);
                self.pet = None;
            }
            Some(skin) => {
                self.pet = Some(Pet::novo(skin, self.agora_ms()));
                self.palco = None;
                if let Some(superficie) = self.superficie.as_mut() {
                    superficie.esquecer_cena();
                }
                self.definir_visivel(visivel);
                self.tentar_aprontar();
            }
        }
        self.publicar();
    }

    /// Antes de sair do processo: quadro transparente, camada destruída e
    /// uma ida e volta com prazo, para garantir que o compositor processou
    /// tudo antes de o socket fechar (o libwayland-server descarta o que não
    /// leu quando o cliente desliga).
    pub fn encerrar(&mut self) {
        let tinha_camada = self.superficie.is_some();
        self.definir_visivel(false);
        self.cancelar_relogio();
        self.superficie = None;
        if !tinha_camada {
            return;
        }
        match sincronia::sincronizar(&self.conexao, LIMITE_SAIDA) {
            Ok(()) => depurar!("o compositor processou o quadro transparente e a destruição"),
            Err(e) => aviso!("saindo sem confirmação do compositor: {e}"),
        }
    }

    fn criar_superficie(&mut self) {
        let superficie = Superficie::criar(
            &self.compositor,
            &self.camadas,
            self.fracional.as_ref(),
            &self.viewporter,
            &self.qh,
        );
        depurar!("camada criada (geração {})", superficie.geracao);
        self.palco = None;
        self.superficie = Some(superficie);
    }

    /// Esconde uma camada com pixels do pet: região de input vazia, quadro
    /// transparente com commit e destruição no frame callback seguinte (ou
    /// em [`ESPERA_DESTRUIR`]). Funciona também logo depois de uma troca de
    /// escala, quando o buffer antigo já foi descartado: o quadro vazio vai
    /// num buffer novo, zerado, com dano na tela inteira.
    fn apagar_e_destruir(&mut self) {
        self.cancelar_relogio();
        self.estresse = None;
        let (Some(superficie), Some(pet)) = (self.superficie.as_mut(), self.pet.as_ref()) else {
            self.superficie = None;
            return;
        };
        superficie.saindo = true;
        if let Err(e) = superficie.definir_regiao(None, &self.compositor) {
            aviso!("região de input ao esconder: {e}");
        }
        let geracao = superficie.geracao;
        match superficie.desenhar(&[], pet.skin(), &self.shm, &self.qh, true) {
            Ok(Desenho::Enviado { .. }) => self.contar_commit(),
            Ok(_) => {}
            Err(e) => {
                aviso!("quadro transparente ao esconder: {e}");
                self.superficie = None;
                return;
            }
        }
        self.depois(ESPERA_DESTRUIR, geracao, |sessao| {
            if sessao.superficie.as_ref().is_some_and(|s| s.saindo) {
                sessao.superficie = None;
                sessao.publicar();
            }
        });
    }

    /// Desenha o quadro de agora (se a camada está pronta e algo mudou) e
    /// agenda a próxima troca. Com um frame callback pendente o quadro
    /// espera por ele e nenhum timer é armado: é o callback que chama de
    /// novo, já com o quadro do instante em que chegar. Com a tela apagada
    /// ele não chega, e o pet fica sem commit nenhum até ela acender.
    fn desenhar(&mut self) {
        self.desenhar_com(false);
    }

    /// [`Self::desenhar`]; `forcar` faz o commit mesmo com um frame callback
    /// pendente (sem pedir outro).
    fn desenhar_com(&mut self, forcar: bool) {
        let agora_ms = self.agora_ms();
        let Some(palco) = self.palco else {
            return;
        };
        let (Some(superficie), Some(pet)) = (self.superficie.as_mut(), self.pet.as_ref()) else {
            return;
        };
        if superficie.saindo {
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
        let regiao = pet.regiao_de_toque(&palco);
        let mudou_regiao = superficie
            .definir_regiao(regiao, &self.compositor)
            .unwrap_or_else(|e| {
                aviso!("região de input: {e}");
                false
            });
        let resultado = superficie.desenhar(&cena, pet.skin(), &self.shm, &self.qh, forcar);
        match resultado {
            Ok(Desenho::Enviado { retangulos, area }) => {
                self.contar_commit();
                if self.estresse.is_none() {
                    depurar!("quadro enviado: {retangulos} retângulo(s) de dano, {area} px");
                }
            }
            Ok(Desenho::SemMudanca) if mudou_regiao => {
                superficie.commit_de_estado();
                self.contar_commit();
            }
            // O frame callback pendente chama `desenhar` de novo.
            Ok(Desenho::Adiado) => return,
            Ok(Desenho::SemMudanca) => {}
            Err(e) => aviso!("desenho: {e}"),
        }
        if let Some(proxima) = proxima {
            self.armar_relogio(proxima);
        }
    }

    fn armar_relogio(&mut self, prazo_ms: u64) {
        self.cancelar_relogio();
        let quando = self.inicio + Duration::from_millis(prazo_ms);
        let inserido = self
            .handle
            .insert_source(Timer::from_deadline(quando), |_, _, laco| {
                if let Some(sessao) = laco.sessao_mut() {
                    sessao.relogio = None;
                    sessao.desenhar();
                }
                TimeoutAction::Drop
            });
        match inserido {
            Ok(token) => self.relogio = Some(token),
            Err(e) => erro!("não consegui armar o relógio da animação: {e}"),
        }
    }

    fn cancelar_relogio(&mut self) {
        if let Some(token) = self.relogio.take() {
            self.handle.remove(token);
        }
    }

    /// Arma um timer que chama `acao` nesta sessão se a superfície ainda
    /// for a mesma geração.
    fn depois(&self, daqui_a: Duration, geracao: u64, acao: fn(&mut Sessao)) {
        let timer = Timer::from_duration(daqui_a);
        let inserido = self.handle.insert_source(timer, move |_, _, laco| {
            if let Some(sessao) = laco.sessao_mut() {
                let mesma = sessao.superficie.as_ref().map(|s| s.geracao) == Some(geracao);
                if mesma {
                    acao(sessao);
                }
            }
            TimeoutAction::Drop
        });
        if let Err(e) = inserido {
            erro!("não consegui armar um timer da camada: {e}");
        }
    }

    fn prazo_escala(&mut self) {
        self.vencer(Prazo::Escala);
    }

    fn prazo_enter(&mut self) {
        self.vencer(Prazo::Enter);
    }

    fn vencer(&mut self, prazo: Prazo) {
        if let Some(superficie) = self.superficie.as_mut() {
            if superficie.pronta().is_none() {
                depurar!("prazo de reserva vencido: {prazo:?}");
            }
            superficie.venceu(prazo);
        }
        self.tentar_aprontar();
    }

    /// Recalcula tamanho, escala e monitor da camada; quando mudam, refaz o
    /// palco (D e posição) e desenha.
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
        if let Some(pet) = &self.pet {
            let palco = pet.palco(pronta.logico, pronta.escala, (largura, altura));
            info!(
                "pet «{}» com D={} e célula em ({}, {}) pixels do monitor",
                pet.skin().id,
                palco.d,
                palco.x,
                palco.y
            );
            self.palco = Some(palco);
            self.desenhar();
        }
    }

    /// Larga a camada atual e cria outra (output NULL: o monitor focado)
    /// depois de [`RECRIAR_APOS_FECHAR`], para o monitor assentar. Usado no
    /// `closed` e quando aparece um monitor de verdade para uma camada que
    /// caiu no FALLBACK. A camada largada aqui nunca tem pixels do pet (o
    /// compositor já a fechou, ou ela nunca desenhou).
    fn recriar_depois(&mut self) {
        self.cancelar_relogio();
        self.superficie = None;
        self.palco = None;
        self.publicar();
        let timer = Timer::from_duration(RECRIAR_APOS_FECHAR);
        let inserido = self.handle.insert_source(timer, |_, _, laco| {
            if let Some(sessao) = laco.sessao_mut()
                && sessao.visivel
                && sessao.superficie.is_none()
            {
                sessao.criar_superficie();
            }
            TimeoutAction::Drop
        });
        if let Err(e) = inserido {
            erro!("não consegui agendar a recriação da camada: {e}");
        }
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
            self.publicar();
        } else if superficie.pendente {
            self.desenhar();
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
                self.depois(superficie::PRAZO_ESCALA, geracao, Sessao::prazo_escala);
                self.depois(superficie::PRAZO_ENTER, geracao, Sessao::prazo_enter);
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
    /// No M1 o ponteiro só troca o cursor para "pegar" em cima do pet; o
    /// arraste chega no M4. Coordenadas nunca vão para o log.
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        eventos: &[PointerEvent],
    ) {
        for evento in eventos {
            let nossa = self
                .superficie
                .as_ref()
                .is_some_and(|s| s.eh(&evento.surface));
            if let (true, PointerEventKind::Enter { serial }) = (nossa, &evento.kind)
                && let Some(forma) = &self.forma_do_cursor
            {
                forma.set_shape(*serial, Shape::Grab);
            }
        }
    }
}

impl Drop for Sessao {
    fn drop(&mut self) {
        self.cancelar_relogio();
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn commits_por_minuto_esquece_o_que_passou_de_60_s() {
        let t0 = Instant::now();
        let mut c = Commits::default();
        for i in 0..5 {
            c.contar(t0 + Duration::from_secs(i * 10));
        }
        assert_eq!(c.por_minuto(t0 + Duration::from_secs(40)), 5);
        assert_eq!(c.por_minuto(t0 + Duration::from_secs(75)), 3);
        assert_eq!(c.por_minuto(t0 + Duration::from_secs(500)), 0);
        assert_eq!(c.total, 5);
    }
}
