//! As peças do laço no Windows: a preparação do processo (DPI por-monitor v2,
//! para os pixels baterem com o palco), a fatia do laço de mensagens
//! (`MsgWaitForMultipleObjectsEx` + `PeekMessageW`, como o `rodar_fatia` do
//! macOS) e o [`Despertador`] que a entrada HTTP usa para acordar a thread
//! principal de outra thread (`PostThreadMessageW`).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows::Win32::Foundation::{BOOL, LPARAM, TRUE, WPARAM};
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE,
    PeekMessageW, PostThreadMessageW, QS_ALLINPUT, TranslateMessage, WM_NULL,
};

/// O id da thread principal (o laço), para o [`Despertador`] postar nela de
/// outra thread. `0` antes de `preparar`.
static THREAD_ID: AtomicU32 = AtomicU32::new(0);

/// Marcado quando o console pede para encerrar (Ctrl+C, fechar a janela). O
/// laço confere a cada volta e sai com calma (grava a memória, fecha a janela).
static ENCERRAR: AtomicBool = AtomicBool::new(false);

/// Acorda a thread principal postando uma `WM_NULL` nela (de qualquer thread).
fn acordar_thread() {
    let tid = THREAD_ID.load(Ordering::SeqCst);
    if tid != 0 {
        // SAFETY: `PostThreadMessageW` é seguro de qualquer thread; a WM_NULL
        // não carrega dado e é ignorada no despacho.
        unsafe {
            let _ = PostThreadMessageW(tid, WM_NULL, WPARAM(0), LPARAM(0));
        }
    }
}

/// Prepara o processo: DPI por-monitor v2 (os `HMONITOR`/posições em pixels
/// físicos batem com o palco do Motor) e guarda o id da thread principal.
pub fn preparar() {
    // SAFETY: chamada única no começo do processo, sem janelas ainda; o
    // Windows aceita o contexto antes de qualquer HWND top-level.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    // SAFETY: sem efeito colateral; devolve o id da thread atual.
    let tid = unsafe { GetCurrentThreadId() };
    THREAD_ID.store(tid, Ordering::SeqCst);
}

/// Roda o laço de mensagens por até `segundos`: dorme em
/// `MsgWaitForMultipleObjectsEx` (acorda com mensagem, ou no prazo, ou quando o
/// [`Despertador`] posta) e despacha tudo o que estiver na fila. Sem mensagens,
/// retorna no prazo — é a fatia entre duas voltas do laço.
pub fn rodar_fatia(segundos: f64) {
    let ms = (segundos * 1000.0).clamp(0.0, u32::MAX as f64) as u32;
    // SAFETY: APIs padrão do laço de mensagens Win32; os ponteiros das MSG são
    // locais e válidos durante a chamada.
    unsafe {
        MsgWaitForMultipleObjectsEx(None, ms, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Acorda a thread principal de outra thread (a entrada HTTP): posta uma
/// mensagem de thread, que faz o `MsgWaitForMultipleObjectsEx` da fatia voltar
/// na hora. Nenhum dado trafega — só o acorda.
#[derive(Debug, Clone, Copy, Default)]
pub struct Despertador;

impl pet_core::plataforma::Despertador for Despertador {
    fn despertar(&self) {
        acordar_thread();
    }
}

/// Instala o tratador de Ctrl+C / fechar o console: marca o pedido de
/// encerramento e acorda a thread principal, que sai com calma na próxima
/// volta. Chamar uma vez, na thread principal.
pub fn instalar_encerramento() {
    // SAFETY: registra o tratador uma vez; `ctrl_handler` é uma `extern
    // "system" fn` válida.
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(ctrl_handler), true);
    }
}

/// `true` depois de um Ctrl+C ou fechar o console. O laço confere e sai.
pub fn pediram_encerrar() -> bool {
    ENCERRAR.load(Ordering::SeqCst)
}

extern "system" fn ctrl_handler(_tipo: u32) -> BOOL {
    ENCERRAR.store(true, Ordering::SeqCst);
    acordar_thread();
    TRUE
}
