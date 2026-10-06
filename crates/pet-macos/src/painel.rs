//! O [`Overlay`] do macOS: um `NSPanel` não ativador, pequeno, que anda, com
//! o conteúdo num `CALayer` (CGImage BGRA pré-multiplicado, filtro nearest).
//! O clique vem de uma `NSView` que loga `mouseDown`/`Dragged`/`Up` e vira
//! [`EventoPonteiro`]; o click-through é o plano B (decisão 0101): a janela
//! alterna `ignoresMouseEvents` pela posição do ponteiro na caixa de toque.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSCursor, NSEvent, NSPanel, NSScreen, NSView,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::CGImage;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_quartz_core::CALayer;

use pet_core::cena::Elemento;
use pet_core::geometria::Ret;
use pet_core::plataforma::{
    Botao, CapOverlay, Cursor, Desenho, EventoOverlay, EventoPonteiro, Fase, InfoOverlay, Monitor,
    Overlay, UltimoQuadro,
};
use pet_core::skin::Skin;

use crate::pixels::{self, Tela};

/// Nível do painel: bem no alto, por cima de tudo (o spike usou 1000).
const NIVEL: isize = 1000;
/// Espera para destruir depois de esconder (ms), como no Wayland.
const ESPERA_DESTRUIR_MS: u64 = 50;

/// Estado que a `NSView` compartilha com o [`Painel`]: para onde mandar os
/// eventos do ponteiro e a geometria para converter as coordenadas.
#[derive(Default)]
pub struct PonteiroCompartilhado {
    pub eventos: Vec<EventoPonteiro>,
    pub bbox: Option<Ret>,
    pub tela: Option<Tela>,
    /// Um botão está apertado: segura o click-through (o arraste continua).
    pub apertado: bool,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "BichinhoVistaPet"]
    #[ivars = Rc<RefCell<PonteiroCompartilhado>>]
    struct VistaPet;

    impl VistaPet {
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, e: &NSEvent) {
            self.ivars().borrow_mut().apertado = true;
            self.empurrar(e, |x, y| EventoPonteiro::Apertou {
                botao: Botao::Esquerdo,
                x,
                y,
            });
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, e: &NSEvent) {
            self.empurrar(e, |x, y| EventoPonteiro::Moveu { x, y });
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, e: &NSEvent) {
            self.ivars().borrow_mut().apertado = false;
            self.empurrar(e, |x, y| EventoPonteiro::Soltou {
                botao: Botao::Esquerdo,
                x,
                y,
            });
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_down(&self, e: &NSEvent) {
            self.empurrar(e, |x, y| EventoPonteiro::Apertou {
                botao: Botao::Direito,
                x,
                y,
            });
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_up(&self, e: &NSEvent) {
            self.empurrar(e, |x, y| EventoPonteiro::Soltou {
                botao: Botao::Direito,
                x,
                y,
            });
        }

        // Recebe o primeiro clique mesmo sem ser a janela chave (não ativador).
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _e: *mut NSEvent) -> bool {
            true
        }
    }
);

impl VistaPet {
    fn empurrar(&self, e: &NSEvent, faz: impl Fn(i32, i32) -> EventoPonteiro) {
        let comp = self.ivars();
        let mut c = comp.borrow_mut();
        let (Some(tela), Some(bbox)) = (c.tela.clone(), c.bbox) else {
            return;
        };
        let loc = e.locationInWindow();
        let (x, y) = tela.janela_para_palco(bbox, loc);
        let ev = faz(x, y);
        c.eventos.push(ev);
    }
}

/// A janela viva.
struct Janela {
    panel: Retained<NSPanel>,
    _vista: Retained<VistaPet>,
    layer: Retained<CALayer>,
    tela: Tela,
    monitor: Monitor,
    bbox: Option<Ret>,
    ultima_cena: Vec<Elemento>,
    toque: Option<Ret>,
    seq: u64,
    commit: Instant,
    quadro: Option<CFRetained<CGImage>>,
    conteudo: bool,
}

pub struct Painel {
    mtm: MainThreadMarker,
    janela: Option<Janela>,
    eventos: Vec<EventoOverlay>,
    ponteiro: Rc<RefCell<PonteiroCompartilhado>>,
    saindo: bool,
    destruir_em: Option<u64>,
    agora_ms: u64,
}

impl Painel {
    pub fn novo(mtm: MainThreadMarker) -> Painel {
        Painel {
            mtm,
            janela: None,
            eventos: Vec::new(),
            ponteiro: Rc::new(RefCell::new(PonteiroCompartilhado::default())),
            saindo: false,
            destruir_em: None,
            agora_ms: 0,
        }
    }

    /// O laço marca o relógio a cada volta: `apagar_e_destruir` e os prazos
    /// usam este `agora_ms` (o `desenhar` não recebe o relógio).
    pub fn marcar_tempo(&mut self, agora_ms: u64) {
        self.agora_ms = agora_ms;
    }

