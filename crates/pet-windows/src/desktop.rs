//! O [`Desktop`] do Windows (T8.7, parte Windows): a janela em foco (o terminal
//! de cada sessão do Claude) é seguida por um `SetWinEventHook` de
//! `EVENT_SYSTEM_FOREGROUND` (orientado a evento: toda troca entra na hora, com
//! o relógio de parede certo, mesmo quando o laço está devagar com o pet
//! escondido), com um polling de `GetForegroundWindow` como rede de segurança.
//! O clique a traz para a frente (`SetForegroundWindow`, com o
//! `AttachThreadInput` para vencer o foreground-lock). O anel de ativações e o
//! casamento com o `ts` do hook são do Motor (decisões 0055 e 0057), os mesmos
//! do M4/M5; aqui a "janela" é o **handle** dela (um número), nunca o título.

use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};

use pet_core::plataforma::{Alca, CapDesktop, Desktop, ErroFoco, EventoDesktop, InfoDesktop};

use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EVENT_SYSTEM_FOREGROUND, GetForegroundWindow, GetWindowThreadProcessId,
    IsIconic, IsWindow, SW_RESTORE, SetForegroundWindow, ShowWindow, WINEVENT_OUTOFCONTEXT,
};

/// As trocas de foco que o hook capturou, para o laço drenar. A `wndproc`/hook
/// e o laço rodam na mesma thread (o hook é `OUTOFCONTEXT`: o callback vem no
/// pump de mensagens), então um `thread_local` basta.
#[derive(Default)]
struct Foco {
    trocas: Vec<(String, u64)>,
}

thread_local! {
    static FOCO: RefCell<Foco> = RefCell::new(Foco::default());
}

/// Chamado pelo Windows quando uma janela vem para a frente. Só anota o handle
/// (número) e a hora; nunca o título.
unsafe extern "system" fn ao_trocar_foco(
    _hook: HWINEVENTHOOK,
    evento: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _thread: u32,
    _ms: u32,
) {
    // OBJID_WINDOW = 0: só a janela de topo que ganhou o foco.
    if evento == EVENT_SYSTEM_FOREGROUND && id_object == 0 && !hwnd.0.is_null() {
        let txt = hwnd_para_texto(hwnd);
        FOCO.with(|f| f.borrow_mut().trocas.push((txt, parede_ms())));
    }
}

#[derive(Default)]
pub struct DesktopWin {
    eventos: Vec<EventoDesktop>,
    iniciado: bool,
    /// O último handle em foco (como texto), para anotar só as trocas.
    ultimo: Option<String>,
    hook: Option<HWINEVENTHOOK>,
}

impl DesktopWin {
    pub fn novo() -> DesktopWin {
        DesktopWin::default()
    }

    /// Liga a fonte, instala o hook de foco e, a cada volta, emite as trocas que
    /// o hook capturou (com o relógio de parede do evento — o mesmo do `ts` do
    /// hook). Um polling de `GetForegroundWindow` fecha qualquer fresta.
    pub fn pollar(&mut self) {
        if !self.iniciado {
            self.iniciado = true;
            self.instalar_hook();
            self.eventos.push(EventoDesktop::Ligado(true));
            let frente = janela_em_foco();
            if let Some(alca) = &frente {
                self.eventos.push(EventoDesktop::JanelaInicial {
                    janela: Alca(alca.clone()),
                    parede_ms: parede_ms(),
                });
            }
            self.ultimo = frente;
            return;
        }

        // As trocas que o hook capturou, em ordem, com o tempo de cada uma.
        let trocas = FOCO.with(|f| std::mem::take(&mut f.borrow_mut().trocas));
        for (alca, quando) in trocas {
            if Some(&alca) != self.ultimo.as_ref() {
                self.eventos.push(EventoDesktop::JanelaAtiva {
                    janela: Some(Alca(alca.clone())),
                    parede_ms: quando,
                });
                self.ultimo = Some(alca);
            }
        }

        // Rede de segurança: se o hook perdeu uma troca, o foreground atual
        // ainda a pega (o `ultimo` evita repetir).
        let frente = janela_em_foco();
        if frente != self.ultimo {
            self.eventos.push(EventoDesktop::JanelaAtiva {
                janela: frente.clone().map(Alca),
                parede_ms: parede_ms(),
            });
            self.ultimo = frente;
        }
    }

