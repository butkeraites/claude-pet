//! O [`Overlay`] do Windows: uma janela pequena layered (`WS_EX_LAYERED |
//! TOPMOST | TOOLWINDOW | NOACTIVATE | TRANSPARENT`) que anda e desenha o pet
//! com `UpdateLayeredWindow` (DIB BGRA pré-multiplicado — o que o raster do
//! pet-core já entrega). A janela é reposicionada e redimensionada a cada
//! quadro para o retângulo da cena no palco (a "janela pequena que anda").
//!
//! Desenha e anima o pet, e trata o ponteiro (arrasto, clique, clique direito)
//! pelo plano B do click-through (decisão 0111): a janela alterna
//! `WS_EX_TRANSPARENT` conforme o cursor está sobre a caixa de toque do pet,
//! e o `SetCapture` segura o mouse durante o arraste. A lógica do arraste e do
//! clique mora no Motor; aqui só traduzimos os eventos do Win32.

use std::cell::RefCell;
use std::sync::Once;

use pet_core::cena::Elemento;
use pet_core::geometria::Ret;
use pet_core::plataforma::{
    Botao, CapOverlay, Cursor, Desenho, EventoOverlay, EventoPonteiro, Fase, InfoOverlay, Monitor,
    Overlay, UltimoQuadro,
};
use pet_core::skin::Skin;

use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::CreateCompatibleDC;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateDIBSection, DeleteDC, DeleteObject,
    GetDC, HGDIOBJ, MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint, MonitorFromWindow, ReleaseDC,
    SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GWL_EXSTYLE, GetCursorPos, GetForegroundWindow,
    GetWindowLongPtrW, IDC_HAND, IDC_SIZEALL, LoadCursorW, RegisterClassW, SW_HIDE, SW_SHOWNA,
    SetCursor, SetWindowLongPtrW, ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::w;

use crate::pixels::{self, Tela};

/// Prazo curto para destruir depois de esconder (sem fantasma, como o macOS).
const SAIDA_MS: u64 = 50;

static REGISTRA: Once = Once::new();

/// Registra a classe da janela uma vez.
fn registrar(hinst: HINSTANCE) {
    REGISTRA.call_once(|| {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            lpszClassName: w!("ZecaOverlay"),
            ..Default::default()
        };
        // SAFETY: a WNDCLASSW está preenchida; RegisterClassW só a registra.
        unsafe {
            RegisterClassW(&wc);
        }
    });
}

/// O estado que a `wndproc` (mouse) e o `Painel` compartilham. O daemon é de
/// uma thread só (o laço e a `wndproc` rodam na principal), então um
/// `thread_local` basta — sem ponteiro cru nem `GWLP_USERDATA`.
#[derive(Default)]
struct Ponteiro {
    eventos: Vec<EventoPonteiro>,
    /// Onde a janela está no palco (para converter cliente → palco).
    bbox: Option<Ret>,
    /// Um botão está apertado: segura o click-through (o arraste continua).
    apertado: bool,
}

thread_local! {
    static PONTEIRO: RefCell<Ponteiro> = RefCell::new(Ponteiro::default());
}

/// Empilha um evento do ponteiro, convertendo o cliente (topo esquerda da
/// janela) para o palco pela `bbox` atual. Coordenadas nunca vão para o log.
fn empurrar_ponteiro(lp: LPARAM, faz: impl Fn(i32, i32) -> EventoPonteiro) {
    let cx = (lp.0 & 0xFFFF) as i16 as i32;
    let cy = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
    PONTEIRO.with(|p| {
        let mut p = p.borrow_mut();
        if let Some(bbox) = p.bbox {
            let ev = faz(bbox.x + cx, bbox.y + cy);
            p.eventos.push(ev);
        }
    });
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            PONTEIRO.with(|p| p.borrow_mut().apertado = true);
            // SAFETY: handle válido; segura o mouse durante o arraste.
            unsafe {
                let _ = SetCapture(hwnd);
            }
            empurrar_ponteiro(lp, |x, y| EventoPonteiro::Apertou {
                botao: Botao::Esquerdo,
                x,
                y,
            });
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            empurrar_ponteiro(lp, |x, y| EventoPonteiro::Moveu { x, y });
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            PONTEIRO.with(|p| p.borrow_mut().apertado = false);
            // SAFETY: libera o mouse depois do arraste.
            unsafe {
                let _ = ReleaseCapture();
            }
            empurrar_ponteiro(lp, |x, y| EventoPonteiro::Soltou {
                botao: Botao::Esquerdo,
                x,
                y,
            });
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            empurrar_ponteiro(lp, |x, y| EventoPonteiro::Apertou {
                botao: Botao::Direito,
                x,
                y,
            });
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            empurrar_ponteiro(lp, |x, y| EventoPonteiro::Soltou {
                botao: Botao::Direito,
                x,
                y,
            });
            LRESULT(0)
        }
        // SAFETY: repasse padrão do Win32 para o resto.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

