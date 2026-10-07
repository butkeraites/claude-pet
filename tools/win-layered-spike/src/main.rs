//! Spike do overlay do Windows (M8): uma janela layered (alfa por pixel),
//! topmost, sem foco e click-through, desenhada com `UpdateLayeredWindow` — a
//! mesma técnica que o `pet-windows` vai usar. Desenha um disco magenta com um
//! anel branco (o lugar do Zeca) e fica num laço de mensagens até ser morto; o
//! workflow tira o screenshot e eu confiro que apareceu na tela.

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() {
    win::rodar();
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
        DIB_RGB_COLORS, DeleteDC, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW,
        SW_SHOWNA, ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WNDCLASSW, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };
    use windows::core::w;

    const W: i32 = 220;
    const H: i32 = 220;
    const X: i32 = 260;
    const Y: i32 = 180;

    extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
    }

    /// Pinta o disco magenta com anel branco, em BGRA pré-multiplicado.
    fn pintar(bits: *mut u8) {
        let cx = W as f32 / 2.0;
        let cy = H as f32 / 2.0;
        let r = 100.0f32;
        for y in 0..H {
            for x in 0..W {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let (b, g, r8, a): (u8, u8, u8, u8) = if dist <= r {
                    let (cr, cg, cb): (u32, u32, u32) = if dist >= r - 6.0 {
                        (255, 255, 255) // anel branco
                    } else {
                        (224, 50, 200) // magenta
                    };
                    let alpha: u32 = 235;
                    let pm = |c: u32| ((c * alpha) / 255) as u8;
                    (pm(cb), pm(cg), pm(cr), alpha as u8)
                } else {
                    (0, 0, 0, 0)
                };
                // SAFETY: o buffer tem W*H*4 bytes (CreateDIBSection); o índice fica dentro.
                unsafe {
                    let p = bits.add(((y * W + x) * 4) as usize);
                    *p = b;
                    *p.add(1) = g;
                    *p.add(2) = r8;
                    *p.add(3) = a;
                }
            }
        }
    }

    pub fn rodar() {
        // SAFETY: sequência padrão do Win32 (classe, janela layered, DIB,
        // UpdateLayeredWindow, laço de mensagens); handles conferidos.
        unsafe {
            let hmod = GetModuleHandleW(None).expect("GetModuleHandleW");
            let hinst = HINSTANCE(hmod.0);
            let classe = w!("ZecaSpikeClass");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: hinst,
                lpszClassName: classe,
                ..Default::default()
            };
            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT,
                classe,
                w!("Zeca spike"),
                WS_POPUP,
                X,
                Y,
                W,
                H,
                None,
                None,
                hinst,
                None,
            )
            .expect("CreateWindowExW");

            // DIB top-down 32bpp (altura negativa).
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: W,
                    biHeight: -H,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let screen = GetDC(None);
            let memdc = CreateCompatibleDC(screen);
            let mut bits: *mut c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(screen, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                .expect("CreateDIBSection");
            pintar(bits.cast());
            let antigo = SelectObject(memdc, HGDIOBJ(dib.0));

            let blend = BLENDFUNCTION {
                BlendOp: 0,       // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1,   // AC_SRC_ALPHA
            };
            let dst = POINT { x: X, y: Y };
            let src = POINT { x: 0, y: 0 };
            let size = SIZE { cx: W, cy: H };
            UpdateLayeredWindow(
                hwnd,
                screen,
                Some(&dst),
                Some(&size),
                memdc,
                Some(&src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
            .expect("UpdateLayeredWindow");

            let _ = ShowWindow(hwnd, SW_SHOWNA);
            SelectObject(memdc, antigo);
            let _ = DeleteDC(memdc);
            ReleaseDC(None, screen);

            println!("janela layered criada ({W}x{H}); laço de mensagens");
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = DispatchMessageW(&msg);
            }
        }
    }
}
