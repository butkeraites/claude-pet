//! `bichinho simular` e `bichinho cenario` (decisão 0078): o cérebro e o
//! Motor num relógio falso, offline, e o cenário gravado de um pet de debug.
//!
//! Nenhum dos dois fala com o pet nem com o desktop: o `simular` roda o
//! executor dos testes (`pet_core::cenario`, com a janela de mentira) e o
//! `cenario` só transforma a entrada padrão na saída.

use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use pet_core::cenario;

/// O `/v1/debug/eventos` guarda 200 eventos de metadados: bem menos que
/// isto. Uma entrada maior não é o que o `bin/pet eventos` manda.
const MAX_ENTRADA: u64 = 16 * 1024 * 1024;

/// `bichinho simular <cenário.jsonl>`: a linha do tempo das intenções, uma
/// por linha (o formato do `.esperado.jsonl`), sem personagem: as intenções
/// não dependem da skin.
pub fn simular(mut argumentos: impl Iterator<Item = String>) -> ExitCode {
    let (Some(caminho), None) = (argumentos.next(), argumentos.next()) else {
        eprintln!("uso: bichinho simular <cenário.jsonl>");
        return ExitCode::from(2);
    };
    let texto = match std::fs::read_to_string(&caminho) {
        Ok(texto) => texto,
        Err(e) => {
            eprintln!("não consegui ler {caminho}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let nome = nome_do_cenario(&caminho);
    match cenario::ler(&nome, &texto).and_then(|c| cenario::rodar(&c, None)) {
        Ok(linha) => {
            print!("{}", cenario::linhas(&linha));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// `bichinho cenario [nome]`: o JSON do `/v1/debug/eventos` na entrada
/// padrão vira um cenário com pseudônimos na saída (só metadados; decisão
/// 0078).
pub fn cenario(mut argumentos: impl Iterator<Item = String>) -> ExitCode {
    let nome = argumentos.next().unwrap_or_else(|| "gravado".to_owned());
    if argumentos.next().is_some() {
        eprintln!("uso: bichinho cenario [nome] < eventos.json");
        return ExitCode::from(2);
    }
    let mut json = String::new();
    if let Err(e) = std::io::stdin().take(MAX_ENTRADA).read_to_string(&mut json) {
        eprintln!("não consegui ler a entrada: {e}");
        return ExitCode::FAILURE;
    }
    match cenario::de_eventos(&nome, &json) {
        Ok(texto) => {
            print!("{texto}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// O nome do cenário pelo arquivo (`cenarios/pergunta.jsonl` → `pergunta`):
/// o do cabeçalho, se houver, tem de ser o mesmo.
fn nome_do_cenario(caminho: &str) -> String {
    let arquivo = Path::new(caminho)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(caminho);
    arquivo.strip_suffix(".jsonl").unwrap_or(arquivo).to_owned()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_nome_vem_do_arquivo() {
        assert_eq!(nome_do_cenario("cenarios/pergunta.jsonl"), "pergunta");
        assert_eq!(nome_do_cenario("/tmp/x/real-laco.jsonl"), "real-laco");
        assert_eq!(nome_do_cenario("solto"), "solto");
    }
}
