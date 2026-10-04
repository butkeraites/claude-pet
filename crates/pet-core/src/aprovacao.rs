//! Aprovação do personagem (decisão 0026): o Renan aprova o **conteúdo
//! exato** de uma skin, depois de ver a folha de contato.
//!
//! - **Impressão digital:** o sha256 do texto que o `sha256sum` imprime para
//!   os três arquivos da skin, na ordem `skin.json`, dados, folha (os nomes
//!   do `skin.json`). No host, `sha256sum skin.json sheet.json sheet.png |
//!   sha256sum` dá o mesmo valor: é assim que o `bin/pet skin-aprovar`
//!   prova que a skin da imagem é a que o Renan viu.
//! - **Registro:** `aprovacao.json` com id, impressão e hora, junto de uma
//!   cópia dos três arquivos aprovados (o snapshot), no volume `/state`.
//! - **Decisão:** a skin da imagem aparece se a impressão dela é a
//!   aprovada; senão, o snapshot aprovado (reserva, se a da imagem quebrou ou
//!   mudou); senão, o pet fica escondido. Nunca a skin de teste.
//!
//! Aqui só a parte pura (cálculo e decisão); ler e gravar o `/state` é do
//! daemon.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::skin::Skin;

/// Bytes → hexadecimal minúsculo.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `sha256sum` de cada arquivo, como texto, e o sha256 desse texto.
pub fn impressao(arquivos: &[(&str, &[u8])]) -> String {
    let mut lista = String::new();
    for (nome, dados) in arquivos {
        lista.push_str(&hex(&Sha256::digest(dados)));
        lista.push_str("  ");
        lista.push_str(nome);
        lista.push('\n');
    }
    hex(&Sha256::digest(lista.as_bytes()))
}

/// Os três arquivos da skin na ordem da impressão: `skin.json`, dados,
/// folha (os nomes vêm do `skin.json` e não podem sair da pasta).
pub fn arquivos_da_skin(pasta: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let ler = |nome: &str| {
        std::fs::read(pasta.join(nome)).map_err(|e| format!("{}: {e}", pasta.join(nome).display()))
    };
    let skin_json = ler("skin.json")?;
    let v: serde_json::Value =
        serde_json::from_slice(&skin_json).map_err(|e| format!("skin.json: {e}"))?;
    let nome = |campo: &str| -> Result<String, String> {
        let n = v[campo]
            .as_str()
            .ok_or_else(|| format!("skin.json sem «{campo}»"))?;
        let simples = !n.is_empty() && !n.contains(['/', '\\']) && n != "." && n != "..";
        simples
            .then(|| n.to_owned())
            .ok_or_else(|| format!("«{campo}» precisa ser um arquivo da própria pasta"))
    };
    let (dados, folha) = (nome("dados")?, nome("folha")?);
    let conteudo_dados = ler(&dados)?;
    let conteudo_folha = ler(&folha)?;
    Ok(vec![
        ("skin.json".to_owned(), skin_json),
        (dados, conteudo_dados),
        (folha, conteudo_folha),
    ])
}

pub fn impressao_dos_arquivos(arquivos: &[(String, Vec<u8>)]) -> String {
    let refs: Vec<(&str, &[u8])> = arquivos
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    impressao(&refs)
}

/// Impressão da skin de uma pasta.
pub fn impressao_da_pasta(pasta: &Path) -> Result<String, String> {
    Ok(impressao_dos_arquivos(&arquivos_da_skin(pasta)?))
}

