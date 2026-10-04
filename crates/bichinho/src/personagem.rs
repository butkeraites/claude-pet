//! Quem aparece na tela (decisões 0011 e 0026).
//!
//! - Com `PET_DEBUG=1`: a skin xadrez `_teste`, só para testar o motor. Com
//!   `PET_DEBUG_PERSONAGEM=1` junto, o debug usa o personagem de verdade,
//!   com as mesmas regras de aprovação (para conferir o Zeca com as rotas de
//!   debug, como a nitidez).
//! - Sem debug: a skin configurada (`aparencia.skin`, padrão `zeca`),
//!   procurada nas pastas de `PET_SKINS`, **só se o Renan aprovou**: a da
//!   imagem, se a impressão digital dela é a aprovada; senão a cópia
//!   aprovada em `/state`; senão o pet fica escondido (`tela:
//!   sem_personagem`).
//! - A skin de teste **nunca** vira personagem, nem configurada à mão, nem
//!   como reserva de uma skin quebrada.

use std::path::{Path, PathBuf};

use pet_core::aprovacao::{Candidata, Decisao, Origem, decidir};
use pet_core::skin::Skin;

use crate::aprovacao;

/// A skin xadrez de QA.
pub const SKIN_DE_TESTE: &str = "_teste";

/// Quem está na tela: (id, impressão digital, origem). Uma aprovação que não
/// muda isso (aprovar de novo a mesma skin, revogar outra) não mexe na tela.
pub type NaTela = Option<(String, Option<String>, Option<Origem>)>;

/// A escolha nova é exatamente o personagem que já está na tela (aprovar de
/// novo a mesma skin, revogar a de outro id): nada a trocar.
pub fn mesma_tela(havia_skin: bool, antes: &NaTela, depois: &NaTela) -> bool {
    havia_skin && depois.is_some() && antes == depois
}

#[derive(Debug)]
pub struct Escolha {
    /// A skin que vai para a tela, se houver.
    pub skin: Option<Skin>,
    /// A skin pedida (a de teste em debug, senão a configurada).
    pub pedida: String,
    /// De onde veio o personagem aprovado (imagem ou cópia em `/state`).
    pub origem: Option<Origem>,
    /// Impressão digital da skin na tela.
    pub sha256: Option<String>,
    /// Por que não há personagem, ou o que a skin carregada reclamou.
    pub avisos: Vec<String>,
}

/// Onde procurar skins e aprovações, e o modo.
#[derive(Debug, Clone)]
pub struct Onde {
    /// Pastas de skins da imagem (`PET_SKINS`), em ordem.
    pub busca: Vec<PathBuf>,
    /// Estado persistente (`PET_ESTADO`, o volume `/state`).
    pub estado: PathBuf,
    pub debug: bool,
    /// Em debug, usar o personagem aprovado em vez da skin de teste.
    pub debug_personagem: bool,
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

fn sem_personagem(pedida: &str, avisos: Vec<String>) -> Escolha {
    Escolha {
        skin: None,
        pedida: pedida.to_owned(),
        origem: None,
        sha256: None,
        avisos,
    }
}

pub fn escolher(onde: &Onde, configurada: &str) -> Escolha {
    let mut avisos = Vec::new();
    if onde.debug && !onde.debug_personagem {
        let skin = match procurar(SKIN_DE_TESTE, &onde.busca) {
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
            origem: None,
            sha256: None,
            avisos,
        };
    }
    if configurada == SKIN_DE_TESTE {
        avisos.push("a skin de teste nunca é personagem (só com PET_DEBUG=1)".into());
        return sem_personagem(configurada, avisos);
    }
    let imagem = procurar(configurada, &onde.busca).map(|p| aprovacao::candidata(&p));
    let registro = match aprovacao::ler_registro(&onde.estado, configurada) {
        Ok(r) => r,
        Err(e) => {
            avisos.push(format!(
                "aprovação ilegível ({e}): conta como sem aprovação"
            ));
            None
        }
    };
    let snapshot = registro
        .as_ref()
        .and_then(|_| aprovacao::snapshot(&onde.estado, configurada));
    match decidir(configurada, imagem, registro.as_ref(), snapshot) {
        Decisao::Mostrar {
            candidata,
            origem,
            avisos: motivo,
        } => {
            let Candidata { skin, sha256 } = *candidata;
            avisos.extend(motivo);
            avisos.extend(skin.avisos.iter().cloned());
            Escolha {
                skin: Some(skin),
                pedida: configurada.to_owned(),
                origem: Some(origem),
                sha256: Some(sha256),
                avisos,
            }
        }
        Decisao::Esconder { avisos: motivo } => {
            avisos.extend(motivo);
            sem_personagem(configurada, avisos)
        }
    }
}

#[cfg(test)]
mod testes {
    use std::fs;

    use super::*;
    use crate::aprovacao::testes::Ambiente;

    fn onde(a: &Ambiente, debug: bool, debug_personagem: bool) -> Onde {
        Onde {
            busca: a.busca(),
            estado: a.estado.clone(),
            debug,
            debug_personagem,
        }
    }

