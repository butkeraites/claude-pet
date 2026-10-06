//! O [`Desktop`] do macOS (T8.7, parte macOS): o app em foco (o terminal de
//! cada sessão do Claude) vem do `NSWorkspace`, e o clique traz esse app para
//! a frente (`NSRunningApplication.activate`, sem permissão). O anel de
//! ativações e o casamento com o `ts` do hook são do Motor (decisões 0055 e
//! 0057), os mesmos do M4; aqui a "janela" é o **bundle id do app**, nunca o
//! título nem o nome da janela.
//!
//! A janela exata (a aba certa, por `AXUIElement` com a permissão de
//! Acessibilidade) fica para a segunda parte do T8.7.

use std::time::{SystemTime, UNIX_EPOCH};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSWorkspace};

use pet_core::plataforma::{Alca, CapDesktop, Desktop, ErroFoco, EventoDesktop, InfoDesktop};

#[derive(Default)]
pub struct DesktopMac {
    eventos: Vec<EventoDesktop>,
    ultimo: Option<String>,
    iniciado: bool,
}

impl DesktopMac {
    pub fn novo() -> DesktopMac {
        DesktopMac::default()
    }

    /// Lê o app em foco (bundle id) e, se mudou, anota uma ativação no relógio
    /// de parede (o mesmo do `ts` do hook). A primeira leitura liga a fonte e
    /// anota a semente do anel. O laço chama a cada volta.
    pub fn pollar(&mut self) {
        let agora_frente = frontmost_bundle();
        if !self.iniciado {
            self.iniciado = true;
            self.eventos.push(EventoDesktop::Ligado(true));
            if let Some(bundle) = agora_frente.clone() {
                self.eventos.push(EventoDesktop::JanelaInicial {
                    janela: Alca(bundle),
                    parede_ms: parede_ms(),
                });
            }
            self.ultimo = agora_frente;
            return;
        }
        if agora_frente != self.ultimo {
            self.eventos.push(EventoDesktop::JanelaAtiva {
                janela: agora_frente.clone().map(Alca),
                parede_ms: parede_ms(),
            });
            self.ultimo = agora_frente;
        }
    }
}

impl Desktop for DesktopMac {
    fn capacidades(&self) -> CapDesktop {
        CapDesktop {
            // O painel segue o monitor sozinho (NSScreen.main); aqui não.
            segue_foco: false,
            janela_ativa: true,
            foca_janela: true,
            nao_perturbe: false,
        }
    }

    fn focar(&mut self, alvo: &Alca) -> Result<(), ErroFoco> {
        let Some(mtm) = MainThreadMarker::new() else {
            return Err(ErroFoco::Recusado("fora da thread principal".into()));
        };
        let ws = NSWorkspace::sharedWorkspace();
        let rodando = ws.runningApplications();
        let app = rodando.iter().find(|a| {
            a.bundleIdentifier()
                .map(|b| b.to_string() == alvo.0)
                .unwrap_or(false)
        });
        let Some(app) = app else {
            return Err(ErroFoco::JanelaSumiu);
        };
        // macOS 14+: a ativação é cooperativa. Cedemos a ativação para o app
        // alvo e o trazemos para a frente (o spike mostrou que funciona).
        let nosso = NSApplication::sharedApplication(mtm);
        nosso.yieldActivationToApplication(&app);
        if app.activateWithOptions(NSApplicationActivationOptions::empty()) {
            info!("focando o app {} (NSRunningApplication.activate)", alvo.0);
            Ok(())
        } else {
            Err(ErroFoco::Recusado("o macOS recusou a ativação".into()))
        }
    }

    fn eventos(&mut self) -> Vec<EventoDesktop> {
        std::mem::take(&mut self.eventos)
    }

    fn janela_ativa(&self) -> Option<Alca> {
        frontmost_bundle().map(Alca)
    }

    fn info(&self) -> InfoDesktop {
        InfoDesktop {
            protocolos: vec!["NSWorkspace".to_owned()],
            janelas: usize::from(self.ultimo.is_some()),
        }
    }
}

/// O bundle id do app em foco agora, se tiver (um app gráfico sempre tem).
fn frontmost_bundle() -> Option<String> {
    let ws = NSWorkspace::sharedWorkspace();
    ws.frontmostApplication()?
        .bundleIdentifier()
        .map(|b| b.to_string())
}

fn parede_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
