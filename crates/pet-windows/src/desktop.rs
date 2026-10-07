//! O [`Desktop`] do Windows (T8.7, parte Windows): a janela em foco (o terminal
//! de cada sessão do Claude) vem do `GetForegroundWindow`, e o clique a traz
//! para a frente (`SetForegroundWindow`, com o `AttachThreadInput` para vencer o
//! foreground-lock). O anel de ativações e o casamento com o `ts` do hook são do
//! Motor (decisões 0055 e 0057), os mesmos do M4/M5; aqui a "janela" é o
//! **handle** dela (um número), nunca o título nem o nome.

use std::time::{SystemTime, UNIX_EPOCH};

use pet_core::plataforma::{Alca, CapDesktop, Desktop, ErroFoco, EventoDesktop, InfoDesktop};

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow,
    SW_RESTORE, SetForegroundWindow, ShowWindow,
};

#[derive(Default)]
pub struct DesktopWin {
    eventos: Vec<EventoDesktop>,
    iniciado: bool,
    /// O último handle em foco (como texto), para anotar só as trocas.
    ultimo: Option<String>,
}

impl DesktopWin {
    pub fn novo() -> DesktopWin {
        DesktopWin::default()
    }

    /// Lê a janela em foco agora e, se mudou, anota uma ativação no relógio de
    /// parede (o mesmo do `ts` do hook). A primeira leitura liga a fonte e
    /// semeia o anel. O laço chama a cada volta.
    pub fn pollar(&mut self) {
        let frente = janela_em_foco();
        if !self.iniciado {
            self.iniciado = true;
            self.eventos.push(EventoDesktop::Ligado(true));
            if let Some(alca) = &frente {
                self.eventos.push(EventoDesktop::JanelaInicial {
                    janela: Alca(alca.clone()),
                    parede_ms: parede_ms(),
                });
            }
            self.ultimo = frente;
            return;
        }
        if frente != self.ultimo {
            self.eventos.push(EventoDesktop::JanelaAtiva {
                janela: frente.clone().map(Alca),
                parede_ms: parede_ms(),
            });
            self.ultimo = frente;
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
        InfoDesktop {
            protocolos: vec!["Win32".to_owned()],
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
