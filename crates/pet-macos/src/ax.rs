//! Acessibilidade (AXUIElement): a janela exata do terminal da sessão (T8.7,
//! decisão 0106). Com a permissão de Acessibilidade, o clique levanta a janela
//! que estava em foco no app quando ele veio para a frente — sem nunca guardar
//! o título (só a referência opaca `AXUIElement`).
//!
//! `unsafe` com `// SAFETY:` em cada bloco; as funções AX são do framework
//! ApplicationServices (HIServices).

use std::ptr::NonNull;

use objc2_application_services::{
    AXError, AXIsProcessTrusted, AXIsProcessTrustedWithOptions, AXUIElement,
    kAXTrustedCheckOptionPrompt,
};
use objc2_core_foundation::{CFDictionary, CFRetained, CFString, CFType, kCFBooleanTrue};

/// Pergunta ao macOS se este processo tem a Acessibilidade e, se não, mostra o
/// diálogo pedindo (uma vez; depois o Renan concede nas Ajustes do Sistema).
/// Devolve se já está concedida.
pub fn pedir_confianca() -> bool {
    // {kAXTrustedCheckOptionPrompt: true}
    // SAFETY: ler os statics constantes do framework (uma CFString e o
    // CFBoolean verdadeiro).
    let (chave, valor): (&CFType, &CFType) = unsafe {
        (
            kAXTrustedCheckOptionPrompt,
            kCFBooleanTrue.expect("kCFBooleanTrue"),
        )
    };
    let opcoes = CFDictionary::from_slices(&[chave], &[valor]);
    let tipado: &CFDictionary<CFType, CFType> = &opcoes;
    // O `CFDictionary<CFType,CFType>` e o `CFDictionary` (opaco) que a função
    // pede têm o mesmo layout (os parâmetros são PhantomData).
    // SAFETY: reinterpretação entre instâncias do mesmo tipo que só diferem no
    // PhantomData; a função só lê o dicionário.
    unsafe {
        let opaco: &CFDictionary = &*core::ptr::from_ref(tipado).cast();
        AXIsProcessTrustedWithOptions(Some(opaco))
    }
}

/// Já tem a Acessibilidade (sem mostrar diálogo nenhum).
pub fn confiavel() -> bool {
    // SAFETY: só lê o estado do TCC.
    unsafe { AXIsProcessTrusted() }
}

/// A janela em foco do app de `pid`, se a Acessibilidade deixa. A referência é
/// opaca (nunca o título).
pub fn janela_em_foco(pid: i32) -> Option<CFRetained<AXUIElement>> {
    // SAFETY: new_application aceita um pid qualquer; devolve um AXUIElement.
    let app = unsafe { AXUIElement::new_application(pid) };
    let atributo = CFString::from_str("AXFocusedWindow");
    let mut valor: *const CFType = std::ptr::null();
    // SAFETY: `valor` é um ponteiro de saída válido.
    let erro = unsafe { app.copy_attribute_value(&atributo, NonNull::new(&mut valor).unwrap()) };
    if erro != AXError::Success {
        return None;
    }
    let bruto = NonNull::new(valor as *mut AXUIElement)?;
    // SAFETY: no sucesso, `valor` é um AXUIElement com +1 retain (regra Copy do
    // CoreFoundation), do qual tomamos posse.
    Some(unsafe { CFRetained::from_raw(bruto) })
}

/// Levanta (traz para a frente) a janela. `true` se o macOS aceitou.
pub fn levantar(janela: &AXUIElement) -> bool {
    let acao = CFString::from_str("AXRaise");
    // SAFETY: perform_action numa referência de janela válida.
    let erro = unsafe { janela.perform_action(&acao) };
    erro == AXError::Success
}