/// A janela viva: o handle, a tela onde está e o que ela guarda do último
/// quadro.
struct Janela {
    hwnd: HWND,
    tela: Tela,
    monitor: Monitor,
    /// O pet está desenhado (há pixels dele na janela).
    conteudo: bool,
    ultima_cena: Vec<Elemento>,
    /// Onde a janela está no palco agora (para o ponteiro e o toque).
    bbox: Option<Ret>,
    toque: Option<Ret>,
    seq: u64,
    commit_ms: u64,
}

/// O [`Overlay`] do Windows.
pub struct Painel {
    janela: Option<Janela>,
    saindo: bool,
    destruir_em: Option<u64>,
    agora_ms: u64,
    eventos: Vec<EventoOverlay>,
    hinst: HINSTANCE,
    /// A janela está em `WS_EX_TRANSPARENT` agora (os cliques atravessam). A
    /// janela nasce assim; o `atualizar_click_through` só alterna quando muda.
    atravessa: bool,
}

impl Painel {
    pub fn novo() -> Painel {
        // SAFETY: GetModuleHandleW(None) devolve o módulo do processo.
        let hmod = unsafe { GetModuleHandleW(None) }.expect("GetModuleHandleW");
        Painel {
            janela: None,
            saindo: false,
            destruir_em: None,
            agora_ms: 0,
            eventos: Vec::new(),
            hinst: HINSTANCE(hmod.0),
            atravessa: true,
        }
    }

    /// O relógio do laço (o Motor conta os prazos por ele).
    pub fn marcar_tempo(&mut self, agora_ms: u64) {
        self.agora_ms = agora_ms;
    }

    /// O `HMONITOR` do monitor em foco agora (o da janela ativa, ou o do
    /// cursor, ou o primário).
    fn hmonitor_ativo() -> windows::Win32::Graphics::Gdi::HMONITOR {
        // SAFETY: chamadas sem efeito colateral; os handles podem ser nulos,
        // e MonitorFrom* trata isso com o flag.
        unsafe {
            let fg = GetForegroundWindow();
            if !fg.0.is_null() {
                return MonitorFromWindow(fg, MONITOR_DEFAULTTOPRIMARY);
            }
            let mut p = POINT::default();
            let _ = GetCursorPos(&mut p);
            MonitorFromPoint(p, MONITOR_DEFAULTTOPRIMARY)
        }
    }

    /// Segue o monitor ativo: se mudou, refaz a `Tela`/`Monitor` e pede ao
    /// Motor para refazer o palco (`Pronta`).
    pub fn seguir_monitor(&mut self) {
        if self.saindo {
            return;
        }
        let Some(j) = self.janela.as_mut() else {
            return;
        };
        let Some(nova) = Tela::de_hmonitor(Self::hmonitor_ativo()) else {
            return;
        };
        if nova.nome == j.tela.nome {
            return;
        }
        j.monitor = nova.para_monitor();
        j.tela = nova;
        j.ultima_cena.clear();
        self.eventos.push(EventoOverlay::Pronta);
    }