/// 64 dígitos hexadecimais minúsculos.
pub fn sha_valido(sha: &str) -> bool {
    sha.len() == 64
        && sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `aprovacao.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registro {
    pub id: String,
    pub sha256: String,
    /// Milissegundos desde 1970 no relógio do host.
    pub aprovada_em_ms: u64,
}

/// Uma skin carregada e a impressão dela.
#[derive(Debug)]
pub struct Candidata {
    pub skin: Skin,
    pub sha256: String,
}

/// De onde vem a skin que aparece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origem {
    /// A skin que veio na imagem, aprovada.
    Imagem,
    /// A cópia aprovada guardada em `/state`.
    Snapshot,
}

#[derive(Debug)]
pub enum Decisao {
    Mostrar {
        candidata: Box<Candidata>,
        origem: Origem,
        avisos: Vec<String>,
    },
    Esconder {
        avisos: Vec<String>,
    },
}

fn curto(sha: &str) -> &str {
    &sha[..sha.len().min(12)]
}

/// Qual skin aparece. `imagem`: a skin configurada nas pastas da imagem
/// (`None` se não existe; `Some(Err)` se não carrega). `snapshot`: a cópia
/// aprovada em `/state` (idem).
pub fn decidir(
    id: &str,
    imagem: Option<Result<Candidata, String>>,
    registro: Option<&Registro>,
    snapshot: Option<Result<Candidata, String>>,
) -> Decisao {
    let Some(reg) = registro else {
        let mut avisos = vec![match &imagem {
            None => format!(
                "skin «{id}» não instalada: rode bin/pet skin-instalar <zip> e depois bin/pet subir"
            ),
            Some(Err(e)) => format!("skin «{id}» da imagem não carrega: {e}"),
            Some(Ok(_)) => format!(
                "skin «{id}» sem aprovação: veja a folha de contato e rode bin/pet skin-aprovar {id}"
            ),
        }];
        if imagem.is_some() {
            avisos.push("sem personagem aprovado o pet fica escondido (decisão 0026)".into());
        }
        return Decisao::Esconder { avisos };
    };
    let mut avisos = Vec::new();
    let motivo_imagem = match imagem {
        Some(Ok(c)) if c.sha256 == reg.sha256 => {
            return Decisao::Mostrar {
                candidata: Box::new(c),
                origem: Origem::Imagem,
                avisos,
            };
        }
        Some(Ok(c)) => format!(
            "a skin «{id}» da imagem mudou depois da aprovação (sha {} na imagem, {} aprovado); \
             para aprovar a nova, veja a folha de contato e rode bin/pet skin-aprovar {id}",
            curto(&c.sha256),
            curto(&reg.sha256)
        ),
        Some(Err(e)) => format!("a skin «{id}» da imagem não carrega: {e}"),
        None => format!("a imagem não tem a skin «{id}»"),
    };
    match snapshot {
        Some(Ok(s)) if s.sha256 == reg.sha256 => {
            avisos.push(format!(
                "{motivo_imagem}; mostrando a cópia aprovada de /state"
            ));
            Decisao::Mostrar {
                candidata: Box::new(s),
                origem: Origem::Snapshot,
                avisos,
            }
        }
        outra => {
            avisos.push(motivo_imagem);
            avisos.push(match outra {
                Some(Ok(_)) => "a cópia de /state não bate com a aprovação".into(),
                Some(Err(e)) => format!("a cópia aprovada de /state não carrega: {e}"),
                None => "não há cópia aprovada em /state".into(),
            });
            avisos.push("o pet fica escondido até uma skin aprovada voltar".into());
            Decisao::Esconder { avisos }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::skin::testes::skin_minima;

    #[test]
    fn impressao_e_a_do_sha256sum() {
        // sha256sum de "a" e de "" (valores conhecidos) e o sha256 da lista.
        let lista = "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb  skin.json\n\
                     e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  sheet.json\n";
        let esperado = hex(&Sha256::digest(lista.as_bytes()));
        assert_eq!(
            impressao(&[("skin.json", b"a"), ("sheet.json", b"")]),
            esperado
        );
        assert!(sha_valido(&esperado));
        assert!(!sha_valido("ABC"));
        assert!(!sha_valido(&esperado.to_uppercase()));
    }

    fn candidata(sha: &str) -> Candidata {
        Candidata {
            skin: skin_minima(),
            sha256: sha.into(),
        }
    }

    fn reg(sha: &str) -> Registro {
        Registro {
            id: "mini".into(),
            sha256: sha.into(),
            aprovada_em_ms: 1,
        }
    }

    #[test]
    fn sem_aprovacao_fica_escondido() {
        let d = decidir("mini", Some(Ok(candidata("aa"))), None, None);
        let Decisao::Esconder { avisos } = d else {
            panic!("sem aprovação apareceu");
        };
        assert!(
            avisos[0].contains("bin/pet skin-aprovar mini"),
            "{avisos:?}"
        );
        let Decisao::Esconder { avisos } = decidir("mini", None, None, None) else {
            panic!();
        };
        assert!(avisos[0].contains("skin-instalar"), "{avisos:?}");
    }

    #[test]
    fn aprovada_mostra_a_da_imagem() {
        let d = decidir(
            "mini",
            Some(Ok(candidata("aa"))),
            Some(&reg("aa")),
            Some(Ok(candidata("aa"))),
        );
        assert!(
            matches!(d, Decisao::Mostrar { origem: Origem::Imagem, ref avisos, .. } if avisos.is_empty())
        );
    }

    #[test]
    fn imagem_mudada_ou_quebrada_usa_o_snapshot() {
        let d = decidir(
            "mini",
            Some(Ok(candidata("bb"))),
            Some(&reg("aa")),
            Some(Ok(candidata("aa"))),
        );
        let Decisao::Mostrar { origem, avisos, .. } = d else {
            panic!("devia mostrar o snapshot");
        };
        assert_eq!(origem, Origem::Snapshot);
        assert!(
            avisos[0].contains("mudou depois da aprovação"),
            "{avisos:?}"
        );
        let d = decidir(
            "mini",
            Some(Err("png quebrado".into())),
            Some(&reg("aa")),
            Some(Ok(candidata("aa"))),
        );
        assert!(matches!(
            d,
            Decisao::Mostrar {
                origem: Origem::Snapshot,
                ..
            }
        ));
        let d = decidir("mini", None, Some(&reg("aa")), Some(Ok(candidata("aa"))));
        assert!(matches!(
            d,
            Decisao::Mostrar {
                origem: Origem::Snapshot,
                ..
            }
        ));
    }

    #[test]
    fn nada_bate_fica_escondido() {
        let d = decidir(
            "mini",
            Some(Ok(candidata("bb"))),
            Some(&reg("aa")),
            Some(Ok(candidata("cc"))),
        );
        assert!(matches!(d, Decisao::Esconder { .. }));
        let d = decidir("mini", Some(Ok(candidata("bb"))), Some(&reg("aa")), None);
        let Decisao::Esconder { avisos } = d else {
            panic!();
        };
        assert!(
            avisos.iter().any(|a| a.contains("não há cópia")),
            "{avisos:?}"
        );
    }

    #[test]
    fn arquivos_da_pasta_na_ordem_da_impressao() {
        let pasta = std::env::temp_dir().join(format!("bichinho-aprovacao-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        std::fs::write(
            pasta.join("skin.json"),
            r#"{"folha":"f.png","dados":"d.json"}"#,
        )
        .unwrap();
        std::fs::write(pasta.join("d.json"), "{}").unwrap();
        std::fs::write(pasta.join("f.png"), [1u8, 2, 3]).unwrap();
        let a = arquivos_da_skin(&pasta).unwrap();
        let nomes: Vec<&str> = a.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(nomes, ["skin.json", "d.json", "f.png"]);
        assert_eq!(
            impressao_da_pasta(&pasta).unwrap(),
            impressao_dos_arquivos(&a)
        );
        std::fs::write(
            pasta.join("skin.json"),
            r#"{"folha":"../f.png","dados":"d.json"}"#,
        )
        .unwrap();
        assert!(arquivos_da_skin(&pasta).is_err(), "nome que sai da pasta");
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
