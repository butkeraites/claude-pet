//! Marcos que o pet lembra para não repetir (item do M6, decisão 0117): a
//! saudação da primeira aparição e o aviso de que o Claude está em silêncio.
//!
//! Puro: o Motor guarda e o núcleo do daemon grava em `/state` (JSON, troca
//! de uma vez) e lê na partida, como as posições ([`super::posicoes`]).
//! Diferente da memória das sessões ([`crate::memoria`]), não se prende ao
//! boot id da máquina: a primeira aparição é uma vez só, e um reinício da
//! máquina não a repete.

use serde::{Deserialize, Serialize};

/// Versão do arquivo.
pub const FORMATO: u32 = 1;

/// O que o pet já fez e não repete.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Marcos {
    /// O pet já cumprimentou na primeira aparição.
    pub ja_cumprimentou: bool,
    /// O pet já avisou que não chega sinal do Claude (o aviso some de novo
    /// quando um evento enfim chega).
    pub ja_avisou_sem_sinal: bool,
}

/// O arquivo em disco. Campos novos no futuro entram com `serde(default)`, e
/// um leitor velho ignora o que não conhece.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Arquivo {
    formato: u32,
    #[serde(default)]
    ja_cumprimentou: bool,
    #[serde(default)]
    ja_avisou_sem_sinal: bool,
}

impl Marcos {
    /// Lê o arquivo de `/state`. Formato desconhecido ou JSON quebrado: erro
    /// (o pet segue sem marcos, e cumprimenta como na primeira vez).
    pub fn ler(json: &str) -> Result<Marcos, String> {
        let arquivo: Arquivo =
            serde_json::from_str(json).map_err(|_| "o arquivo não é o JSON esperado".to_owned())?;
        if arquivo.formato != FORMATO {
            return Err(format!("formato {} desconhecido", arquivo.formato));
        }
        Ok(Marcos {
            ja_cumprimentou: arquivo.ja_cumprimentou,
            ja_avisou_sem_sinal: arquivo.ja_avisou_sem_sinal,
        })
    }

    /// O arquivo, do jeito que vai para `/state`.
    pub fn json(&self) -> String {
        let arquivo = Arquivo {
            formato: FORMATO,
            ja_cumprimentou: self.ja_cumprimentou,
            ja_avisou_sem_sinal: self.ja_avisou_sem_sinal,
        };
        serde_json::to_string_pretty(&arquivo).unwrap_or_default()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn grava_e_le_de_volta_igual() {
        let m = Marcos {
            ja_cumprimentou: true,
            ja_avisou_sem_sinal: true,
        };
        assert_eq!(Marcos::ler(&m.json()).unwrap(), m);
        // O padrão (nada feito ainda) ida e volta.
        let vazio = Marcos::default();
        assert_eq!(Marcos::ler(&vazio.json()).unwrap(), vazio);
    }

    #[test]
    fn arquivo_ruim_ou_de_outra_versao_e_erro() {
        assert!(Marcos::ler("lixo").is_err());
        assert!(Marcos::ler(r#"{"formato":2}"#).is_err());
        // Um campo que falta vale como falso (compatível com um arquivo mais
        // velho que só tinha a saudação).
        let so_um = Marcos::ler(r#"{"formato":1,"ja_cumprimentou":true}"#).unwrap();
        assert_eq!(
            so_um,
            Marcos {
                ja_cumprimentou: true,
                ja_avisou_sem_sinal: false,
            }
        );
    }
}
