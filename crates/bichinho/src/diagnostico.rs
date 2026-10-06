//! `bichinho diagnostico`: um relatório que o Renan cola. Diz o que o Claude
//! Code enxerga (o binário no PATH, de onde os hooks chamam), se o daemon
//! responde, o backend e as pastas; no macOS, se o plugin e o LaunchAgent
//! estão no lugar e o estado das permissões. Nenhum conteúdo, só metadados.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::ambiente::Ambiente;
use crate::daemon;

pub fn rodar() -> ExitCode {
    println!("bichinho diagnóstico");
    println!("  versão: {} (fonte {})", pet_core::VERSAO, crate::FONTE);
    if let Ok(exe) = std::env::current_exe() {
        println!("  este binário: {}", exe.display());
    }
    match achar_no_path("bichinho") {
        Some(p) => println!("  no PATH como «bichinho»: {}", p.display()),
        None => println!(
            "  no PATH como «bichinho»: NÃO — o hook do plugin não acha; \
             rode scripts/mac-instalar.sh"
        ),
    }

    let backend = if cfg!(target_os = "macos") {
        if std::env::var("PET_SEM_JANELA").as_deref() == Ok("1") {
            "macOS sem janela (PET_SEM_JANELA)"
        } else {
            "macOS nativo (NSPanel + CALayer)"
        }
    } else if cfg!(target_os = "linux") {
        "Linux (Wayland)"
    } else {
        "sem janela"
    };
    println!("  backend: {backend}");

    match Ambiente::ler(|n| std::env::var(n).ok()) {
        Ok(amb) => {
            println!("  escuta: {}", amb.escuta);
            let vivo = daemon::checar_saude(amb.escuta);
            println!(
                "  daemon: {}",
                if vivo {
                    "de pé (responde /saude)"
                } else {
                    "não responde — suba pelo LaunchAgent ou bichinho rodar"
                }
            );
            let onde = amb.onde();
            println!("  estado: {}", onde.estado.display());
            let config = amb.arquivo_config();
            println!(
                "  config: {} ({})",
                config.display(),
                if config.exists() {
                    "existe"
                } else {
                    "não existe (usa os padrões e o ambiente)"
                }
            );
        }
        Err(e) => println!("  ambiente: erro ({e})"),
    }

    #[cfg(target_os = "macos")]
    macos();

    ExitCode::SUCCESS
}

/// Procura `programa` nas pastas do `PATH` (o mesmo que o `command -v` faz).
fn achar_no_path(programa: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(programa))
        .find(|p| p.is_file())
}

#[cfg(target_os = "macos")]
fn macos() {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(home) = &home {
        let agente = home.join("Library/LaunchAgents/dev.bichinho.pet.plist");
        println!(
            "  LaunchAgent: {} ({})",
            agente.display(),
            if agente.exists() {
                "instalado (sobe no login)"
            } else {
                "não instalado (scripts/mac-instalar.sh --launchagent)"
            }
        );
    }
    let (acc, tela) = pet_macos::permissoes();
    println!(
        "  Acessibilidade (janela exata do terminal): {}",
        descreve_permissao(acc)
    );
    println!(
        "  Gravação de Tela (bichinho foto): {}",
        descreve_permissao(tela)
    );
    println!("  plugin: confira com «claude plugin list» (o hook é «bichinho avisar»)");
}

#[cfg(target_os = "macos")]
fn descreve_permissao(estado: Option<bool>) -> &'static str {
    match estado {
        Some(true) => "concedida",
        Some(false) => "não concedida (opcional; pedida quando usada)",
        None => "não dá para checar aqui",
    }
}
