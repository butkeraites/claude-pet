//! Quem aparece na tela (decisão 0011).
//!
//! - Com `PET_DEBUG=1`: a skin xadrez `_teste`, só para testar o motor.
//! - Sem debug: a skin configurada (`aparencia.skin`, padrão `zeca`),
//!   procurada nas pastas de `PET_SKINS`. A aprovação de personagem (folha
//!   de contato + snapshot em `/state`) só chega no M2; até lá nenhuma skin
//!   está aprovada e o pet fica escondido (`tela: sem_personagem`).
//! - A skin de teste **nunca** vira personagem, nem configurada à mão, nem
//!   como reserva de uma skin quebrada.

use std::path::{Path, PathBuf};

use pet_core::skin::Skin;

/// A skin xadrez de QA.
pub const SKIN_DE_TESTE: &str = "_teste";

#[derive(Debug)]
pub struct Escolha {
    /// A skin que vai para a tela, se houver.
    pub skin: Option<Skin>,
    /// A skin pedida (a de teste em debug, senão a configurada).
    pub pedida: String,
    /// Por que não há personagem, ou o que a skin carregada reclamou.
    pub avisos: Vec<String>,
}

/// Primeira pasta `<busca>/<id>` com um `skin.json`.
pub fn procurar(id: &str, busca: &[PathBuf]) -> Option<PathBuf> {
    busca
        .iter()
        .map(|pasta| pasta.join(id))
        .find(|pasta| pasta.join("skin.json").is_file())
}

fn carregar(pasta: &Path, avisos: &mut Vec<String>) -> Option<Skin> {
    match Skin::carregar(pasta) {
        Ok(skin) => {
            avisos.extend(skin.avisos.iter().cloned());
            Some(skin)
        }
        Err(e) => {
            avisos.push(format!("{}: {e}", pasta.display()));
            None
        }
    }
}

pub fn escolher(debug: bool, configurada: &str, busca: &[PathBuf]) -> Escolha {
    let mut avisos = Vec::new();
    if debug {
        let skin = match procurar(SKIN_DE_TESTE, busca) {
            Some(pasta) => carregar(&pasta, &mut avisos),
            None => {
                avisos.push(format!(
                    "skin de teste «{SKIN_DE_TESTE}» não encontrada em PET_SKINS"
                ));
                None
            }
        };
        return Escolha {
            skin,
            pedida: SKIN_DE_TESTE.into(),
            avisos,
        };
    }
    if configurada == SKIN_DE_TESTE {
        avisos.push("a skin de teste nunca é personagem (só com PET_DEBUG=1)".into());
    } else {
        match procurar(configurada, busca) {
            Some(pasta) => avisos.push(format!(
                "skin «{configurada}» encontrada em {}, mas ainda sem aprovação (chega no M2)",
                pasta.display()
            )),
            None => avisos.push(format!(
                "skin «{configurada}» não encontrada em PET_SKINS (o pack ainda não foi instalado)"
            )),
        }
    }
    Escolha {
        skin: None,
        pedida: configurada.to_owned(),
        avisos,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn skins_do_repo() -> Vec<PathBuf> {
        vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skins")]
    }

    #[test]
    fn debug_mostra_a_skin_de_teste() {
        let e = escolher(true, "zeca", &skins_do_repo());
        assert_eq!(e.skin.as_ref().map(|s| s.id.as_str()), Some("_teste"));
        assert!(e.avisos.is_empty(), "{:?}", e.avisos);
    }

    #[test]
    fn sem_debug_e_sem_aprovacao_fica_escondido() {
        let e = escolher(false, "zeca", &skins_do_repo());
        assert!(e.skin.is_none());
        assert_eq!(e.pedida, "zeca");
        assert!(e.avisos[0].contains("não encontrada"), "{:?}", e.avisos);
    }

    #[test]
    fn skin_de_teste_nunca_vira_personagem() {
        let e = escolher(false, "_teste", &skins_do_repo());
        assert!(e.skin.is_none());
        assert!(e.avisos[0].contains("nunca"), "{:?}", e.avisos);
    }

    #[test]
    fn skin_que_existe_mas_nao_foi_aprovada() {
        // Uma "zeca" de mentira (cópia da xadrez) numa pasta temporária:
        // achar a skin não basta para aparecer no M1.
        let raiz = std::env::temp_dir().join(format!("claude-pet-skins-{}", std::process::id()));
        let zeca = raiz.join("zeca");
        std::fs::create_dir_all(&zeca).unwrap();
        for nome in ["skin.json", "sheet.json", "sheet.png"] {
            std::fs::copy(
                skins_do_repo()[0].join("_teste").join(nome),
                zeca.join(nome),
            )
            .unwrap();
        }
        let e = escolher(false, "zeca", std::slice::from_ref(&raiz));
        let _ = std::fs::remove_dir_all(&raiz);
        assert!(e.skin.is_none());
        assert!(e.avisos[0].contains("sem aprovação"), "{:?}", e.avisos);
    }

    #[test]
    fn debug_sem_a_skin_de_teste_avisa() {
        let e = escolher(true, "zeca", &[PathBuf::from("/nao/existe")]);
        assert!(e.skin.is_none());
        assert_eq!(e.avisos.len(), 1);
    }
}