    fn instalar_hook(&mut self) {
        // SAFETY: instala o hook de EVENT_SYSTEM_FOREGROUND. OUTOFCONTEXT: o
        // callback roda na nossa thread, no pump de mensagens; o módulo é nulo
        // (o callback está no processo). idprocess/idthread 0 = todos.
        let h = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                HMODULE::default(),
                Some(ao_trocar_foco),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if h.0.is_null() {
            aviso!("não instalei o SetWinEventHook do foco; sigo só com o polling");
        } else {
            self.hook = Some(h);
        }
    }
}

impl Drop for DesktopWin {
    fn drop(&mut self) {
        if let Some(h) = self.hook.take() {
            // SAFETY: desfaz o hook na mesma thread que o instalou.
            unsafe {
                let _ = UnhookWinEvent(h);
            }
        }
    }
}

impl Desktop for DesktopWin {
    fn capacidades(&self) -> CapDesktop {
        CapDesktop {
            // A janela segue o monitor sozinha (como o macOS).
            segue_foco: false,
            janela_ativa: true,
            foca_janela: true,
            nao_perturbe: false,
        }
    }

    fn focar(&mut self, alvo: &Alca) -> Result<(), ErroFoco> {
        let Some(hwnd) = alca_para_hwnd(alvo) else {
            return Err(ErroFoco::JanelaSumiu);
        };
        // SAFETY: handles do Win32; a janela pode ter sumido, conferido com
        // IsWindow antes de mexer nela.
        let ok = unsafe {
            if !IsWindow(hwnd).as_bool() {
                return Err(ErroFoco::JanelaSumiu);
            }
            // O foreground-lock recusa um SetForegroundWindow de quem não está
            // na frente; o truque é anexar a entrada da thread da janela em
            // foco à nossa enquanto trazemos a alvo para a frente.
            let fg = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg, None);
            let nossa = GetCurrentThreadId();
            let anexou = fg_thread != 0 && fg_thread != nossa;
            if anexou {
                let _ = AttachThreadInput(nossa, fg_thread, true);
            }
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let trouxe = SetForegroundWindow(hwnd).as_bool();
            let _ = BringWindowToTop(hwnd);
            if anexou {
                let _ = AttachThreadInput(nossa, fg_thread, false);
            }
            trouxe
        };
        if ok {
            info!("focando a janela {} (SetForegroundWindow)", alvo.0);
            Ok(())
        } else {
            Err(ErroFoco::Recusado("o Windows recusou o foco".into()))
        }
    }

    fn eventos(&mut self) -> Vec<EventoDesktop> {
        std::mem::take(&mut self.eventos)
    }

    fn janela_ativa(&self) -> Option<Alca> {
        janela_em_foco().map(Alca)
    }

    fn info(&self) -> InfoDesktop {
        let protocolos = if self.hook.is_some() {
            vec!["Win32".to_owned(), "WinEventHook".to_owned()]
        } else {
            vec!["Win32".to_owned()]
        };
        InfoDesktop {
            protocolos,
            janelas: usize::from(self.ultimo.is_some()),
        }
    }
}

/// O handle da janela em foco agora, como texto (`0x…`), ou `None` se não há
/// uma. Nunca o título.
fn janela_em_foco() -> Option<String> {
    // SAFETY: sem efeito colateral; devolve o handle (pode ser nulo).
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    Some(hwnd_para_texto(hwnd))
}

fn hwnd_para_texto(hwnd: HWND) -> String {
    format!("{:#x}", hwnd.0 as isize)
}

fn alca_para_hwnd(alca: &Alca) -> Option<HWND> {
    let s = alca.0.strip_prefix("0x").unwrap_or(&alca.0);
    let v = isize::from_str_radix(s, 16).ok()?;
    if v == 0 {
        return None;
    }
    Some(HWND(v as *mut core::ffi::c_void))
}

fn parede_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