    /// Alterna o click-through pela posição do cursor (plano B, decisão 0111).
    /// O laço chama a cada volta. Com um botão apertado, a janela continua
    /// pegando (o arraste segue mesmo quando o pet anda atrás do cursor).
    pub fn atualizar_click_through(&mut self) {
        if self.saindo {
            return;
        }
        let (hwnd, dentro) = {
            let Some(j) = self.janela.as_ref() else {
                return;
            };
            let apertado = PONTEIRO.with(|p| p.borrow().apertado);
            let dentro = if apertado {
                true
            } else if let Some(toque) = j.toque {
                let mut pt = POINT::default();
                // SAFETY: lê a posição do cursor na tela (sem efeito colateral).
                unsafe {
                    let _ = GetCursorPos(&mut pt);
                }
                let (x, y) = j.tela.tela_para_palco(pt.x, pt.y);
                toque.contem(x, y)
            } else {
                false
            };
            (j.hwnd, dentro)
        };
        let atravessar = !dentro;
        if atravessar != self.atravessa {
            self.atravessa = atravessar;
            // SAFETY: alterna o bit `WS_EX_TRANSPARENT` da janela viva.
            unsafe {
                let cur = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                let novo = if atravessar {
                    cur | WS_EX_TRANSPARENT.0
                } else {
                    cur & !WS_EX_TRANSPARENT.0
                };
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, novo as isize);
            }
        }
    }

    /// Esconde a janela (sem fantasma: o Windows não tem fade da camada).
    fn esconder(j: &mut Janela) {
        if j.conteudo {
            // SAFETY: handle válido da janela.
            unsafe {
                let _ = ShowWindow(j.hwnd, SW_HIDE);
            }
            j.conteudo = false;
        }
        PONTEIRO.with(|p| p.borrow_mut().bbox = None);
    }
}

