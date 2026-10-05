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

// --- o que importa em cada linha da tabela do PLANO (decisão 0077) ----------

use crate::motor::intencoes::Tipo;

/// As linhas do tipo `i` (o nome da intenção na linha do tempo).
fn so<'a>(linha: &'a [Intencao], i: &str) -> Vec<&'a Intencao> {
    linha
        .iter()
        .filter(|x| serde_json::to_value(x).unwrap()["i"] == i)
        .collect()
}

/// Os níveis da escalada, em ordem, com o instante.
fn niveis(linha: &[Intencao]) -> Vec<(u64, u8)> {
    linha
        .iter()
        .filter_map(|x| match &x.tipo {
            Tipo::Escalada { nivel, .. } => Some((x.t_ms, *nivel)),
            _ => None,
        })
        .collect()
}

fn chamadas(linha: &[Intencao]) -> usize {
    linha
        .iter()
        .filter(
            |x| matches!(&x.tipo, Tipo::Reacao { nome, motivo: "aviso", .. } if nome == "alert"),
        )
        .count()
}

#[test]
fn pergunta_e_pergunta_dupla_sao_um_aviso_so() {
    for nome in ["pergunta", "pergunta-dupla"] {
        let linha = linha_do_tempo(nome);
        assert_eq!(chamadas(&linha), 1, "{nome}: uma chamada");
        let baloes: Vec<_> = so(&linha, "balao");
        assert_eq!(baloes.len(), 1, "{nome}: um balão");
        assert!(
            matches!(&baloes[0].tipo, Tipo::Balao { linhas, .. } if linhas[0].ends_with("pergunta pra você")),
            "{nome}: o balão da pergunta"
        );
        let n = niveis(&linha);
        assert_eq!(
            n.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
            vec![1, 0],
            "{nome}: L1 até a resposta"
        );
        assert!(so(&linha, "rajada").is_empty() && so(&linha, "voo").is_empty());
    }
}

#[test]
fn plano_lido_no_terminal_fica_em_l1() {
    let linha = linha_do_tempo("plano-lido-no-terminal");
    assert_eq!(chamadas(&linha), 1);
    assert_eq!(
        niveis(&linha).iter().map(|(_, n)| *n).max(),
        Some(1),
        "lendo no terminal do Claude, com pausas de menos de 60 s"
    );
    assert!(so(&linha, "rajada").is_empty() && so(&linha, "voo").is_empty());
    assert!(matches!(
        &so(&linha, "balao")[0].tipo,
        Tipo::Balao { linhas, .. } if linhas[0].starts_with("Plano pra aprovar!")
    ));
}

#[test]
fn pergunta_ausente_escala_ate_o_teto_e_para() {
    let linha = linha_do_tempo("pergunta-ausente");
    // O aviso abre em 60 s.
    assert_eq!(
        niveis(&linha),
        vec![
            (60_000, 1),
            (90_000, 2),
            (150_000, 3),
            (360_000, 4),
            (3_020_000, 0)
        ]
    );
    let rajadas = |nivel: u8| {
        so(&linha, "rajada")
            .iter()
            .filter(|x| matches!(x.tipo, Tipo::Rajada { nivel: n, .. } if n == nivel))
            .map(|x| x.t_ms)
            .collect::<Vec<_>>()
    };
    assert_eq!(rajadas(2), vec![90_000, 96_000, 102_000, 108_000, 114_000]);
    assert_eq!(rajadas(4).len(), 30, "uma por minuto, por 30 min");
    assert_eq!(rajadas(4).last(), Some(&2_100_000));
    let voos: Vec<u64> = so(&linha, "voo").iter().map(|x| x.t_ms).collect();
    assert_eq!(
        voos,
        vec![150_000, 210_000, 270_000],
        "três voos, nenhum na volta"
    );
    let pulsos: Vec<(u64, bool)> = so(&linha, "pulso")
        .iter()
        .map(|x| match x.tipo {
            Tipo::Pulso { ligado, .. } => (x.t_ms, ligado),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(pulsos, vec![(360_000, true), (2_160_000, false)]);
}

#[test]
fn pergunta_com_volta_voa_na_hora_em_que_o_renan_volta() {
    let linha = linha_do_tempo("pergunta-com-volta");
    let voos: Vec<(u64, &str)> = so(&linha, "voo")
        .iter()
        .map(|x| match x.tipo {
            Tipo::Voo { motivo, .. } => (x.t_ms, motivo),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        voos,
        vec![
            (110_000, "escalada"),
            (130_000, "voltou"),
            (190_000, "escalada")
        ]
    );
    // Olhou o terminal do Claude: o pulso para na hora.
    assert!(
        so(&linha, "pulso")
            .iter()
            .any(|x| x.t_ms == 400_000 && matches!(x.tipo, Tipo::Pulso { ligado: false, .. }))
    );
}

#[test]
fn com_o_nao_perturbe_nunca_passa_de_l1() {
    let linha = linha_do_tempo("pergunta-nao-perturbe");
    assert_eq!(chamadas(&linha), 1, "a L1 toca");
    assert_eq!(
        niveis(&linha).iter().map(|(_, n)| *n).collect::<Vec<_>>(),
        vec![1, 0]
    );
    assert!(so(&linha, "rajada").is_empty() && so(&linha, "voo").is_empty());
}
