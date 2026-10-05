//! Os cenários dourados (decisão 0077): cada `cenarios/<nome>.jsonl` roda no
//! executor com a skin `_teste`, e a linha do tempo tem de ser a do
//! `cenarios/<nome>.esperado.jsonl`. Mudou de propósito? Regere com
//! `PET_ATUALIZAR_OURO=1 cargo test -p pet-core cenario` e leia o diff.

use std::path::{Path, PathBuf};

use super::*;

fn pasta() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cenarios")
}

fn skin_teste() -> Rc<Skin> {
    let pasta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skins/_teste");
    Rc::new(Skin::carregar(&pasta).expect("skins/_teste"))
}

/// Os nomes dos cenários da pasta (sem `.jsonl`), em ordem.
fn nomes() -> Vec<String> {
    let mut nomes: Vec<String> = std::fs::read_dir(pasta())
        .expect("a pasta cenarios/")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(".jsonl") && !n.ends_with(".esperado.jsonl"))
        .map(|n| n.trim_end_matches(".jsonl").to_owned())
        .collect();
    nomes.sort();
    nomes
}

/// Roda o cenário `nome` da pasta e devolve a linha do tempo.
pub(crate) fn linha_do_tempo(nome: &str) -> Vec<Intencao> {
    let texto = std::fs::read_to_string(pasta().join(format!("{nome}.jsonl")))
        .unwrap_or_else(|e| panic!("{nome}: {e}"));
    let cenario = ler(nome, &texto).unwrap_or_else(|e| panic!("{nome}: {e}"));
    rodar(&cenario, Some(skin_teste())).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn cenarios_dourados() {
    let atualizar = std::env::var("PET_ATUALIZAR_OURO").as_deref() == Ok("1");
    let nomes = nomes();
    assert!(!nomes.is_empty(), "nenhum cenário em cenarios/");
    let mut falhas = Vec::new();
    for nome in &nomes {
        let obtido = linhas(&linha_do_tempo(nome));
        let arquivo = pasta().join(format!("{nome}.esperado.jsonl"));
        if atualizar {
            std::fs::write(&arquivo, &obtido).unwrap();
            continue;
        }
        let esperado = std::fs::read_to_string(&arquivo).unwrap_or_else(|_| {
            panic!(
                "falta {} (PET_ATUALIZAR_OURO=1 para gerar)",
                arquivo.display()
            )
        });
        if obtido != esperado {
            let primeira = obtido
                .lines()
                .zip(esperado.lines())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| obtido.lines().count().min(esperado.lines().count()));
            falhas.push(format!(
                "«{nome}» mudou na linha {}:\n  esperado: {}\n  obtido:   {}",
                primeira + 1,
                esperado.lines().nth(primeira).unwrap_or("(nada)"),
                obtido.lines().nth(primeira).unwrap_or("(nada)")
            ));
        }
    }
    // Nenhum esperado sem o cenário dele.
    for entrada in std::fs::read_dir(pasta()).unwrap().filter_map(|e| e.ok()) {
        let nome = entrada.file_name().into_string().unwrap_or_default();
        if let Some(base) = nome.strip_suffix(".esperado.jsonl") {
            assert!(
                nomes.iter().any(|n| n == base),
                "{nome} sem o cenário {base}.jsonl"
            );
        }
    }
    assert!(
        falhas.is_empty(),
        "{}\n(se foi de propósito: PET_ATUALIZAR_OURO=1 cargo test -p pet-core cenario)",
        falhas.join("\n")
    );
}

#[test]
fn le_o_formato_e_recusa_o_que_nao_serve() {
    let bom = r#"
# um comentário
{"cenario": "x", "descricao": "d", "padrao": {"sid": "s1", "ent": "cli", "proj": "api"}}
{"t": 0, "evento": {"e": "UserPromptSubmit", "turno": "p1"}}
{"t": 10, "evento": {"e": "Stop", "turno": "p1", "ts": 9}}
{"t": 20, "desktop": {"ocioso": true}}
{"t": 20, "desktop": {"janela_ativa": null}}
{"t": 30, "clique": "direito"}
{"t": 40, "fim": true}
"#;
    let c = ler("x", bom).unwrap();
    assert_eq!(c.descricao.as_deref(), Some("d"));
    assert_eq!(c.passos.len(), 6);
    let Passo::Evento { evento, .. } = &c.passos[1] else {
        panic!("evento");
    };
    assert_eq!(
        (
            evento.sid.as_deref(),
            evento.proj.as_deref(),
            evento.ts,
            evento.e.as_str()
        ),
        (Some("s1"), Some("api"), Some(BASE_PAREDE + 9), "Stop")
    );
    assert_eq!(
        c.passos[3],
        Passo::Desktop {
            t: 20,
            evento: Desktop::JanelaAtiva(None)
        }
    );
    for (ruim, erro) in [
        (
            r#"{"t": 10, "fim": true}
{"t": 5, "fim": true}"#,
            "volta no tempo",
        ),
        (r#"{"cenario": "outro"}"#, "o cabeçalho diz"),
        (
            r#"{"t": 1, "evento": {"e": "Stop", "sid": "tem espaço"}}"#,
            "descartaria",
        ),
        (
            r#"{"t": 1, "evento": {"e": "Stop"}, "clique": "esquerdo"}"#,
            "só um de",
        ),
        (r#"{"t": 1, "desktop": {"nuvem": true}}"#, "não conheço"),
        (r#"{"t": 1, "clique": "meio"}"#, "esquerdo ou direito"),
        (r#"{"evento": {"e": "Stop"}}"#, "falta o t"),
        (
            r#"{"cenario": "x", "config": {"celebracao.modo": "exagerada"}}"#,
            "config",
        ),
        (r#"{"cenario": "x", "config": {"nada": 1}}"#, "secao.chave"),
        ("não é json", "não é JSON"),
    ] {
        let e = ler("x", ruim).unwrap_err();
        assert!(e.contains(erro), "{ruim}: {e}");
    }
}

#[test]
fn o_executor_vence_os_prazos_como_o_laco() {
    // Um prompt e um Stop: o aceno sai na acomodação, em 30,8 s, sem passo
    // nenhum no meio (o executor vence o prazo do cérebro sozinho).
    let c = ler(
        "x",
        r#"{"cenario": "x", "padrao": {"sid": "s1", "ent": "cli"}}
{"t": 0, "evento": {"e": "UserPromptSubmit", "turno": "p1"}}
{"t": 30000, "evento": {"e": "Stop", "turno": "p1"}}
{"t": 40000, "fim": true}"#,
    )
    .unwrap();
    let linha = rodar(&c, Some(skin_teste())).unwrap();
    let reacoes: Vec<(u64, String)> = linha
        .iter()
        .filter_map(|i| match &i.tipo {
            crate::motor::intencoes::Tipo::Reacao { nome, .. } => Some((i.t_ms, nome.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(reacoes, vec![(30_800, "nod".to_owned())]);
    // Sem personagem, o cérebro decide igual.
    assert_eq!(
        linhas(&rodar(&c, None).unwrap()),
        linhas(&linha),
        "as intenções não dependem da skin"
    );
}
