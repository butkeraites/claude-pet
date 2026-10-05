//! A memória das sessões em disco (decisão 0093): `<estado>/sessoes.json`
//! (no container, `/state/sessoes.json`). O formato, a conferência campo a
//! campo e o que volta são do núcleo puro (`pet_core::memoria`); aqui ficam o
//! arquivo e o boot id da máquina, que é do Linux.
//!
//! Quem grava é só o laço principal ([`crate::nucleo::Nucleo::guardar_memoria`]):
//! nunca a entrada HTTP, nunca o hook.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use pet_core::memoria::{self, ARQUIVO, Lida, MAX_BYTES, Memoria, Recusa};

/// O que a partida achou em disco.
#[derive(Debug)]
pub enum Leitura {
    /// Nenhum arquivo: a primeira partida, ou nada guardado.
    Nada,
    /// O arquivo, lido e conferido.
    Lida(Lida),
    /// Ignorado inteiro (corrompido, de outra versão, grande demais): o
    /// motivo nunca cita o conteúdo.
    Recusada(Recusa),
    /// Não deu para abrir ou ler (a permissão, o disco).
    Erro(String),
}

/// O arquivo da memória dentro da pasta do estado.
pub fn caminho(estado: &Path) -> PathBuf {
    estado.join(ARQUIVO)
}

/// Lê a memória das sessões, até [`MAX_BYTES`] (um arquivo maior é recusado
/// sem ler o resto).
pub fn ler(estado: &Path) -> Leitura {
    let caminho = caminho(estado);
    let arquivo = match fs::File::open(&caminho) {
        Ok(arquivo) => arquivo,
        Err(e) if e.kind() == ErrorKind::NotFound => return Leitura::Nada,
        Err(e) => return Leitura::Erro(format!("{}: {e}", caminho.display())),
    };
    let mut bytes = Vec::new();
    if let Err(e) = arquivo.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes) {
        return Leitura::Erro(format!("{}: {e}", caminho.display()));
    }
    if bytes.len() > MAX_BYTES {
        return Leitura::Recusada(Recusa::Grande);
    }
    let Ok(texto) = String::from_utf8(bytes) else {
        return Leitura::Recusada(Recusa::Json);
    };
    match memoria::ler(&texto) {
        Ok(lida) => Leitura::Lida(lida),
        Err(recusa) => Leitura::Recusada(recusa),
    }
}

/// Grava de uma vez: um arquivo temporário na mesma pasta, só do dono, e o
/// `rename` por cima do de antes (quem lê nunca vê um arquivo pela metade).
pub fn gravar(estado: &Path, memoria: &Memoria) -> std::io::Result<()> {
    fs::create_dir_all(estado)?;
    let temporario = estado.join(format!("{ARQUIVO}.tmp"));
    let mut opcoes = fs::OpenOptions::new();
    opcoes.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opcoes.mode(0o600);
    }
    let mut arquivo = opcoes.open(&temporario)?;
    arquivo.write_all(memoria.texto().as_bytes())?;
    drop(arquivo);
    fs::rename(&temporario, caminho(estado))
}

/// O boot id da máquina: no Linux, `/proc/sys/kernel/random/boot_id` (o do
/// host, também dentro do container). Muda a cada partida da máquina, e com
/// ela todo Claude de antes morreu.
#[cfg(target_os = "linux")]
pub fn boot_id() -> Option<String> {
    boot_id_de(&fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?)
}

/// Sem o boot id (Windows e macOS até o M8), nada é guardado: não daria para
/// saber se a máquina reiniciou.
#[cfg(not(target_os = "linux"))]
pub fn boot_id() -> Option<String> {
    None
}

/// O boot id lido do arquivo, se é um.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn boot_id_de(texto: &str) -> Option<String> {
    let id = texto.trim();
    memoria::eh_boot(id).then(|| id.to_owned())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::aprovacao::testes::Ambiente;

    #[test]
    fn grava_de_uma_vez_e_le_de_volta() {
        let a = Ambiente::novo("memoria-gravar");
        let estado = a.raiz.join("estado-novo");
        assert!(matches!(ler(&estado), Leitura::Nada));
        let memoria = Memoria::nova(1_790_000_000_000, Some("boot-1".into()));
        gravar(&estado, &memoria).unwrap();
        let Leitura::Lida(lida) = ler(&estado) else {
            panic!("lida");
        };
        assert_eq!(lida.memoria, memoria);
        assert!(
            !estado.join(format!("{ARQUIVO}.tmp")).exists(),
            "o rename levou o temporário"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let modo = fs::metadata(caminho(&estado)).unwrap().permissions().mode();
            assert_eq!(modo & 0o777, 0o600, "só o dono lê");
        }
    }

    #[test]
    fn o_arquivo_ruim_grande_ou_de_outra_versao_e_recusado() {
        let a = Ambiente::novo("memoria-ruim");
        let estado = a.raiz.join("estado");
        fs::create_dir_all(&estado).unwrap();
        for (texto, recusa) in [
            ("{\"versao\": 1, \"SEGREDO".to_owned(), Recusa::Json),
            (
                r#"{"versao": 9, "gravada_ms": 1, "sessoes": []}"#.to_owned(),
                Recusa::Versao(9),
            ),
            (
                format!("{{\"x\": \"{}\"}}", "a".repeat(MAX_BYTES)),
                Recusa::Grande,
            ),
        ] {
            fs::write(caminho(&estado), &texto).unwrap();
            match ler(&estado) {
                Leitura::Recusada(r) => assert_eq!(r, recusa),
                outra => panic!("{outra:?}"),
            }
        }
        fs::write(caminho(&estado), [0xff, 0xfe, b'{']).unwrap();
        assert!(matches!(ler(&estado), Leitura::Recusada(Recusa::Json)));
    }

    #[test]
    fn o_boot_id_e_um_uuid_aparado() {
        assert_eq!(
            boot_id_de("4e45d6f5-4cb9-4bb6-bc18-875b91983f1d\n").as_deref(),
            Some("4e45d6f5-4cb9-4bb6-bc18-875b91983f1d")
        );
        assert_eq!(boot_id_de(""), None);
        assert_eq!(boot_id_de("não é um id"), None);
        #[cfg(target_os = "linux")]
        assert!(boot_id().is_some(), "o /proc deste Linux tem o boot id");
    }
}