    #[test]
    fn so_troca_a_tela_quando_muda_quem_esta_nela() {
        let zeca =
            |sha: &str, origem| Some(("zeca".to_owned(), Some(sha.to_owned()), Some(origem)));
        let a = zeca("aa", Origem::Imagem);
        assert!(mesma_tela(true, &a, &a), "aprovar de novo a mesma skin");
        assert!(
            !mesma_tela(true, &a, &zeca("bb", Origem::Imagem)),
            "skin nova"
        );
        assert!(
            !mesma_tela(true, &a, &zeca("aa", Origem::Snapshot)),
            "a da imagem quebrou: vai a cópia de /state"
        );
        assert!(!mesma_tela(true, &a, &None), "revogou: esconde");
        assert!(
            !mesma_tela(false, &None, &None),
            "nada antes, nada agora: publica de qualquer jeito"
        );
        assert!(
            !mesma_tela(false, &a, &a),
            "sem skin carregada ainda: troca"
        );
    }

    #[test]
    fn debug_mostra_a_skin_de_teste() {
        let a = Ambiente::novo("p-debug");
        let e = escolher(&onde(&a, true, false), "zeca");
        assert_eq!(e.skin.as_ref().map(|s| s.id.as_str()), Some("_teste"));
        assert!(e.avisos.is_empty(), "{:?}", e.avisos);
        assert_eq!(e.origem, None);
    }

    #[test]
    fn sem_aprovacao_fica_escondido() {
        let a = Ambiente::novo("p-sem");
        let e = escolher(&onde(&a, false, false), "zeca");
        assert!(e.skin.is_none());
        assert_eq!(e.pedida, "zeca");
        assert!(e.avisos[0].contains("sem aprovação"), "{:?}", e.avisos);
        // Nem o debug com personagem mostra sem aprovação.
        assert!(escolher(&onde(&a, true, true), "zeca").skin.is_none());
    }

    #[test]
    fn nao_instalada_avisa_para_instalar() {
        let a = Ambiente::novo("p-nada");
        let e = escolher(&onde(&a, false, false), "outra");
        assert!(e.skin.is_none());
        assert!(e.avisos[0].contains("skin-instalar"), "{:?}", e.avisos);
    }

    #[test]
    fn aprovada_aparece_da_imagem() {
        let a = Ambiente::novo("p-aprovada");
        aprovacao::aprovar("zeca", &a.sha(), &a.busca(), &a.estado).unwrap();
        let e = escolher(&onde(&a, false, false), "zeca");
        assert_eq!(e.skin.as_ref().map(|s| s.id.as_str()), Some("zeca"));
        assert_eq!(e.origem, Some(Origem::Imagem));
        assert_eq!(e.sha256, Some(a.sha()));
        // Debug com personagem: o mesmo Zeca aprovado, não a xadrez.
        let d = escolher(&onde(&a, true, true), "zeca");
        assert_eq!(d.skin.as_ref().map(|s| s.id.as_str()), Some("zeca"));
    }

    #[test]
    fn imagem_mudada_ou_quebrada_usa_a_copia_aprovada() {
        let a = Ambiente::novo("p-reserva");
        aprovacao::aprovar("zeca", &a.sha(), &a.busca(), &a.estado).unwrap();
        // Reconstruiu a skin: a imagem tem outra versão, ainda não aprovada.
        let skin_json = a.skins.join("zeca/skin.json");
        let texto = fs::read_to_string(&skin_json).unwrap();
        fs::write(
            &skin_json,
            texto.replace("\"corpo_px\": 32", "\"corpo_px\": 30"),
        )
        .unwrap();
        let e = escolher(&onde(&a, false, false), "zeca");
        assert_eq!(e.origem, Some(Origem::Snapshot));
        assert!(
            e.avisos[0].contains("mudou depois da aprovação"),
            "{:?}",
            e.avisos
        );
        assert_eq!(e.skin.as_ref().map(|s| s.corpo_px), Some(32), "a aprovada");
        // A da imagem quebrou: a cópia segura.
        fs::write(a.skins.join("zeca/sheet.png"), b"lixo").unwrap();
        assert_eq!(
            escolher(&onde(&a, false, false), "zeca").origem,
            Some(Origem::Snapshot)
        );
        // Revogou: escondido, mesmo com a cópia sumindo junto.
        aprovacao::revogar("zeca", &a.estado).unwrap();
        assert!(escolher(&onde(&a, false, false), "zeca").skin.is_none());
    }

    #[test]
    fn skin_de_teste_nunca_vira_personagem() {
        let a = Ambiente::novo("p-teste");
        let e = escolher(&onde(&a, false, false), "_teste");
        assert!(e.skin.is_none());
        assert!(e.avisos[0].contains("nunca"), "{:?}", e.avisos);
        assert!(escolher(&onde(&a, true, true), "_teste").skin.is_none());
    }

    #[test]
    fn debug_sem_a_skin_de_teste_avisa() {
        let e = escolher(
            &Onde {
                busca: vec![PathBuf::from("/nao/existe")],
                estado: PathBuf::from("/nao/existe"),
                debug: true,
                debug_personagem: false,
            },
            "zeca",
        );
        assert!(e.skin.is_none());
        assert_eq!(e.avisos.len(), 1);
    }
}
