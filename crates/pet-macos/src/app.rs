//! O `NSApplication` do daemon: app Accessory (sem Dock, sem menu), uma
//! atividade segurada contra o App Nap, e as peças do laço — a fatia do run
//! loop e o [`Despertador`] que a entrada HTTP usa para acordar a thread
//! principal de outra thread.

use objc2::MainThreadMarker;
use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_core_foundation::CFRunLoop;
use objc2_foundation::{NSActivityOptions, NSDate, NSProcessInfo, NSRunLoop, NSString};

/// Prepara o `NSApplication` como Accessory e segura uma atividade (App Nap).
/// Obtém o `MainThreadMarker` aqui dentro (o `bichinho` não depende do objc2);
/// o app compartilhado vive para sempre, então nada é devolvido.
pub fn preparar() {
    let mtm = MainThreadMarker::new().expect("o laço do macOS roda na thread principal");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    // Segura uma atividade enquanto o processo vive: o spike mostrou que o App
    // Nap não atrapalhou, mas é a garantia da pesquisa (decisão 0101). O token
    // é vazado de propósito (o daemon vive até morrer).
    let info = NSProcessInfo::processInfo();
    let razao = NSString::from_str("bichinho: o pet reage ao Claude Code");
    let token = info.beginActivityWithOptions_reason(NSActivityOptions::UserInitiated, &razao);
    std::mem::forget(token);

    app.finishLaunching();
}

/// Roda o run loop da thread principal por até `segundos` (processa os eventos
/// do AppKit e a fila principal). O [`Despertador`] faz esta espera terminar
/// antes quando chega algo na caixa.
pub fn rodar_fatia(segundos: f64) {
    autoreleasepool(|_| {
        let ate = NSDate::dateWithTimeIntervalSinceNow(segundos.max(0.0));
        NSRunLoop::currentRunLoop().runUntilDate(&ate);
    });
}

/// Acorda a thread principal de outra thread (a entrada HTTP): o
/// `CFRunLoop::wake_up` do run loop principal faz a fatia terminar na hora, e o
/// laço esvazia a caixa.
#[derive(Debug, Clone, Copy, Default)]
pub struct Despertador;

impl pet_core::plataforma::Despertador for Despertador {
    fn despertar(&self) {
        // `CFRunLoop::main`/`wake_up` são seguros de qualquer thread.
        if let Some(main) = CFRunLoop::main() {
            main.wake_up();
        }
    }
}
