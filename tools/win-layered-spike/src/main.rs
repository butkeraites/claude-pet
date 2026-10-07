//! Spike do overlay do Windows (M8): uma janela layered (alfa por pixel),
//! topmost, sem foco e click-through, com o **Zeca de verdade** rasterizado
//! pelo `pet-core` e desenhado com `UpdateLayeredWindow` — a mesma técnica que
//! o `pet-windows` vai usar. Fica num laço de mensagens até ser morto; o
//! workflow tira o screenshot e eu confiro que o Zeca apareceu na tela.

/// Rasteriza a pose `idle` do Zeca num buffer BGRA pré-multiplicado (o que o
/// `UpdateLayeredWindow` quer — e o que o raster do pet-core já produz), e
/// devolve (largura, altura, bytes). A skin vai embutida (self-contido).
fn zeca_idle() -> (i32, i32, Vec<u8>) {
    use pet_core::geometria::Ret;
    use pet_core::raster::{Alvo, desenhar_quadro};
    use pet_core::skin::Skin;

    let skin = Skin::de_partes(
        include_str!("../../../skins/zeca-livre-escuro/skin.json"),
        include_str!("../../../skins/zeca-livre-escuro/sheet.json"),
        include_bytes!("../../../skins/zeca-livre-escuro/sheet.png"),
    )
    .expect("skin zeca-livre-escuro");

    // A pose fixa do repouso: o primeiro quadro da primeira tag do estado idle.
    let tag_idle = &skin.estados["idle"][0];
    let q = skin
        .tags
        .iter()
        .find(|t| &t.nome == tag_idle)
        .expect("tag idle")
        .de;

    let d: i32 = 8; // bloco D×D; tamanho bom para o spike
    let (cw, ch) = (skin.ancoras.celula.0, skin.ancoras.celula.1);
    let (w, h) = (cw * d, ch * d);

    let mut dados = vec![0u8; (w * h * 4) as usize];
    {
        let mut alvo = Alvo::novo(&mut dados, w, h);
        let full = Ret::novo(0, 0, w, h);
        desenhar_quadro(&mut alvo, &skin, q, 0, 0, d, false, full);
    }
    (w, h, dados)
}

#[cfg(not(windows))]
fn main() {
    // Fora do Windows: só valida que a rasterização roda (o raster é puro).
    let (w, h, dados) = zeca_idle();
    assert_eq!(dados.len(), (w * h * 4) as usize);
    println!("Zeca idle rasterizado: {w}x{h}");
}

#[cfg(windows)]
fn main() {
    win::rodar();
}

#[cfg(windows)]
mod win {
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

    const X: i32 = 300;
    const Y: i32 = 180;

    extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
    }

    pub fn rodar() {
        let (w, h, dados) = super::zeca_idle();

        // SAFETY: sequência padrão do Win32 (classe, janela layered, DIB,
        // UpdateLayeredWindow, laço de mensagens); o buffer tem w*h*4 bytes.
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
                w!("Zeca"),
                WS_POPUP,
                X,
                Y,
                w,
                h,
                None,
                None,
                hinst,
                None,
            )
            .expect("CreateWindowExW");

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
            let dib = CreateDIBSection(screen, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                .expect("CreateDIBSection");
            // O raster já entrega BGRA pré-multiplicado: copia direto.
            std::ptr::copy_nonoverlapping(dados.as_ptr(), bits.cast::<u8>(), dados.len());
            let antigo = SelectObject(memdc, HGDIOBJ(dib.0));

            let blend = BLENDFUNCTION {
                BlendOp: 0,     // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1, // AC_SRC_ALPHA
            };
            let dst = POINT { x: X, y: Y };
            let src = POINT { x: 0, y: 0 };
            let size = SIZE { cx: w, cy: h };
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

            println!("Zeca desenhado ({w}x{h}); laço de mensagens");
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = DispatchMessageW(&msg);
            }
        }
    }
}
