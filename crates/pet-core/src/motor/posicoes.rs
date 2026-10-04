//! Posições salvas por monitor (M4, decisão 0049).
//!
//! Onde o Renan deixou o pet em cada monitor, como fração do palco: o ponto
//! dos pés (a âncora `pe` da skin) dividido pelo tamanho do monitor em
//! pixels do dispositivo. Assim a posição sobrevive a outra escala, a outro
//! tamanho (`aparencia.tamanho`) e a outra skin; o corpo é preso de novo na
//! área útil ao voltar.
//!
//! A chave é a descrição do monitor (fabricante, modelo e série), com o nome
//! do conector como reserva: numa dock o `DP-3` vira `DP-5`, e a descrição
//! fica. O Hyprland põe o conector no fim da descrição do `wl_output`
//! (`… (eDP-1)`); esse pedaço sai da chave.
//!
//! Puro: o Motor guarda e o núcleo do daemon grava em `/state` (JSON, troca
//! de uma vez) e lê na partida.

use serde::{Deserialize, Serialize};

use crate::geometria::Ancoras;
use crate::plataforma::Monitor;

/// Versão do arquivo.
pub const FORMATO: u32 = 1;
/// Monitores lembrados; passou disso, o usado há mais tempo sai.
pub const MAX_MONITORES: usize = 32;
/// Maior chave aceita (uma descrição de monitor é curta).
const MAX_CHAVE: usize = 200;

/// Onde ficam os pés, como fração do palco (0 a 1 nos dois eixos).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Fracao {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Entrada {
    chave: String,
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Arquivo {
    formato: u32,
    monitores: Vec<Entrada>,
}

/// As posições, da usada há mais tempo à mais recente.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Posicoes {
    monitores: Vec<(String, Fracao)>,
}

/// A chave de um monitor: a descrição sem o conector do fim, ou o nome.
pub fn chave(monitor: &Monitor) -> Option<String> {
    let descricao = monitor.descricao.as_deref().map(|d| {
        let d = d.trim();
        match monitor.nome.as_deref() {
            Some(nome) => d
                .strip_suffix(&format!("({nome})"))
                .map_or(d, str::trim_end),
            None => d,
        }
    });
    let chave = match descricao.filter(|d| !d.is_empty()) {
        Some(d) => format!("descricao:{d}"),
        None => format!("nome:{}", monitor.nome.as_deref()?.trim()),
    };
    chave_valida(&chave).then_some(chave)
}

fn chave_valida(chave: &str) -> bool {
    !chave.is_empty() && chave.len() <= MAX_CHAVE && !chave.chars().any(char::is_control)
}

fn fracao_valida(f: &Fracao) -> bool {
    f.x.is_finite() && f.y.is_finite()
}

impl Fracao {
    /// A fração dos pés com a célula em (x, y), D `d`, num palco `tela`.
    pub fn da_celula(x: i32, y: i32, d: i32, ancoras: &Ancoras, tela: (i32, i32)) -> Fracao {
        let w = tela.0.max(1) as f64;
        let h = tela.1.max(1) as f64;
        Fracao {
            x: ((x + ancoras.pe.0 * d) as f64 / w).clamp(0.0, 1.0),
            y: ((y + ancoras.pe.1 * d) as f64 / h).clamp(0.0, 1.0),
        }
    }

    /// A célula com os pés na fração, D `d`, num palco `tela` (antes de
    /// prender na área útil).
    pub fn celula(&self, d: i32, ancoras: &Ancoras, tela: (i32, i32)) -> (i32, i32) {
        let px = (self.x.clamp(0.0, 1.0) * tela.0 as f64).round() as i32;
        let py = (self.y.clamp(0.0, 1.0) * tela.1 as f64).round() as i32;
        (px - ancoras.pe.0 * d, py - ancoras.pe.1 * d)
    }
}

impl Posicoes {
    /// Lê o arquivo de `/state`. Formato desconhecido ou JSON quebrado: erro
    /// (o pet segue com as posições padrão); entradas que não servem caem
    /// sozinhas.
    pub fn ler(json: &str) -> Result<Posicoes, String> {
        let arquivo: Arquivo =
            serde_json::from_str(json).map_err(|_| "o arquivo não é o JSON esperado".to_owned())?;
        if arquivo.formato != FORMATO {
            return Err(format!("formato {} desconhecido", arquivo.formato));
        }
        let mut posicoes = Posicoes::default();
        for e in arquivo.monitores {
            let fracao = Fracao { x: e.x, y: e.y };
            if chave_valida(&e.chave) && fracao_valida(&fracao) {
                posicoes.guardar(&e.chave, fracao);
            }
        }
        Ok(posicoes)
    }

    /// O arquivo, do jeito que vai para `/state`.
    pub fn json(&self) -> String {
        let arquivo = Arquivo {
            formato: FORMATO,
            monitores: self
                .monitores
                .iter()
                .map(|(chave, f)| Entrada {
                    chave: chave.clone(),
                    x: f.x,
                    y: f.y,
                })
                .collect(),
        };
        serde_json::to_string_pretty(&arquivo).unwrap_or_default()
    }

    /// Guarda (ou troca) a posição de um monitor: ela vira a mais recente.
    pub fn guardar(&mut self, chave: &str, fracao: Fracao) {
        if !chave_valida(chave) || !fracao_valida(&fracao) {
            return;
        }
        self.monitores.retain(|(c, _)| c != chave);
        if self.monitores.len() >= MAX_MONITORES {
            self.monitores.remove(0);
        }
        let fracao = Fracao {
            x: fracao.x.clamp(0.0, 1.0),
            y: fracao.y.clamp(0.0, 1.0),
        };
        self.monitores.push((chave.to_owned(), fracao));
    }

