//! Coordenadas e raster do backend do Windows: a ponte entre o palco do Motor
//! (pixels do dispositivo, origem no canto superior esquerdo do monitor) e o
//! Win32. No Windows o palco **já é** o sistema de coordenadas da tela (topo
//! esquerda, físico), então as conversões são triviais — sem os flips de Y do
//! macOS. O raster do `pet-core` já entrega **BGRA pré-multiplicado**, que é o
//! que o `UpdateLayeredWindow` quer.

use pet_core::cena::Elemento;
use pet_core::geometria::Ret;
use pet_core::plataforma::Monitor;
use pet_core::raster::Alvo;
use pet_core::skin::Skin;

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

/// A geometria de um monitor, para converter o palco em posição de janela e o
/// ponteiro de volta. Nada de título ou nome de janela sai daqui.
#[derive(Clone)]
pub struct Tela {
    /// Retângulo do monitor na tela, em pixels físicos.
    pub monitor: RECT,
    /// Área útil (sem a barra de tarefas), em pixels físicos.
    pub util: RECT,
    /// Pixels do dispositivo por pixel lógico (dpi / 96).
    pub escala: f64,
    /// Um id estável do monitor, sem título: a posição dele.
    pub nome: String,
}

impl Tela {
    /// Lê a geometria de um `HMONITOR`.
    pub fn de_hmonitor(hmon: HMONITOR) -> Option<Tela> {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `info.cbSize` está posto; `GetMonitorInfoW` preenche a struct.
        let ok = unsafe { GetMonitorInfoW(hmon, &mut info) };
        if !ok.as_bool() {
            return None;
        }
        let mut dpix = 96u32;
        let mut dpiy = 96u32;
        // SAFETY: ponteiros válidos; a função só escreve os dpi.
        let _ = unsafe { GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpix, &mut dpiy) };
        let escala = (dpix as f64 / 96.0).max(1.0);
        let nome = format!("display@{},{}", info.rcMonitor.left, info.rcMonitor.top);
        Some(Tela {
            monitor: info.rcMonitor,
            util: info.rcWork,
            escala,
            nome,
        })
    }

    /// O [`Monitor`] que o Overlay anuncia ao Motor.
    pub fn para_monitor(&self) -> Monitor {
        let s = self.escala;
        let pw = (self.monitor.right - self.monitor.left) as f64;
        let ph = (self.monitor.bottom - self.monitor.top) as f64;
        let logico = ((pw / s).round() as u32, (ph / s).round() as u32);
        // Área útil no palco (físico, origem no topo do monitor).
        let util = Ret::novo(
            self.util.left - self.monitor.left,
            self.util.top - self.monitor.top,
            self.util.right - self.util.left,
            self.util.bottom - self.util.top,
        );
        Monitor {
            nome: Some(self.nome.clone()),
            descricao: Some(self.nome.clone()),
            logico,
            escala: s,
            origem: Some((
                (self.monitor.left as f64 / s).round() as i32,
                (self.monitor.top as f64 / s).round() as i32,
            )),
            area_util: Some(util),
        }
    }

    /// Um retângulo do palco (físico, topo do monitor) vira a posição+tamanho
    /// da janela na tela (físico): só soma a origem do monitor (sem flip).
    pub fn palco_para_tela(&self, bbox: Ret) -> (i32, i32, i32, i32) {
        (
            self.monitor.left + bbox.x,
            self.monitor.top + bbox.y,
            bbox.w,
            bbox.h,
        )
    }

    /// Um ponto da tela (físico) vira um ponto do palco (físico, topo do
    /// monitor). Para o click-through pelo plano B (`GetCursorPos`).
    pub fn tela_para_palco(&self, sx: i32, sy: i32) -> (i32, i32) {
        (sx - self.monitor.left, sy - self.monitor.top)
    }
}

/// A caixa que contém a cena inteira no palco (a união dos limites de cada
/// elemento). `None` se a cena está vazia. (Puro; igual ao macOS.)
pub fn bbox_da_cena(cena: &[Elemento], skin: &Skin) -> Option<Ret> {
    let mut it = cena.iter().map(|e| e.limites(skin)).filter(|r| !r.vazio());
    let primeiro = it.next()?;
    let (mut x0, mut y0) = (primeiro.x, primeiro.y);
    let (mut x1, mut y1) = (primeiro.x + primeiro.w, primeiro.y + primeiro.h);
    for r in it {
        x0 = x0.min(r.x);
        y0 = y0.min(r.y);
        x1 = x1.max(r.x + r.w);
        y1 = y1.max(r.y + r.h);
    }
    Some(Ret::novo(x0, y0, x1 - x0, y1 - y0))
}

/// Desloca a cena para a origem do `bbox` (a janela desenha do próprio canto).
pub fn transladar(cena: &[Elemento], dx: i32, dy: i32) -> Vec<Elemento> {
    cena.iter()
        .map(|e| match *e {
            Elemento::Sprite {
                quadro,
                x,
                y,
                d,
                espelhar,
            } => Elemento::Sprite {
                quadro,
                x: x - dx,
                y: y - dy,
                d,
                espelhar,
            },
            Elemento::Bloco { ret, cor } => Elemento::Bloco {
                ret: Ret::novo(ret.x - dx, ret.y - dy, ret.w, ret.h),
                cor,
            },
            Elemento::Glifo { c, x, y, d, cor } => Elemento::Glifo {
                c,
                x: x - dx,
                y: y - dy,
                d,
                cor,
            },
        })
        .collect()
}

/// Rasteriza a cena num buffer BGRA pré-multiplicado do tamanho do `bbox`.
pub fn rasterizar(cena: &[Elemento], skin: &Skin, bbox: Ret) -> Vec<u8> {
    let (w, h) = (bbox.w.max(1), bbox.h.max(1));
    let mut dados = vec![0u8; (w * h * 4) as usize];
    let deslocada = transladar(cena, bbox.x, bbox.y);
    let mut alvo = Alvo::novo(&mut dados, w, h);
    let tela = alvo.limites();
    pet_core::cena::redesenhar(&mut alvo, &deslocada, skin, &[tela]);
    dados
}