    /// Alterna o click-through pela posição do ponteiro (plano B, decisão
    /// 0101). O laço chama a cada volta. Com um botão apertado, não mexe (o
    /// arraste continua).
    pub fn atualizar_click_through(&mut self) {
        let Some(j) = self.janela.as_ref() else {
            return;
        };
        if self.saindo {
            return;
        }
        let apertado = self.ponteiro.borrow().apertado;
        let dentro = if apertado {
            // Durante um aperto/arraste, a janela continua pegando (o cursor
            // sai do corpo quando o pet anda atrás dele).
            true
        } else if let Some(toque) = j.toque {
            let (x, y) = j.tela.desktop_para_palco(NSEvent::mouseLocation());
            toque.contem(x, y)
        } else {
            false
        };
        j.panel.setIgnoresMouseEvents(!dentro);
    }

    fn tela_ativa(&self) -> Option<Retained<NSScreen>> {
        NSScreen::mainScreen(self.mtm).or_else(|| NSScreen::screens(self.mtm).firstObject())
    }

    /// Segue o monitor ativo (`NSScreen.main`). Se mudou, refaz a tela e o
    /// monitor e anuncia `Pronta` para o Motor refazer o palco no monitor novo
    /// (decisão 0101; o seguir fino com poof do M4 vem depois). O laço chama no
    /// batimento.
    pub fn seguir_monitor(&mut self) {
        if self.saindo || self.janela.is_none() {
            return;
        }
        let Some(tela_ns) = NSScreen::mainScreen(self.mtm) else {
            return;
        };
        let nova = Tela::de(&tela_ns);
        if nova.nome == self.janela.as_ref().unwrap().tela.nome {
            return;
        }
        let monitor = nova.monitor(tela_ns.visibleFrame());
        self.ponteiro.borrow_mut().tela = Some(nova.clone());
        let j = self.janela.as_mut().unwrap();
        j.monitor = monitor;
        j.tela = nova;
        j.ultima_cena.clear();
        self.eventos.push(EventoOverlay::Pronta);
    }
}

impl Overlay for Painel {
    fn capacidades(&self) -> CapOverlay {
        crate::cap_overlay()
    }

    fn fase(&self) -> Fase {
        match &self.janela {
            None => Fase::Ausente,
            Some(_) if self.saindo => Fase::Saindo,
            Some(j) => Fase::Viva {
                conteudo: j.conteudo,
            },
        }
    }

    fn pronta(&self) -> Option<Monitor> {
        if self.saindo {
            return None;
        }
        self.janela.as_ref().map(|j| j.monitor.clone())
    }

