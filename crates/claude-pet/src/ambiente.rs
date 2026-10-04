//! Variáveis `PET_*` internas: onde escutar, onde estão config e estado,
//! onde o runtime do host foi montado. Os padrões servem para rodar fora do
//! Docker (`cargo run`); o Dockerfile define os valores do container.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Porta padrão da entrada de eventos (decisão 0008).
pub const PORTA_PADRAO: u16 = 27380;

#[derive(Debug, Clone)]
pub struct Ambiente {
    /// Onde o servidor HTTP escuta (`PET_ESCUTA`).
    pub escuta: SocketAddr,
    /// Porta que os hooks enxergam no host (`PET_PORTA_PUBLICA`); vale para
    /// conferir o cabeçalho Host.
    pub porta_publica: u16,
    /// Pasta com `claude-pet.toml` (`PET_CONFIG`).
    pub pasta_config: PathBuf,
    /// Pasta do estado persistente (`PET_ESTADO`).
    pub pasta_estado: PathBuf,
    /// Onde o `/run/user` do host está montado (`PET_HOST_RUNTIME`); só a
    /// descoberta do Wayland usa.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub runtime_host: PathBuf,
    /// Pastas onde procurar skins, em ordem (`PET_SKINS`, separadas por `:`).
    pub skins: Vec<PathBuf>,
    /// Modo debug (`PET_DEBUG=1`): libera rotas e a skin `_teste`.
    pub debug: bool,
    /// Em debug, o personagem aprovado no lugar da skin `_teste`
    /// (`PET_DEBUG_PERSONAGEM=1`; decisão 0026).
    pub debug_personagem: bool,
}

impl Ambiente {
    pub fn ler(var: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let escuta = match var("PET_ESCUTA") {
            Some(texto) => texto
                .trim()
                .parse()
                .map_err(|_| format!("PET_ESCUTA inválida: «{texto}»"))?,
            None => SocketAddr::from(([127, 0, 0, 1], PORTA_PADRAO)),
        };
        let porta_publica = match var("PET_PORTA_PUBLICA") {
            Some(texto) => texto
                .trim()
                .parse()
                .map_err(|_| format!("PET_PORTA_PUBLICA inválida: «{texto}»"))?,
            None => escuta.port(),
        };
        let home = var("HOME").unwrap_or_else(|| "/tmp".to_owned());
        let pasta_estado = var("PET_ESTADO")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(home).join(".local/state/claude-pet"));
        Ok(Ambiente {
            escuta,
            porta_publica,
            pasta_config: var("PET_CONFIG")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("config")),
            pasta_estado,
            runtime_host: var("PET_HOST_RUNTIME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/run/user")),
            skins: var("PET_SKINS")
                .unwrap_or_else(|| "skins:skins-locais".to_owned())
                .split(':')
                .filter(|p| !p.trim().is_empty())
                .map(|p| PathBuf::from(p.trim()))
                .collect(),
            debug: var("PET_DEBUG").is_some_and(|v| v.trim() == "1"),
            debug_personagem: var("PET_DEBUG_PERSONAGEM").is_some_and(|v| v.trim() == "1"),
        })
    }

    /// Onde procurar skins e aprovações.
    pub fn onde(&self) -> crate::personagem::Onde {
        crate::personagem::Onde {
            busca: self.skins.clone(),
            estado: self.pasta_estado.clone(),
            debug: self.debug,
            debug_personagem: self.debug_personagem,
        }
    }

    pub fn arquivo_config(&self) -> PathBuf {
        self.pasta_config.join("claude-pet.toml")
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn padroes_para_rodar_fora_do_docker() {
        let a = Ambiente::ler(|_| None).unwrap();
        assert_eq!(a.escuta, "127.0.0.1:27380".parse().unwrap());
        assert_eq!(a.porta_publica, 27380);
        assert_eq!(a.arquivo_config(), PathBuf::from("config/claude-pet.toml"));
        assert_eq!(
            a.skins,
            vec![PathBuf::from("skins"), PathBuf::from("skins-locais")]
        );
        assert!(!a.debug);
    }

    #[test]
    fn valores_do_container() {
        let a = Ambiente::ler(|nome| match nome {
            "PET_ESCUTA" => Some("0.0.0.0:27380".into()),
            "PET_PORTA_PUBLICA" => Some("28000".into()),
            "PET_DEBUG" => Some("1".into()),
            "PET_SKINS" => Some("/opt/claude-pet/skins:/opt/claude-pet/skins-locais".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!(a.skins.len(), 2);
        assert_eq!(a.skins[1], PathBuf::from("/opt/claude-pet/skins-locais"));
        assert_eq!(a.escuta.port(), 27380);
        assert_eq!(a.porta_publica, 28000);
        assert!(a.debug);
        assert!(!a.debug_personagem);
        let p = Ambiente::ler(|n| (n == "PET_DEBUG_PERSONAGEM").then(|| "1".into())).unwrap();
        assert!(p.debug_personagem);
    }

    #[test]
    fn escuta_invalida_e_erro() {
        assert!(Ambiente::ler(|n| (n == "PET_ESCUTA").then(|| "nada".into())).is_err());
    }
}
