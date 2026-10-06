//! Coordenadas e pixels: a ponte entre o palco do Motor (pixels do
//! dispositivo, origem no canto superior esquerdo do monitor) e o AppKit
//! (pontos, origem no canto inferior esquerdo do desktop), e a cena virando um
//! `CGImage` BGRA pré-multiplicado para o `CALayer`.

use std::ffi::c_void;

use objc2_app_kit::NSScreen;
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{
    CGBitmapContextCreate, CGBitmapContextCreateImage, CGColorSpace, CGImage,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use pet_core::cena::Elemento;
use pet_core::geometria::Ret;
use pet_core::plataforma::Monitor;
use pet_core::raster::Alvo;
use pet_core::skin::Skin;

/// A geometria de um monitor, guardada para converter o palco em frame de
/// janela e o ponteiro de volta. O `NSScreen` fica guardado só pela geometria
/// (frame e escala); nunca sai nada dele para log.
#[derive(Clone)]
pub struct Tela {
    /// Frame do monitor em pontos (origem no canto inferior esquerdo do
    /// desktop).
    pub frame: NSRect,
    /// Pixels do dispositivo por ponto.
    pub escala: f64,
    pub nome: String,
    pub descricao: String,
}

impl Tela {
    pub fn de(tela: &NSScreen) -> Tela {
        let frame = tela.frame();
        let escala = tela.backingScaleFactor();
        let descricao = tela.localizedName().to_string();
        // Um id estável por tela, sem título: a descrição mais a posição.
        let nome = format!(
            "{}@{}x{}",
            descricao, frame.origin.x as i64, frame.origin.y as i64
        );
        Tela {
            frame,
            escala,
            nome,
            descricao,
        }
    }

    /// O [`Monitor`] que o Overlay anuncia ao Motor.
    pub fn monitor(&self, visivel: NSRect) -> Monitor {
        let f = self.frame;
        let s = self.escala;
        let logico = (f.size.width.round() as u32, f.size.height.round() as u32);
        // Área útil no palco (device px, origem no topo do monitor).
        let left = (visivel.origin.x - f.origin.x) * s;
        let top = ((f.origin.y + f.size.height) - (visivel.origin.y + visivel.size.height)) * s;
        let w = visivel.size.width * s;
        let h = visivel.size.height * s;
        Monitor {
            nome: Some(self.nome.clone()),
            descricao: Some(self.descricao.clone()),
            logico,
            escala: s,
            origem: Some((f.origin.x.round() as i32, f.origin.y.round() as i32)),
            area_util: Some(Ret::novo(
                left.round() as i32,
                top.round() as i32,
                w.round() as i32,
                h.round() as i32,
            )),
        }
    }

    /// Um retângulo do palco (device px, topo do monitor) vira o frame da
    /// janela em pontos (canto inferior esquerdo do desktop).
    pub fn palco_para_frame(&self, bbox: Ret) -> NSRect {
        let s = self.escala;
        let lx = bbox.x as f64 / s;
        let ly = bbox.y as f64 / s; // distância do topo do monitor
        let lw = bbox.w as f64 / s;
        let lh = bbox.h as f64 / s;
        let origin_x = self.frame.origin.x + lx;
        let origin_y = self.frame.origin.y + self.frame.size.height - (ly + lh);
        NSRect::new(NSPoint::new(origin_x, origin_y), NSSize::new(lw, lh))
    }

    /// Um ponto do desktop (pontos, origem no canto inferior esquerdo da tela
    /// principal) vira um ponto do palco deste monitor (device px, topo do
    /// monitor). Para o click-through pelo plano B (`NSEvent.mouseLocation`).
    pub fn desktop_para_palco(&self, p: NSPoint) -> (i32, i32) {
        let s = self.escala;
        let lx = p.x - self.frame.origin.x;
        let ly = (self.frame.origin.y + self.frame.size.height) - p.y;
        ((lx * s).round() as i32, (ly * s).round() as i32)
    }

    /// Um ponto local da janela (pontos, origem no canto inferior esquerdo da
    /// janela) vira um ponto do palco (device px, topo do monitor). `bbox` é o
    /// retângulo do palco que a janela cobre agora.
    pub fn janela_para_palco(&self, bbox: Ret, local: NSPoint) -> (i32, i32) {
        let s = self.escala;
        let lh = bbox.h as f64 / s;
        let px = bbox.x as f64 + local.x * s;
        let py = bbox.y as f64 + (lh - local.y) * s;
        (px.round() as i32, py.round() as i32)
    }
}

/// A caixa que contém a cena inteira no palco (a união dos limites de cada
/// elemento). `None` se a cena está vazia.
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

/// Um `CGImage` BGRA pré-multiplicado a partir do buffer.
pub fn cgimage(w: i32, h: i32, dados: &[u8]) -> Option<CFRetained<CGImage>> {
    // kCGImageAlphaPremultipliedFirst (2) | kCGBitmapByteOrder32Little (2<<12)
    const BITMAP_INFO: u32 = 2 | (2 << 12);
    let espaco = CGColorSpace::new_device_rgb()?;
    // SAFETY: os parâmetros batem com o buffer (8 bits por componente, 4 bytes
    // por pixel, passo w*4); o CGBitmapContextCreateImage copia os bytes, então
    // `dados` só precisa viver até o fim desta função.
    unsafe {
        let ctx = CGBitmapContextCreate(
            dados.as_ptr() as *mut c_void,
            w as usize,
            h as usize,
            8,
            (w * 4) as usize,
            Some(&espaco),
            BITMAP_INFO,
        )?;
        CGBitmapContextCreateImage(Some(&ctx))
    }
}

/// O ponteiro para passar ao `setContents:` do CALayer (um CGImageRef é aceito
/// como `id`, o `(__bridge id)` do ObjC).
pub fn contents_ptr(img: &CFRetained<CGImage>) -> *const objc2::runtime::AnyObject {
    let ptr: *const CGImage = &**img;
    ptr.cast()
}
