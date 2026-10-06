//! O [`Desktop`] do macOS (T8.7, parte macOS): o app em foco (o terminal de
//! cada sessão do Claude) vem do `NSWorkspace`, e o clique traz esse app para
//! a frente (`NSRunningApplication.activate`, sem permissão). O anel de
//! ativações e o casamento com o `ts` do hook são do Motor (decisões 0055 e
//! 0057), os mesmos do M4; aqui a "janela" é o **bundle id do app**, nunca o
//! título nem o nome da janela.
//!
//! Com a permissão de Acessibilidade (decisão 0106), o clique vai à **janela
//! exata**: a que estava em foco no app quando ele veio para a frente, por
//! `AXUIElement` (uma referência opaca; nunca o título).

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSWorkspace};
use objc2_application_services::AXUIElement;
use objc2_core_foundation::CFRetained;

use pet_core::plataforma::{Alca, CapDesktop, Desktop, ErroFoco, EventoDesktop, InfoDesktop};

use crate::ax;

#[derive(Default)]
pub struct DesktopMac {
    eventos: Vec<EventoDesktop>,
    ultimo: Option<String>,
    iniciado: bool,
    /// A janela em foco de cada app na última vez que ele veio para a frente
    /// (só com a Acessibilidade). A referência é opaca; nunca o título.
    janelas: HashMap<String, CFRetained<AXUIElement>>,
    /// Tem a Acessibilidade (reavaliado a cada volta: o Renan pode conceder
    /// depois).
    acc: bool,
    /// Já pediu a Acessibilidade uma vez (o diálogo do sistema).
    pediu: bool,
}

impl DesktopMac {
    pub fn novo() -> DesktopMac {
        DesktopMac::default()
    }

    /// Lê o app em foco (bundle id) e, se mudou, anota uma ativação no relógio
    /// de parede (o mesmo do `ts` do hook) e guarda a janela em foco dele (com
    /// a Acessibilidade). A primeira leitura liga a fonte, pede a
    /// Acessibilidade uma vez e semeia o anel. O laço chama a cada volta.
    pub fn pollar(&mut self) {
        // Pede a Acessibilidade uma vez; depois só confere (o Renan concede
        // nas Ajustes do Sistema, e aí a janela exata passa a valer).
        if !self.pediu {
            self.pediu = true;
            self.acc = ax::pedir_confianca();
            if !self.acc {
                aviso!(
                    "sem a permissão de Acessibilidade: o clique traz o app do terminal para a \
                     frente, mas não a janela exata. Conceda em Ajustes do Sistema › \
                     Privacidade e Segurança › Acessibilidade (decisão 0106)."
                );
            }
        } else {
            self.acc = ax::confiavel();
        }

        let frente = frontmost();
        let bundle = frente.as_ref().map(|(b, _)| b.clone());
        if !self.iniciado {
            self.iniciado = true;
            self.eventos.push(EventoDesktop::Ligado(true));
            if let Some((b, pid)) = &frente {
                self.lembrar_janela(b, *pid);
                self.eventos.push(EventoDesktop::JanelaInicial {
                    janela: Alca(b.clone()),
                    parede_ms: parede_ms(),
                });
            }
            self.ultimo = bundle;
            return;
        }
        if bundle != self.ultimo {
            if let Some((b, pid)) = &frente {
                self.lembrar_janela(b, *pid);
            }
            self.eventos.push(EventoDesktop::JanelaAtiva {
                janela: bundle.clone().map(Alca),
                parede_ms: parede_ms(),
            });
            self.ultimo = bundle;
        }
    }

    /// Guarda a janela em foco do app (só com a Acessibilidade).
    fn lembrar_janela(&mut self, bundle: &str, pid: i32) {
        if !self.acc {
            return;
        }
        if let Some(janela) = ax::janela_em_foco(pid) {
            self.janelas.insert(bundle.to_owned(), janela);
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
        if !app.activateWithOptions(NSApplicationActivationOptions::empty()) {
            return Err(ErroFoco::Recusado("o macOS recusou a ativação".into()));
        }
        // Com a Acessibilidade, levanta a janela exata daquele app.
        let exata = self
            .janelas
            .get(&alvo.0)
            .map(|j| ax::levantar(j))
            .unwrap_or(false);
        info!(
            "focando o app {} (NSRunningApplication.activate; janela exata: {})",
            alvo.0, exata
        );
        Ok(())
    }

    fn eventos(&mut self) -> Vec<EventoDesktop> {
        std::mem::take(&mut self.eventos)
    }

    fn janela_ativa(&self) -> Option<Alca> {
        frontmost().map(|(b, _)| Alca(b))
    }

    fn info(&self) -> InfoDesktop {
        let mut protocolos = vec!["NSWorkspace".to_owned()];
        if self.acc {
            protocolos.push("Acessibilidade".to_owned());
        }
        InfoDesktop {
            protocolos,
            janelas: usize::from(self.ultimo.is_some()),
        }
    }
}

/// O app em foco agora: `(bundle id, pid)`, se tiver (um app gráfico sempre
/// tem um bundle id).
fn frontmost() -> Option<(String, i32)> {
    let ws = NSWorkspace::sharedWorkspace();
    let app = ws.frontmostApplication()?;
    let bundle = app.bundleIdentifier()?.to_string();
    Some((bundle, app.processIdentifier()))
}

fn parede_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
