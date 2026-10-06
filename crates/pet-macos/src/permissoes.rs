//! Estado das permissões do macOS para o `bichinho diagnostico`: a
//! Acessibilidade (a janela exata do terminal, T8.7) e a Gravação de Tela (o
//! `bichinho foto`). As duas são opcionais e pedidas quando usadas.
//!
//! São duas funções C sem argumento, declaradas à mão (o objc2 não as expõe):
//! `AXIsProcessTrusted` (ApplicationServices) e `CGPreflightScreenCaptureAccess`
//! (CoreGraphics). Checam **este processo**; como o binário é o mesmo (a mesma
//! assinatura ad-hoc) do daemon, o TCC costuma valer para os dois.

// SAFETY: as duas funções só leem o estado do TCC, sem argumento nem ponteiro.
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

// SAFETY: idem, do CoreGraphics.
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> u8;
}

/// `(acessibilidade, gravação_de_tela)`: `Some(true)` concedida, `Some(false)`
/// não, deste processo.
pub fn permissoes() -> (Option<bool>, Option<bool>) {
    // SAFETY: chamadas sem argumento que só consultam o TCC.
    let acc = unsafe { AXIsProcessTrusted() != 0 };
    // SAFETY: idem.
    let tela = unsafe { CGPreflightScreenCaptureAccess() != 0 };
    (Some(acc), Some(tela))
}