    fn criar(&mut self) {
        let Some(tela_ns) = self.tela_ativa() else {
            aviso!("macOS: nenhuma NSScreen para criar o painel");
            return;
        };
        let tela = Tela::de(&tela_ns);
        let visivel = tela_ns.visibleFrame();
        let monitor = tela.monitor(visivel);

        // Começa com um retângulo mínimo; o primeiro `desenhar` acerta.
        let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(2.0, 2.0));
        let estilo = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(self.mtm),
            rect,
            estilo,
            NSBackingStoreType::Buffered,
            false,
        );
        panel.setLevel(NIVEL);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setHidesOnDeactivate(false);
        panel.setIgnoresMouseEvents(true);

        // Content view instrumentado, com a geometria compartilhada.
        {
            let mut c = self.ponteiro.borrow_mut();
            c.tela = Some(tela.clone());
            c.bbox = None;
            c.apertado = false;
        }
        let vista: Retained<VistaPet> = {
            let this = VistaPet::alloc(self.mtm).set_ivars(Rc::clone(&self.ponteiro));
            // SAFETY: initWithFrame do NSView.
            unsafe { msg_send![super(this), initWithFrame: rect] }
        };
        // SAFETY: liga a view e a camada; nearest para a nitidez D×D.
        let layer: Retained<CALayer> = unsafe {
            panel.setContentView(Some(&vista));
            vista.setWantsLayer(true);
            let l = vista.layer().expect("layer");
            let nearest = NSString::from_str("nearest");
            let _: () = msg_send![&l, setMagnificationFilter: &*nearest];
            let _: () = msg_send![&l, setMinificationFilter: &*nearest];
            l.setContentsScale(tela.escala);
            l
        };

        self.saindo = false;
        self.destruir_em = None;
        self.janela = Some(Janela {
            panel,
            _vista: vista,
            layer,
            tela,
            monitor,
            bbox: None,
            ultima_cena: Vec::new(),
            toque: None,
            seq: 0,
            commit: Instant::now(),
            quadro: None,
            conteudo: false,
        });
        self.eventos.push(EventoOverlay::Pronta);
    }

    fn cancelar_saida(&mut self) {
        if self.saindo {
            self.saindo = false;
            self.destruir_em = None;
            if let Some(j) = self.janela.as_ref() {
                j.panel.orderFrontRegardless();
            }
        }
    }

    fn apagar_e_destruir(&mut self, _skin: &Skin) -> bool {
        if let Some(j) = self.janela.as_mut() {
            // Sem fade no macOS: é só esconder (orderOut).
            j.panel.orderOut(None);
            j.conteudo = false;
        }
        self.saindo = true;
        self.destruir_em = Some(self.agora_ms + ESPERA_DESTRUIR_MS);
        true
    }

    fn destruir(&mut self) {
        if let Some(j) = self.janela.take() {
            j.panel.orderOut(None);
        }
        self.saindo = false;
        self.destruir_em = None;
        self.ponteiro.borrow_mut().bbox = None;
    }

    fn desenhar(
        &mut self,
        cena: &[Elemento],
        skin: &Skin,
        toque: Option<Ret>,
        _forcar: bool,
    ) -> Result<Desenho, String> {
        let Some(j) = self.janela.as_mut() else {
            return Err("macOS: desenhar sem janela".into());
        };
        let mudou_cena = cena != j.ultima_cena.as_slice();
        let mudou_toque = toque != j.toque;
        if !mudou_cena && !mudou_toque {
            return Ok(Desenho::SemMudanca);
        }

        match pixels::bbox_da_cena(cena, skin) {
            None => {
                // Cena vazia: quadro transparente.
                if mudou_cena {
                    // SAFETY: tira o conteúdo da camada.
                    unsafe {
                        let vazio: *const AnyObject = std::ptr::null();
                        let _: () = msg_send![&j.layer, setContents: vazio];
                    }
                    j.ultima_cena.clear();
                    j.bbox = None;
                    j.conteudo = false;
                    j.toque = toque;
                    self.ponteiro.borrow_mut().bbox = None;
                    return Ok(Desenho::Enviado {
                        retangulos: 0,
                        area: 0,
                    });
                }
                j.toque = toque;
                Ok(Desenho::SoEstado)
            }
            Some(bbox) => {
                if mudou_cena {
                    let dados = pixels::rasterizar(cena, skin, bbox);
                    let img =
                        pixels::cgimage(bbox.w, bbox.h, &dados).ok_or("macOS: CGImage falhou")?;
                    let frame = j.tela.palco_para_frame(bbox);
                    // SAFETY: reposiciona e troca o conteúdo da camada.
                    unsafe {
                        j.panel.setFrame_display(frame, false);
                        let ptr = pixels::contents_ptr(&img);
                        let _: () = msg_send![&j.layer, setContents: ptr];
                        if !j.conteudo {
                            j.panel.orderFrontRegardless();
                        }
                    }
                    j.quadro = Some(img);
                    j.bbox = Some(bbox);
                    j.ultima_cena = cena.to_vec();
                    j.conteudo = true;
                    j.seq += 1;
                    j.commit = Instant::now();
                    self.ponteiro.borrow_mut().bbox = Some(bbox);
                }
                j.toque = toque;
                Ok(Desenho::Enviado {
                    retangulos: 1,
                    area: (bbox.w as i64) * (bbox.h as i64),
                })
            }
        }
    }

    fn esquecer_cena(&mut self) {
        if let Some(j) = self.janela.as_mut() {
            j.ultima_cena.clear();
        }
    }

    fn cursor(&mut self, cursor: Cursor) {
        let c = match cursor {
            Cursor::Pegar => NSCursor::openHandCursor(),
            Cursor::Agarrar => NSCursor::closedHandCursor(),
        };
        c.set();
    }

    fn info(&self) -> InfoOverlay {
        match &self.janela {
            None => InfoOverlay::default(),
            Some(j) => InfoOverlay {
                monitor: j.monitor.nome.clone(),
                escala: Some(j.tela.escala),
                regiao: j.toque,
                visivel: j.conteudo && !self.saindo,
                shm_bytes: 0,
            },
        }
    }

    fn ultimo_quadro(&self) -> Option<UltimoQuadro> {
        if self.saindo {
            return None;
        }
        let j = self.janela.as_ref()?;
        if !j.conteudo || j.ultima_cena.is_empty() {
            return None;
        }
        Some(UltimoQuadro {
            monitor: j.monitor.nome.clone().unwrap_or_default(),
            cena: j.ultima_cena.clone(),
            seq: j.seq,
            idade_ms: j.commit.elapsed().as_millis() as u64,
        })
    }

    fn proximo_prazo(&self) -> Option<u64> {
        self.destruir_em
    }

    fn vencer(&mut self, agora_ms: u64) {
        self.agora_ms = agora_ms;
        if let Some(prazo) = self.destruir_em
            && agora_ms >= prazo
        {
            self.destruir_em = None;
            self.janela = None;
            self.saindo = false;
            self.ponteiro.borrow_mut().bbox = None;
            self.eventos.push(EventoOverlay::Saiu);
        }
    }

    fn eventos(&mut self) -> Vec<EventoOverlay> {
        let mut saida = std::mem::take(&mut self.eventos);
        let ponteiro: Vec<EventoPonteiro> = std::mem::take(&mut self.ponteiro.borrow_mut().eventos);
        saida.extend(ponteiro.into_iter().map(EventoOverlay::Ponteiro));
        saida
    }

    fn encerrar(&mut self, _confirmar: bool) {
        self.destruir();
    }
}