    pub fn de(&self, chave: &str) -> Option<Fracao> {
        self.monitores
            .iter()
            .find(|(c, _)| c == chave)
            .map(|(_, f)| *f)
    }

    pub fn quantas(&self) -> usize {
        self.monitores.len()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::geometria::Ret;

    fn ancoras() -> Ancoras {
        Ancoras {
            celula: (48, 48),
            pe: (24, 32),
            toque: Ret::novo(14, 13, 19, 19),
        }
    }

    fn edp(descricao: Option<&str>) -> Monitor {
        Monitor {
            nome: Some("eDP-1".into()),
            descricao: descricao.map(str::to_owned),
            logico: (1280, 800),
            escala: 1.5,
            ..Monitor::default()
        }
    }

    #[test]
    fn chave_e_a_descricao_sem_o_conector_ou_o_nome() {
        assert_eq!(
            chave(&edp(Some("Chimei Innolux Corporation 0x1459 (eDP-1)"))).as_deref(),
            Some("descricao:Chimei Innolux Corporation 0x1459")
        );
        assert_eq!(
            chave(&edp(Some("  Chimei Innolux Corporation 0x1459  "))).as_deref(),
            Some("descricao:Chimei Innolux Corporation 0x1459")
        );
        // Numa dock o conector muda e a chave fica.
        let mut dock = edp(Some("Dell Inc. DELL U2720Q 7LN4 (DP-5)"));
        dock.nome = Some("DP-5".into());
        let mut outra_porta = edp(Some("Dell Inc. DELL U2720Q 7LN4 (DP-3)"));
        outra_porta.nome = Some("DP-3".into());
        assert_eq!(chave(&dock), chave(&outra_porta));
        assert_eq!(chave(&edp(None)).as_deref(), Some("nome:eDP-1"));
        assert_eq!(chave(&edp(Some("   "))).as_deref(), Some("nome:eDP-1"));
        assert_eq!(chave(&edp(Some("(eDP-1)"))).as_deref(), Some("nome:eDP-1"));
        let sem_nada = Monitor::default();
        assert_eq!(chave(&sem_nada), None);
        assert_eq!(chave(&edp(Some("quebra\nde linha"))), None);
    }

    #[test]
    fn fracao_dos_pes_ida_e_volta() {
        let a = ancoras();
        let tela = (1920, 1200);
        let f = Fracao::da_celula(1698, 984, 6, &a, tela);
        assert!((f.x - (1698.0 + 144.0) / 1920.0).abs() < 1e-9);
        assert!((f.y - (984.0 + 192.0) / 1200.0).abs() < 1e-9);
        assert_eq!(f.celula(6, &a, tela), (1698, 984));
        // Em outro D (outro tamanho) e outro palco (4K), os pés ficam no
        // mesmo lugar relativo.
        let (x, y) = f.celula(11, &a, (3840, 2160));
        let pes = ((x + 24 * 11) as f64 / 3840.0, (y + 32 * 11) as f64 / 2160.0);
        assert!((pes.0 - f.x).abs() < 0.001 && (pes.1 - f.y).abs() < 0.001);
    }

    #[test]
    fn guardar_ler_e_gravar() {
        let mut p = Posicoes::default();
        p.guardar("descricao:A", Fracao { x: 0.25, y: 0.5 });
        p.guardar("nome:HDMI-A-1", Fracao { x: 1.5, y: -2.0 });
        assert_eq!(
            p.de("nome:HDMI-A-1"),
            Some(Fracao { x: 1.0, y: 0.0 }),
            "presa"
        );
        p.guardar("descricao:A", Fracao { x: 0.75, y: 0.5 });
        assert_eq!(p.quantas(), 2);
        let lido = Posicoes::ler(&p.json()).unwrap();
        assert_eq!(lido, p);
        assert_eq!(lido.de("descricao:A"), Some(Fracao { x: 0.75, y: 0.5 }));
        assert!(Posicoes::ler("lixo").is_err());
        assert!(Posicoes::ler(r#"{"formato":2,"monitores":[]}"#).is_err());
        // Entradas ruins caem sozinhas.
        let lido = Posicoes::ler(
            r#"{"formato":1,"monitores":[{"chave":"","x":0.1,"y":0.1},
            {"chave":"nome:X","x":0.2,"y":0.3}]}"#,
        )
        .unwrap();
        assert_eq!(lido.quantas(), 1);
        p.guardar(
            "nome:Y",
            Fracao {
                x: f64::NAN,
                y: 0.0,
            },
        );
        assert_eq!(p.de("nome:Y"), None, "NaN não entra");
    }

    #[test]
    fn lembra_no_maximo_32_monitores_e_esquece_o_mais_velho() {
        let mut p = Posicoes::default();
        for i in 0..MAX_MONITORES + 3 {
            p.guardar(&format!("nome:M{i}"), Fracao { x: 0.5, y: 0.5 });
        }
        assert_eq!(p.quantas(), MAX_MONITORES);
        assert_eq!(p.de("nome:M0"), None);
        assert!(p.de(&format!("nome:M{}", MAX_MONITORES + 2)).is_some());
        // Usar de novo um velho o põe no fim.
        p.guardar("nome:M3", Fracao { x: 0.1, y: 0.1 });
        p.guardar("nome:novo", Fracao { x: 0.1, y: 0.1 });
        assert!(p.de("nome:M3").is_some(), "{:?}", p.monitores.len());
        assert_eq!(p.de("nome:M4"), None);
    }
}
