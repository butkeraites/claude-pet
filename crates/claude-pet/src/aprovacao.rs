//! Aprovações de personagem no volume `/state` (decisão 0026).
//!
//! `<estado>/skins/<id>/` guarda a cópia dos três arquivos aprovados (o
//! snapshot) e o `aprovacao.json` com a impressão digital. Aprovar troca a
//! pasta inteira de uma vez (escreve ao lado e renomeia); revogar apaga a
//! pasta. Quem decide o que aparece é `pet_core::aprovacao::decidir`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pet_core::aprovacao::{self, Candidata, Registro};
use pet_core::skin::Skin;

use crate::personagem::{SKIN_DE_TESTE, procurar};

/// Arquivo do registro dentro da pasta da skin aprovada.
pub const REGISTRO: &str = "aprovacao.json";

pub fn pasta_aprovadas(estado: &Path) -> PathBuf {
    estado.join("skins")
}

pub fn pasta_da_skin(estado: &Path, id: &str) -> PathBuf {
    pasta_aprovadas(estado).join(id)
}

/// `[a-z0-9_-]{1,40}`: nunca sai da pasta de aprovações.
pub fn id_valido(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// O registro de aprovação, se houver um que se leia.
pub fn ler_registro(estado: &Path, id: &str) -> Result<Option<Registro>, String> {
    let caminho = pasta_da_skin(estado, id).join(REGISTRO);
    match fs::read_to_string(&caminho) {
        Ok(texto) => serde_json::from_str::<Registro>(&texto)
            .map(Some)
            .map_err(|e| format!("{}: {e}", caminho.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", caminho.display())),
    }
}

/// Carrega uma skin e calcula a impressão dela.
pub fn candidata(pasta: &Path) -> Result<Candidata, String> {
    let sha256 = aprovacao::impressao_da_pasta(pasta)?;
    let skin = Skin::carregar(pasta).map_err(|e| e.to_string())?;
    Ok(Candidata { skin, sha256 })
}

/// A cópia aprovada, se existir.
pub fn snapshot(estado: &Path, id: &str) -> Option<Result<Candidata, String>> {
    let pasta = pasta_da_skin(estado, id);
    pasta.join("skin.json").is_file().then(|| candidata(&pasta))
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErroAprovacao {
    /// Pedido mal formado (400).
    Pedido(String),
    /// A skin de teste nunca é personagem (403).
    Proibida(String),
    /// Não há skin com esse id nas pastas da imagem (404).
    NaoAchou(String),
    /// A skin da imagem não é a que o Renan viu (409).
    Diferente { imagem: String, informado: String },
    /// A skin não carrega (422).
    Invalida(String),
    /// Falha ao gravar em `/state` (500).
    Disco(String),
}

impl ErroAprovacao {
    pub fn status(&self) -> u16 {
        match self {
            ErroAprovacao::Pedido(_) => 400,
            ErroAprovacao::Proibida(_) => 403,
            ErroAprovacao::NaoAchou(_) => 404,
            ErroAprovacao::Diferente { .. } => 409,
            ErroAprovacao::Invalida(_) => 422,
            ErroAprovacao::Disco(_) => 500,
        }
    }

    pub fn mensagem(&self) -> String {
        match self {
            ErroAprovacao::Pedido(m)
            | ErroAprovacao::Proibida(m)
            | ErroAprovacao::NaoAchou(m)
            | ErroAprovacao::Invalida(m)
            | ErroAprovacao::Disco(m) => m.clone(),
            ErroAprovacao::Diferente { imagem, informado } => format!(
                "a skin da imagem (sha {imagem}) não é a que você viu (sha {informado}): \
                 rode bin/pet subir para a imagem pegar a skin nova e aprove de novo"
            ),
        }
    }
}

fn agora_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Aprova a skin `id` das pastas da imagem, se a impressão dela for
/// `sha256` (a que o Renan viu): grava o snapshot e o registro.
pub fn aprovar(
    id: &str,
    sha256: &str,
    busca: &[PathBuf],
    estado: &Path,
) -> Result<Registro, ErroAprovacao> {
    if !id_valido(id) {
        return Err(ErroAprovacao::Pedido(format!(
            "id «{id}» fora de [a-z0-9_-]{{1,40}}"
        )));
    }
    if id == SKIN_DE_TESTE {
        return Err(ErroAprovacao::Proibida(
            "a skin de teste nunca vira personagem (decisão 0011)".into(),
        ));
    }
    if !aprovacao::sha_valido(sha256) {
        return Err(ErroAprovacao::Pedido(
            "sha256 precisa ter 64 dígitos hexadecimais minúsculos".into(),
        ));
    }
    let pasta = procurar(id, busca).ok_or_else(|| {
        ErroAprovacao::NaoAchou(format!(
            "a imagem não tem a skin «{id}»: rode bin/pet skin-instalar <zip> e bin/pet subir"
        ))
    })?;
    let arquivos = aprovacao::arquivos_da_skin(&pasta).map_err(ErroAprovacao::Invalida)?;
    let imagem = aprovacao::impressao_dos_arquivos(&arquivos);
    if imagem != sha256 {
        return Err(ErroAprovacao::Diferente {
            imagem,
            informado: sha256.to_owned(),
        });
    }
    let skin = Skin::carregar(&pasta).map_err(|e| ErroAprovacao::Invalida(e.to_string()))?;
    if skin.id != id {
        return Err(ErroAprovacao::Invalida(format!(
            "a pasta «{id}» tem a skin «{}»",
            skin.id
        )));
    }
    let registro = Registro {
        id: id.to_owned(),
        sha256: imagem,
        aprovada_em_ms: agora_ms(),
    };
    gravar(estado, &registro, &arquivos).map_err(ErroAprovacao::Disco)?;
    Ok(registro)
}

/// Escreve a pasta nova ao lado e troca de uma vez.
fn gravar(
    estado: &Path,
    registro: &Registro,
    arquivos: &[(String, Vec<u8>)],
) -> Result<(), String> {
    let base = pasta_aprovadas(estado);
    fs::create_dir_all(&base).map_err(|e| format!("{}: {e}", base.display()))?;
    let sufixo = format!("{}-{}", std::process::id(), agora_ms());
    let nova = base.join(format!(".nova-{}-{sufixo}", registro.id));
    let velha = base.join(format!(".velha-{}-{sufixo}", registro.id));
    let destino = pasta_da_skin(estado, &registro.id);
    let resultado = (|| {
        fs::create_dir(&nova).map_err(|e| format!("{}: {e}", nova.display()))?;
        for (nome, dados) in arquivos {
            fs::write(nova.join(nome), dados).map_err(|e| format!("{nome}: {e}"))?;
        }
        let texto = serde_json::to_string_pretty(registro).map_err(|e| e.to_string())? + "\n";
        fs::write(nova.join(REGISTRO), texto).map_err(|e| format!("{REGISTRO}: {e}"))?;
        // A cópia tem de ser exatamente o que foi aprovado.
        let conferida = aprovacao::impressao_da_pasta(&nova)?;
        if conferida != registro.sha256 {
            return Err("a cópia gravada não bate com a aprovação".into());
        }
        if destino.exists() {
            fs::rename(&destino, &velha).map_err(|e| format!("{}: {e}", destino.display()))?;
        }
        fs::rename(&nova, &destino).map_err(|e| format!("{}: {e}", destino.display()))?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&velha);
    if resultado.is_err() {
        let _ = fs::remove_dir_all(&nova);
    }
    resultado
}

/// Revoga a aprovação de `id`: apaga o snapshot e o registro. `Ok(false)`
/// se não havia aprovação.
pub fn revogar(id: &str, estado: &Path) -> Result<bool, ErroAprovacao> {
    if !id_valido(id) {
        return Err(ErroAprovacao::Pedido(format!(
            "id «{id}» fora de [a-z0-9_-]{{1,40}}"
        )));
    }
    let pasta = pasta_da_skin(estado, id);
    if !pasta.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&pasta)
        .map(|()| true)
        .map_err(|e| ErroAprovacao::Disco(format!("{}: {e}", pasta.display())))
}

#[cfg(test)]
pub mod testes {
    use super::*;

    /// Pastas temporárias com uma skin «zeca» (cópia da xadrez com outro id)
    /// e um `/state` vazio.
    pub struct Ambiente {
        pub raiz: PathBuf,
        pub skins: PathBuf,
        pub estado: PathBuf,
    }

    impl Ambiente {
        pub fn novo(nome: &str) -> Ambiente {
            let raiz = std::env::temp_dir().join(format!(
                "claude-pet-aprovacao-{}-{nome}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&raiz);
            let skins = raiz.join("skins");
            let estado = raiz.join("estado");
            let zeca = skins.join("zeca");
            fs::create_dir_all(&zeca).unwrap();
            fs::create_dir_all(&estado).unwrap();
            let teste = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skins/_teste");
            for nome in ["sheet.json", "sheet.png"] {
                fs::copy(teste.join(nome), zeca.join(nome)).unwrap();
            }
            let skin = fs::read_to_string(teste.join("skin.json"))
                .unwrap()
                .replace("\"_teste\"", "\"zeca\"");
            fs::write(zeca.join("skin.json"), skin).unwrap();
            fs::create_dir_all(skins.join("_teste")).unwrap();
            for nome in ["skin.json", "sheet.json", "sheet.png"] {
                fs::copy(teste.join(nome), skins.join("_teste").join(nome)).unwrap();
            }
            Ambiente {
                raiz,
                skins,
                estado,
            }
        }

        pub fn sha(&self) -> String {
            aprovacao::impressao_da_pasta(&self.skins.join("zeca")).unwrap()
        }

        pub fn busca(&self) -> Vec<PathBuf> {
            vec![self.skins.clone()]
        }
    }

    impl Drop for Ambiente {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.raiz);
        }
    }

    #[test]
    fn aprova_grava_snapshot_e_revoga() {
        let a = Ambiente::novo("aprova");
        let sha = a.sha();
        let r = aprovar("zeca", &sha, &a.busca(), &a.estado).unwrap();
        assert_eq!(r.sha256, sha);
        assert_eq!(ler_registro(&a.estado, "zeca").unwrap(), Some(r));
        let copia = snapshot(&a.estado, "zeca").unwrap().unwrap();
        assert_eq!(copia.sha256, sha);
        assert_eq!(copia.skin.id, "zeca");
        // Aprovar de novo troca a pasta inteira, sem lixo ao lado.
        aprovar("zeca", &sha, &a.busca(), &a.estado).unwrap();
        let entradas: Vec<_> = fs::read_dir(pasta_aprovadas(&a.estado)).unwrap().collect();
        assert_eq!(entradas.len(), 1, "sem .nova/.velha sobrando");
        assert_eq!(revogar("zeca", &a.estado), Ok(true));
        assert_eq!(ler_registro(&a.estado, "zeca").unwrap(), None);
        assert!(snapshot(&a.estado, "zeca").is_none());
        assert_eq!(revogar("zeca", &a.estado), Ok(false));
    }

    #[test]
    fn so_aprova_o_que_o_renan_viu() {
        let a = Ambiente::novo("diferente");
        let outro = "0".repeat(64);
        let erro = aprovar("zeca", &outro, &a.busca(), &a.estado).unwrap_err();
        assert_eq!(erro.status(), 409);
        assert!(erro.mensagem().contains("bin/pet subir"));
        assert!(ler_registro(&a.estado, "zeca").unwrap().is_none());
    }

    #[test]
    fn pedidos_ruins_e_skin_de_teste() {
        let a = Ambiente::novo("ruins");
        let sha = a.sha();
        assert_eq!(
            aprovar("../x", &sha, &a.busca(), &a.estado)
                .unwrap_err()
                .status(),
            400
        );
        assert_eq!(
            aprovar("zeca", "abc", &a.busca(), &a.estado)
                .unwrap_err()
                .status(),
            400
        );
        assert_eq!(
            aprovar("_teste", &sha, &a.busca(), &a.estado)
                .unwrap_err()
                .status(),
            403
        );
        assert_eq!(
            aprovar("outra", &sha, &a.busca(), &a.estado)
                .unwrap_err()
                .status(),
            404
        );
        assert_eq!(revogar("../../etc", &a.estado).unwrap_err().status(), 400);
    }
}