/// Desenha o buffer BGRA pré-multiplicado na janela layered, posicionando-a em
/// (x,y) com o tamanho (w,h), tudo em pixels da tela.
///
/// # Safety
/// `dados` tem `w*h*4` bytes; `hwnd` é uma janela layered viva.
unsafe fn desenhar_dib(hwnd: HWND, x: i32, y: i32, w: i32, h: i32, dados: &[u8]) {
    // SAFETY: todas as chamadas GDI abaixo operam sobre handles que criamos e
    // liberamos aqui mesmo; o `dados` tem w*h*4 bytes (contrato do chamador).
    unsafe {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let screen = GetDC(None);
        let memdc = CreateCompatibleDC(screen);
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let Ok(dib) = CreateDIBSection(
            screen,
            &bmi,
            windows::Win32::Graphics::Gdi::DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        ) else {
            ReleaseDC(None, screen);
            let _ = DeleteDC(memdc);
            return;
        };
        std::ptr::copy_nonoverlapping(
            dados.as_ptr(),
            bits.cast::<u8>(),
            dados.len().min((w * h * 4) as usize),
        );
        let antigo = SelectObject(memdc, HGDIOBJ(dib.0));

        let blend = BLENDFUNCTION {
            BlendOp: 0,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: 1, // AC_SRC_ALPHA
        };
        let dst = POINT { x, y };
        let src = POINT { x: 0, y: 0 };
        let size = SIZE { cx: w, cy: h };
        let _ = UpdateLayeredWindow(
            hwnd,
            screen,
            Some(&dst),
            Some(&size),
            memdc,
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        SelectObject(memdc, antigo);
        let _ = DeleteObject(HGDIOBJ(dib.0));
        let _ = DeleteDC(memdc);
        ReleaseDC(None, screen);
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
        let Some(tela) = Tela::de_hmonitor(Self::hmonitor_ativo()) else {
            return;
        };
        let monitor = tela.para_monitor();
        // SAFETY: cria a janela layered; a classe é registrada antes.
        let hwnd = unsafe {
            registrar(self.hinst);
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT,
                w!("ZecaOverlay"),
                w!("Zeca"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                self.hinst,
                None,
            )
        };
        let Ok(hwnd) = hwnd else {
            return;
        };
        self.janela = Some(Janela {
            hwnd,
            tela,
            monitor,
            conteudo: false,
            ultima_cena: Vec::new(),
            bbox: None,
            toque: None,
            seq: 0,
            commit_ms: 0,
        });
        self.saindo = false;
        self.destruir_em = None;
        self.eventos.push(EventoOverlay::Pronta);
    }

    fn cancelar_saida(&mut self) {
        if self.saindo {
            self.saindo = false;
            self.destruir_em = None;
            if let Some(j) = self.janela.as_ref() {
                // SAFETY: handle válido.
                unsafe {
                    let _ = ShowWindow(j.hwnd, SW_SHOWNA);
                }
            }
        }
    }

    fn apagar_e_destruir(&mut self, _skin: &Skin) -> bool {
        if let Some(j) = self.janela.as_mut() {
            Painel::esconder(j);
        }
        self.saindo = true;
        self.destruir_em = Some(self.agora_ms + SAIDA_MS);
        true
    }

    fn destruir(&mut self) {
        if let Some(j) = self.janela.take() {
            // SAFETY: handle válido; destrói a janela.
            unsafe {
                let _ = DestroyWindow(j.hwnd);
            }
        }
        self.saindo = false;
        self.destruir_em = None;
        self.atravessa = true;
        PONTEIRO.with(|p| {
            let mut p = p.borrow_mut();
            p.bbox = None;
            p.apertado = false;
        });
    }

    fn desenhar(
        &mut self,
        cena: &[Elemento],
        skin: &Skin,
        toque: Option<Ret>,
        _forcar: bool,
    ) -> Result<Desenho, String> {
        let agora = self.agora_ms;
        let Some(j) = self.janela.as_mut() else {
            return Ok(Desenho::SemMudanca);
        };

        let Some(bbox) = pixels::bbox_da_cena(cena, skin) else {
            // Cena vazia: esconde o pet.
            Painel::esconder(j);
            j.ultima_cena.clear();
            j.bbox = None;
            let mudou = j.toque != toque;
            j.toque = toque;
            return Ok(if mudou {
                Desenho::SoEstado
            } else {
                Desenho::SemMudanca
            });
        };

        if j.conteudo && cena == j.ultima_cena.as_slice() {
            let mudou = j.toque != toque;
            j.toque = toque;
            return Ok(if mudou {
                Desenho::SoEstado
            } else {
                Desenho::SemMudanca
            });
        }

        let dados = pixels::rasterizar(cena, skin, bbox);
        let (x, y, w, h) = j.tela.palco_para_tela(bbox);
        // SAFETY: `dados` tem w*h*4 bytes; a janela está viva.
        unsafe {
            desenhar_dib(j.hwnd, x, y, w, h, &dados);
            if !j.conteudo {
                let _ = ShowWindow(j.hwnd, SW_SHOWNA);
            }
        }
        j.ultima_cena = cena.to_vec();
        j.bbox = Some(bbox);
        j.toque = toque;
        j.conteudo = true;
        j.seq += 1;
        j.commit_ms = agora;
        PONTEIRO.with(|p| p.borrow_mut().bbox = Some(bbox));
        Ok(Desenho::Enviado {
            retangulos: 1,
            area: w as i64 * h as i64,
        })
    }

    fn esquecer_cena(&mut self) {
        if let Some(j) = self.janela.as_mut() {
            j.ultima_cena.clear();
        }
    }

    fn cursor(&mut self, cursor: Cursor) {
        // O Windows não tem par mão-aberta/mão-fechada: a mão (IDC_HAND) para
        // «dá para pegar» e as setas de mover (IDC_SIZEALL) para «segurando».
        let nome = match cursor {
            Cursor::Pegar => IDC_HAND,
            Cursor::Agarrar => IDC_SIZEALL,
        };
        // SAFETY: carrega um cursor do sistema e o aplica, na thread do laço.
        unsafe {
            if let Ok(h) = LoadCursorW(None, nome) {
                SetCursor(h);
            }
        }
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
        if !j.conteudo {
            return None;
        }
        Some(UltimoQuadro {
            monitor: j.monitor.nome.clone().unwrap_or_default(),
            cena: j.ultima_cena.clone(),
            seq: j.seq,
            idade_ms: self.agora_ms.saturating_sub(j.commit_ms),
        })
    }

    fn proximo_prazo(&self) -> Option<u64> {
        self.destruir_em
    }

    fn vencer(&mut self, agora_ms: u64) {
        if self.destruir_em.is_some_and(|p| agora_ms >= p) {
            self.destruir();
            self.eventos.push(EventoOverlay::Saiu);
        }
    }

    fn eventos(&mut self) -> Vec<EventoOverlay> {
        let mut saida = std::mem::take(&mut self.eventos);
        let ponteiro = PONTEIRO.with(|p| std::mem::take(&mut p.borrow_mut().eventos));
        saida.extend(ponteiro.into_iter().map(EventoOverlay::Ponteiro));
        saida
    }

    fn encerrar(&mut self, _confirmar: bool) {
        self.destruir();
    }
}
