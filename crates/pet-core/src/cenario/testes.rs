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
        (r#"{"t": 1, "reinicio": {}}"#, "falta o parado_ms"),
        (
            r#"{"t": 1, "reinicio": {"parado_ms": 5, "nuvem": true}}"#,
            "não conheço",
        ),
        (
            r#"{"t": 1, "reinicio": {"parado_ms": 5, "arquivo": "sumido"}}"#,
            "corrompido",
        ),
        (
            r#"{"t": 1, "reinicio": {"parado_ms": 5, "maquina": 1}}"#,
            "true ou false",
        ),
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
    let festas: Vec<(u64, &str)> = linha
        .iter()
        .filter_map(|i| match &i.tipo {
            crate::motor::intencoes::Tipo::Festa {
                reacao: Some(nome), ..
            } => Some((i.t_ms, *nome)),
            _ => None,
        })
        .collect();
    assert_eq!(festas, vec![(30_800, "nod")]);
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
        let baloes: Vec<_> = so(&linha, "balao")
            .into_iter()
            .filter(
                |x| matches!(&x.tipo, Tipo::Balao { motivo, .. } if motivo.starts_with("aviso")),
            )
            .collect();
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
            (150_000, "escalada"),
            (210_000, "escalada"),
            (270_000, "escalada"),
            (3_000_000, "voltou")
        ],
        "os três da L3 com o Renan longe, e o da volta, que tem a conta dele (decisão 0090)"
    );
    // O teto solta a pose de espera: o pet dorme com o selo, e a volta o
    // acorda (decisão 0090).
    let bases: Vec<(u64, &str)> = so(&linha, "base")
        .iter()
        .map(|x| match x.tipo {
            Tipo::Base { estado, .. } => (x.t_ms, estado),
            _ => unreachable!(),
        })
        .filter(|(t, _)| (2_000_000..3_010_000).contains(t))
        .collect();
    assert_eq!(
        bases,
        vec![
            (2_160_000, "idle"),
            (2_340_000, "sleep"),
            (3_000_000, "idle")
        ]
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
            (190_000, "escalada"),
            (250_000, "escalada")
        ],
        "o da volta não gasta os da L3 (decisão 0090)"
    );
    // Olhou o terminal do Claude: o pulso para na hora.
    assert!(
        so(&linha, "pulso")
            .iter()
            .any(|x| x.t_ms == 400_000 && matches!(x.tipo, Tipo::Pulso { ligado: false, .. }))
    );
}

#[test]
fn a_pergunta_noutro_terminal_escala_e_a_vista_no_terminal_dela_para() {
    // Olhar o terminal de outra sessão do Claude não é ver o diálogo desta
    // (decisão 0090): a escalada segue; 5 s no terminal dela e o diálogo
    // conta como visto, e nada mais escala nem com o Renan de volta ao
    // outro terminal; a pose de espera sai 2 min depois.
    let linha = linha_do_tempo("pergunta-noutro-terminal");
    assert_eq!(
        niveis(&linha),
        vec![
            (20_000, 1),
            (50_000, 2),
            (110_000, 3),
            (205_000, 3),
            (600_000, 0)
        ]
    );
    assert!(so(&linha, "escalada").iter().any(|x| matches!(
        x.tipo,
        Tipo::Escalada {
            motivo: "vista",
            ..
        }
    ) && x.t_ms == 205_000));
    assert!(
        so(&linha, "voo").iter().all(|x| x.t_ms < 200_000),
        "nada voa depois de visto"
    );
    assert!(
        so(&linha, "base")
            .iter()
            .any(|x| matches!(x.tipo, Tipo::Base { estado: "idle", .. }) && x.t_ms == 325_000)
    );
}

#[test]
fn a_pergunta_dispensada_no_terminal_nao_escala_e_o_pet_dorme() {
    // O Esc numa pergunta não manda evento nenhum (conferido no 2.1.288):
    // visto no terminal, o diálogo não escala com o Renan longe, a pose sai
    // e o pet dorme; o prompt seguinte tira o aviso (decisão 0090).
    let linha = linha_do_tempo("pergunta-dispensada");
    assert!(so(&linha, "rajada").is_empty() && so(&linha, "voo").is_empty());
    assert_eq!(
        niveis(&linha),
        vec![(5_000, 1), (10_000, 1), (1_260_000, 0)]
    );
    let bases: Vec<(u64, &str)> = so(&linha, "base")
        .iter()
        .map(|x| match x.tipo {
            Tipo::Base { estado, .. } => (x.t_ms, estado),
            _ => unreachable!(),
        })
        .collect();
    assert!(bases.contains(&(130_000, "idle")) && bases.contains(&(310_000, "sleep")));
}

#[test]
fn com_a_protecao_de_tela_a_volta_voa_quando_o_pet_aparece() {
    // O primeiro toque do Renan chega com a proteção de tela ainda aberta: o
    // voo da volta espera o pet poder aparecer (decisão 0090), e nada tocou
    // escondido (nem os voos da L3).
    let linha = linha_do_tempo("pergunta-com-protetor-de-tela");
    let voos: Vec<(u64, &str)> = so(&linha, "voo")
        .iter()
        .map(|x| match x.tipo {
            Tipo::Voo { motivo, .. } => (x.t_ms, motivo),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(voos, vec![(601_000, "voltou")]);
    assert!(
        so(&linha, "rajada")
            .iter()
            .all(|x| x.t_ms < 140_000 || x.t_ms >= 601_000),
        "nada toca com a proteção de tela"
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

/// As linhas dos balões, com o instante.
fn baloes(linha: &[Intencao]) -> Vec<(u64, Vec<String>)> {
    linha
        .iter()
        .filter_map(|x| match &x.tipo {
            Tipo::Balao { linhas, .. } => Some((x.t_ms, linhas.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn dois_prontos_sao_uma_festa_so() {
    let linha = linha_do_tempo("dois-prontos");
    let festas: Vec<(u64, String)> = so(&linha, "festa")
        .iter()
        .map(|x| match &x.tipo {
            Tipo::Festa { sid8, .. } => (x.t_ms, sid8.clone()),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        festas,
        vec![
            (10_800, "s1".to_owned()),
            (11_200, "t1".to_owned()),
            (14_800, "s3".to_owned())
        ],
        "a de teste à parte; a terceira real depois dos 3 s é outra"
    );
    let mescladas = so(&linha, "festa_mesclada");
    assert_eq!(mescladas.len(), 1);
    assert!(matches!(
        &mescladas[0].tipo,
        Tipo::FestaMesclada { sid8, sessoes: 2, nivel: crate::cerebro::Nivel::T2, reacao: Some("done_medium"), confete: 12, .. } if sid8 == "s2"
    ));
    assert!(baloes(&linha).contains(&(12_300, vec!["2 prontos: api, web".to_owned()])));
}

#[test]
fn com_a_protecao_de_tela_a_festa_nao_toca_e_o_pronto_fica() {
    let linha = linha_do_tempo("protetor-de-tela");
    assert!(so(&linha, "festa").iter().all(|x| matches!(
        x.tipo,
        Tipo::Festa {
            escondida: true,
            ..
        }
    )));
    assert!(so(&linha, "reacao").is_empty(), "nada toca, nem na volta");
    assert!(baloes(&linha).is_empty());
    let bases: Vec<(u64, &str)> = so(&linha, "base")
        .iter()
        .map(|x| match x.tipo {
            Tipo::Base { estado, .. } => (x.t_ms, estado),
            _ => unreachable!(),
        })
        .collect();
    assert!(bases.contains(&(20_800, "ready")), "o pronto segura a base");
    assert!(bases.contains(&(140_800, "idle")), "por 2 min");
    let ultimo_selo = so(&linha, "selos").last().map(|x| x.tipo.clone());
    assert_eq!(
        ultimo_selo,
        Some(Tipo::Selos(crate::motor::Selos {
            mais: 1,
            bandeiras: vec![crate::motor::tela::cor("api")],
            corrente: false
        })),
        "e vira a bandeirinha (o +1 é o erro de outra sessão)"
    );
    // O erro e o cansado de outras sessões, escondidos, também não tocaram
    // nem deixaram balão (decisão 0091).
    assert!(so(&linha, "base").iter().any(|x| matches!(
        x.tipo,
        Tipo::Base {
            estado: "error",
            ..
        }
    )));
}

#[test]
fn compartilhando_a_tela_os_baloes_ficam_sem_nome() {
    let linha = linha_do_tempo("compartilhando-tela");
    let discricao: Vec<(u64, bool, bool)> = so(&linha, "discricao")
        .iter()
        .map(|x| match x.tipo {
            Tipo::Discricao {
                ligada,
                tirou_balao,
            } => (x.t_ms, ligada, tirou_balao),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        discricao,
        vec![(11_600, true, true), (340_000, false, false)],
        "a captura não liga; com 2 s de sinal somados o balão com nome sai; a tela \
         parada que pisca não desliga, e os nomes voltam 5 min depois do último sinal"
    );
    for (t, linhas) in baloes(&linha) {
        let texto = linhas.join(" ");
        if (11_600..340_000).contains(&t) {
            assert!(
                !texto.contains("agenda") && !texto.contains("web"),
                "{t}: {texto}"
            );
        }
    }
    assert!(baloes(&linha).contains(&(50_800, vec!["Prontinho!".to_owned()])));
    assert!(baloes(&linha).contains(&(347_800, vec!["Prontinho! agenda-secreta".to_owned()])));
}

// --- cenários gravados (decisão 0078) ----------------------------------------

#[test]
fn gravar_troca_os_ids_e_so_leva_metadados() {
    let sid = "e5659037-01b3-473e-b963-dc15597079f6";
    let aid = "a6f1b2c3d4e5f6a7b";
    let entrada = serde_json::json!({ "eventos": [
        {"e": "SessionStart", "ent": "cli", "proj": "agenda-presidencial", "recebido_ms": 1_791_214_185_974u64,
         "sid": sid, "src": "startup", "term": {"tmux": "%0"}, "ts": 1_791_214_185_973u64},
        {"e": "UserPromptSubmit", "ent": "cli", "proj": "agenda-presidencial", "recebido_ms": 1_791_214_194_693u64,
         "sid": sid, "turno": "df3416fe-86cb", "orig": "comum", "ts": 1_791_214_194_683u64,
         "prompt": "segredo do Renan", "descartados": ["arq"]},
        {"e": "SubagentStart", "ent": "cli", "proj": "agenda-presidencial", "recebido_ms": 1_791_214_196_000u64,
         "sid": sid, "turno": "df3416fe-86cb", "agente": true, "aid": aid, "ts": 1_791_214_195_990u64},
        {"e": "PostToolUse", "ent": "cli", "proj": "agenda-presidencial", "recebido_ms": 1_791_214_197_000u64,
         "sid": sid, "turno": "df3416fe-86cb", "tool": "mcp__github-interno__criar_issue", "dur": 40,
         "ts": 1_791_214_196_990u64},
        {"e": "PostToolUse", "ent": "cli", "proj": "agenda-presidencial", "recebido_ms": 1_791_214_198_000u64,
         "sid": sid, "turno": "df3416fe-86cb", "tool": "Edit", "arq": "9f86d081884c", "dur": 20,
         "ts": 1_791_214_197_990u64},
        {"e": "Stop", "ent": "cli", "proj": "agenda-presidencial", "recebido_ms": 1_791_214_199_000u64,
         "sid": sid, "turno": "df3416fe-86cb", "bg": 1, "bgt": ["subagent"], "bgi": [aid], "crn": 0,
         "ts": 1_791_214_198_990u64}
    ]});
    let texto = de_eventos("gravado", &entrada.to_string()).unwrap();
    for proibido in [
        sid,
        aid,
        "df3416fe",
        "agenda",
        "github",
        "criar_issue",
        "9f86d081884c",
        "segredo",
        "prompt",
        "term",
        "tmux",
        "descartados",
        "recebido_ms",
        "1791214",
    ] {
        assert!(!texto.contains(proibido), "«{proibido}» vazou:\n{texto}");
    }
    let linhas_do_texto: Vec<&str> = texto.lines().collect();
    assert_eq!(
        linhas_do_texto[1],
        r#"{"t":1,"evento":{"e":"SessionStart","ent":"cli","proj":"projeto-a","sid":"s1","src":"startup","ts":0}}"#
    );
    assert!(linhas_do_texto[3].contains(r#""aid":"a1""#));
    assert!(linhas_do_texto[4].contains(r#""tool":"mcp__servidor_a__ferramenta_1""#));
    assert!(linhas_do_texto[5].contains(r#""arq":"000000000001""#));
    assert!(
        linhas_do_texto[6].contains(r#""bgi":["a1"]"#),
        "o agente e a tarefa dele com o mesmo pseudônimo"
    );
    assert_eq!(linhas_do_texto.last(), Some(&r#"{"t":23027,"fim":true}"#));
    // E roda: o agente em segundo plano abre a corrente.
    let cenario = ler("gravado", &texto).unwrap();
    let linha = rodar(&cenario, None).unwrap();
    assert!(
        linha
            .iter()
            .any(|i| matches!(&i.tipo, Tipo::Turno { corrente: Some(c), .. } if c.aberta))
    );
    // O que não é a saída do /v1/debug/eventos é recusado.
    assert!(de_eventos("x", "[]").is_err());
    assert!(de_eventos("x", r#"{"eventos": []}"#).is_err());
    assert!(de_eventos("x", r#"{"eventos": [{"e": "Stop"}]}"#).is_err());
}

/// As festas (sem as mescladas): instante, sessão, nível e reação.
fn festas(linha: &[Intencao]) -> Vec<(u64, String, crate::cerebro::Nivel, &'static str)> {
    linha
        .iter()
        .filter_map(|x| match &x.tipo {
            Tipo::Festa {
                sid8,
                nivel,
                reacao: Some(reacao),
                ..
            } => Some((x.t_ms, sid8.clone(), *nivel, *reacao)),
            _ => None,
        })
        .collect()
}

/// Os turnos fechados: o turno, o fim e a origem (só a de máquina).
fn turnos(
    linha: &[Intencao],
) -> Vec<(
    String,
    crate::cerebro::Fim,
    Option<crate::cerebro::OrigemTurno>,
)> {
    linha
        .iter()
        .filter_map(|x| match &x.tipo {
            Tipo::Turno {
                turno8,
                fim,
                origem,
                ..
            } => Some((turno8.clone().unwrap_or_default(), *fim, *origem)),
            _ => None,
        })
        .collect()
}

/// O que importa em cada linha da tabela do PLANO (M5), além da linha do
/// tempo inteira do dourado (decisão 0077).
#[test]
fn cada_linha_da_tabela_do_plano() {
    use crate::cerebro::Nivel::{T0, T1, T2, T3};
    use crate::cerebro::OrigemTurno;
    let niveis_das_festas = |nome: &str| -> Vec<(u64, crate::cerebro::Nivel, &'static str)> {
        festas(&linha_do_tempo(nome))
            .into_iter()
            .map(|(t, _, n, r)| (t, n, r))
            .collect()
    };
    // rapido e resposta-longa-sem-ferramenta: o aceno T0, sem balão.
    for nome in ["rapido", "resposta-longa-sem-ferramenta"] {
        let linha = linha_do_tempo(nome);
        assert_eq!(
            festas(&linha)
                .iter()
                .map(|f| (f.2, f.3))
                .collect::<Vec<_>>(),
            vec![(T0, "nod")],
            "{nome}"
        );
        assert!(baloes(&linha).is_empty(), "{nome}: o T0 não tem balão");
    }
    // pequeno, medio, grande: T1, T2, T3; o segundo T3 em 10 min vira T2.
    assert_eq!(
        niveis_das_festas("pequeno"),
        vec![(15_800, T1, "done_small")]
    );
    let medio = linha_do_tempo("medio");
    assert_eq!(festas(&medio)[0].2, T2);
    assert!(matches!(
        so(&medio, "festa")[0].tipo,
        Tipo::Festa {
            confete: 12,
            voo: Some("curto"),
            ..
        }
    ));
    assert_eq!(
        niveis_das_festas("grande")
            .iter()
            .map(|f| f.1)
            .collect::<Vec<_>>(),
        vec![T3, T2, T3]
    );
    // idle-prompt-repetido: o idle_prompt nunca vira aviso nem festa.
    let linha = linha_do_tempo("idle-prompt-repetido");
    assert!(so(&linha, "escalada").is_empty());
    assert_eq!(festas(&linha).len(), 1, "só a do turno de verdade");
    // servidor-em-segundo-plano: festas normais com o servidor rodando; a
    // notificação do shell é de máquina, sem festa.
    let linha = linha_do_tempo("servidor-em-segundo-plano");
    assert_eq!(festas(&linha).len(), 4);
    assert!(
        turnos(&linha)
            .iter()
            .any(|(_, _, origem)| *origem == Some(OrigemTurno::Notificacao))
    );
    // workflow-longo: uma festa só, o T3, no Stop que fecha a corrente.
    assert_eq!(
        niveis_das_festas("workflow-longo"),
        vec![(1_213_800, T3, "done_big")]
    );
    // workflow-agentes-depois-do-stop: os agentes que nascem na acomodação e
    // depois do turno comemorado são da corrente (decisão 0089).
    let linha = linha_do_tempo("workflow-agentes-depois-do-stop");
    assert_eq!(
        niveis_das_festas("workflow-agentes-depois-do-stop"),
        vec![(872_800, T3, "done_big")]
    );
    assert_eq!(
        turnos(&linha)
            .iter()
            .map(|(t, fim, _)| (t.as_str(), *fim))
            .collect::<Vec<_>>(),
        vec![
            ("p1", crate::cerebro::Fim::Stop),
            ("p2", crate::cerebro::Fim::Stop)
        ],
        "nada reabre nem fica substituído"
    );
    // digitado-durante-a-corrente: o pedido digitado festeja sozinho, e a
    // corrente fecha na notificação (decisão 0089).
    let linha = linha_do_tempo("digitado-durante-a-corrente");
    assert_eq!(
        niveis_das_festas("digitado-durante-a-corrente"),
        vec![(45_800, T1, "done_small"), (166_700, T1, "done_small")]
    );
    assert_eq!(
        turnos(&linha)
            .iter()
            .map(|(t, _, origem)| (t.as_str(), *origem))
            .collect::<Vec<_>>(),
        vec![
            ("p1", None),
            ("p2", None),
            ("p3", Some(OrigemTurno::Notificacao))
        ]
    );
    assert!(
        so(&linha, "turno").iter().any(
            |x| matches!(&x.tipo, Tipo::Turno { turno8: Some(t), corrente: None, .. } if t == "p2")
        ),
        "o pedido digitado fica fora da corrente"
    );
    // stop-bloqueado: a continuação só festeja se subir.
    assert_eq!(
        niveis_das_festas("stop-bloqueado"),
        vec![
            (5_800, T0, "nod"),
            (12_800, T1, "done_small"),
            (25_800, T1, "done_small")
        ],
        "a continuação de p2 no mesmo nível fica só no registro"
    );
    // interrompido: sem festa, com e sem o PostToolUseFailure.
    let linha = linha_do_tempo("interrompido");
    assert_eq!(
        festas(&linha)
            .iter()
            .map(|f| f.1.as_str())
            .collect::<Vec<_>>(),
        vec!["s2"],
        "só o turno seguinte, q2"
    );
    assert!(
        turnos(&linha)
            .iter()
            .any(|(t, fim, _)| t == "p1" && *fim == crate::cerebro::Fim::Interrompido)
    );
    // erro-limite: o cansado e o erro, sem festa.
    let linha = linha_do_tempo("erro-limite");
    assert!(festas(&linha).is_empty());
    let reacoes: Vec<&str> = so(&linha, "reacao")
        .iter()
        .filter_map(|x| match &x.tipo {
            Tipo::Reacao { motivo, .. } => Some(*motivo),
            _ => None,
        })
        .collect();
    assert_eq!(reacoes, vec!["cansado", "erro"]);
    // real-agente-em-segundo-plano: uma festa só pela corrente.
    assert_eq!(
        niveis_das_festas("real-agente-em-segundo-plano"),
        vec![(12_387, T1, "done_small")]
    );
    assert_eq!(
        niveis_das_festas("real-agente-dentro-da-acomodacao"),
        vec![(9_104, T1, "done_small")]
    );
    // real-servidor-e-shell-curto: o servidor não segura; a notificação do
    // shell curto é de máquina.
    let linha = linha_do_tempo("real-servidor-e-shell-curto");
    assert_eq!(festas(&linha).len(), 3);
    assert_eq!(
        turnos(&linha).last().map(|t| t.2),
        Some(Some(OrigemTurno::Notificacao))
    );
    // real-laco: a notificação e o tique (o Renan longe) são de máquina,
    // sem festa nem pronto; o /exit dá tchau.
    let linha = linha_do_tempo("real-laco");
    assert_eq!(
        festas(&linha).iter().map(|f| f.0).collect::<Vec<_>>(),
        vec![17_164, 33_010]
    );
    assert_eq!(
        turnos(&linha)
            .iter()
            .filter_map(|t| t.2)
            .collect::<Vec<_>>(),
        vec![OrigemTurno::Notificacao, OrigemTurno::Tique]
    );
    assert!(
        so(&linha, "reacao")
            .iter()
            .any(|x| matches!(&x.tipo, Tipo::Reacao { nome, .. } if nome == "bye"))
    );
    // real-pergunta-e-plano: cada diálogo é um aviso só, na L1.
    let linha = linha_do_tempo("real-pergunta-e-plano");
    let n: Vec<u8> = niveis(&linha).iter().map(|(_, n)| *n).collect();
    assert_eq!(n, vec![1, 0, 1, 0]);
    assert_eq!(chamadas(&linha), 2);
    // real-esc-e-compact: nada de festa; o pet volta a parado.
    let linha = linha_do_tempo("real-esc-e-compact");
    assert!(festas(&linha).is_empty());
    assert!(matches!(
        so(&linha, "base").last().map(|x| &x.tipo),
        Some(Tipo::Base { estado: "idle", .. })
    ));
}

/// O `bichinho simular` roda sem personagem: em todo cenário, a linha do
/// tempo tem de ser a mesma do dourado (que roda com a `_teste`).
#[test]
fn sem_personagem_a_linha_do_tempo_e_a_mesma() {
    for nome in nomes() {
        let texto = std::fs::read_to_string(pasta().join(format!("{nome}.jsonl"))).unwrap();
        let cenario = ler(&nome, &texto).unwrap();
        assert_eq!(
            linhas(&rodar(&cenario, None).unwrap()),
            linhas(&linha_do_tempo(&nome)),
            "{nome}"
        );
    }
}

// --- a memória das sessões (decisão 0093) -----------------------------------

/// Os cenários de reinício e o instante em que o pet volta em cada um.
const REINICIOS: [(&str, u64); 15] = [
    ("reinicio-sessao-parada", 60_000),
    ("reinicio-no-meio-do-turno", 75_000),
    ("reinicio-com-pergunta", 120_000),
    ("reinicio-com-pergunta-respondida-fora", 35_000),
    ("reinicio-com-pronto-e-erro", 70_000),
    ("reinicio-depois-de-13-h", 46_810_000),
    ("reinicio-da-maquina", 150_000),
    ("reinicio-com-outro-compositor", 50_000),
    ("reinicio-com-arquivo-corrompido", 25_000),
    ("reinicio-com-nao-perturbe", 65_000),
    ("reinicio-na-soneca", 65_000),
    ("reinicio-compartilhando", 81_000),
    ("reinicio-depois-de-uma-pausa", 620_000),
    ("reinicio-na-acomodacao", 11_500),
    ("reinicio-com-outro-dialogo", 140_000),
];

/// O que tocou entre `de` e `ate`: uma reação, uma rajada, um voo, uma festa
/// ou um balão.
fn tocou_entre(linha: &[Intencao], de: u64, ate: u64) -> Vec<&Intencao> {
    linha
        .iter()
        .filter(|x| {
            (de..ate).contains(&x.t_ms)
                && matches!(
                    x.tipo,
                    Tipo::Reacao { .. }
                        | Tipo::Rajada { .. }
                        | Tipo::Voo { .. }
                        | Tipo::Festa { .. }
                        | Tipo::FestaMesclada { .. }
                        | Tipo::Balao { .. }
                )
        })
        .collect()
}

/// A restauração de um cenário: (sessões, avisos, de fora, motivo).
fn restauracao(linha: &[Intencao]) -> (u32, u32, u32, Option<&'static str>) {
    let r = so(linha, "restauracao");
    assert_eq!(r.len(), 1, "uma restauração por reinício");
    match r[0].tipo {
        Tipo::Restauracao {
            sessoes,
            avisos,
            de_fora,
            motivo,
            ..
        } => (sessoes, avisos, de_fora, motivo),
        _ => unreachable!(),
    }
}

/// As listas do clique (o balão de motivo `lista`), com o instante.
fn listas(linha: &[Intencao]) -> Vec<(u64, Vec<String>)> {
    linha
        .iter()
        .filter_map(|x| match &x.tipo {
            Tipo::Balao {
                linhas,
                motivo: "lista",
            } => Some((x.t_ms, linhas.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn o_pet_que_reinicia_volta_quieto() {
    // Nada toca na volta: nem reação, nem festa, nem balão, nem rajada ou
    // voo, em nenhum dos reinícios (decisão 0093); o que vem depois é pelos
    // eventos e pelos prazos de sempre.
    for (nome, volta) in REINICIOS {
        let linha = linha_do_tempo(nome);
        let na_volta = tocou_entre(&linha, volta, volta + 1);
        assert!(na_volta.is_empty(), "{nome}: tocou na volta: {na_volta:?}");
        let r = so(&linha, "restauracao");
        assert_eq!(
            r.iter().map(|x| x.t_ms).collect::<Vec<_>>(),
            vec![volta],
            "{nome}: a restauração na volta"
        );
    }
}

#[test]
fn a_sessao_parada_volta_na_lista_com_o_tempo_de_antes_e_a_de_teste_nao() {
    let linha = linha_do_tempo("reinicio-sessao-parada");
    assert_eq!(restauracao(&linha), (1, 0, 0, None));
    assert_eq!(
        listas(&linha),
        vec![(70_000, vec!["api: parado (50 s)".to_owned()])],
        "parada desde o Stop de antes (20 s), sem a sessão de teste"
    );
    // O prompt seguinte é um turno como outro qualquer.
    assert_eq!(
        festas(&linha).last().map(|f| (f.0, f.2)),
        Some((90_800, crate::cerebro::Nivel::T1))
    );
}

#[test]
fn o_turno_aberto_na_parada_nao_volta_e_o_stop_perdido_espera_o_idle_prompt() {
    let linha = linha_do_tempo("reinicio-no-meio-do-turno");
    assert_eq!(restauracao(&linha), (2, 0, 0, None));
    // As duas voltam trabalhando (o último evento de cada uma há menos de 5
    // min), a base com a de evento mais novo.
    assert!(so(&linha, "base").iter().any(|x| x.t_ms == 75_000
        && matches!(&x.tipo, Tipo::Base { estado: "working", sid8: Some(s), .. } if s == "s2")));
    // A de api segue o turno: o Stop festeja só o que veio depois da volta
    // (20 s de Bash: 0,48), num turno implícito, com o pronto.
    assert!(so(&linha, "turno").iter().any(|x| x.t_ms == 100_800
        && matches!(&x.tipo, Tipo::Turno { turno8: Some(t), pontuacao: Some(p), .. }
            if t == "p1" && (*p - 0.48).abs() < 1e-9)));
    // O Stop da web se perdeu com o pet fora: nenhum turno dela fecha, e o
    // idle_prompt a deixa parada (o "+1" some) sem festa.
    assert!(
        !so(&linha, "turno")
            .iter()
            .any(|x| matches!(&x.tipo, Tipo::Turno { sid8, .. } if sid8 == "s2"))
    );
    assert!(
        so(&linha, "selos")
            .iter()
            .any(|x| x.t_ms == 130_000 && matches!(&x.tipo, Tipo::Selos(s) if s.mais == 0))
    );
    assert_eq!(festas(&linha).len(), 1);
}

#[test]
fn a_pergunta_volta_e_a_escalada_segue_do_tempo_que_passou() {
    let linha = linha_do_tempo("reinicio-com-pergunta");
    assert_eq!(restauracao(&linha), (1, 1, 0, None));
    assert_eq!(
        chamadas(&linha),
        1,
        "a chamada da L1 não se repete na volta"
    );
    assert!(
        so(&linha, "escalada").iter().any(|x| x.t_ms == 120_000
            && matches!(
                x.tipo,
                Tipo::Escalada {
                    nivel: 3,
                    motivo: "restaurada",
                    ..
                }
            )),
        "o nível de antes, sem chamar"
    );
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
            (100_000, "escalada"),
            (180_000, "escalada"),
            (240_000, "escalada"),
            (700_000, "voltou")
        ],
        "o voo de antes conta; os outros dois saem um minuto inteiro depois da volta"
    );
    // A L4 na hora dela: 5 min depois do aviso de antes da partida.
    assert!(
        niveis(&linha).contains(&(310_000, 4)),
        "{:?}",
        niveis(&linha)
    );
    assert!(tocou_entre(&linha, 110_000, 180_000).is_empty());
    assert!(
        so(&linha, "pulso")
            .iter()
            .any(|x| x.t_ms == 310_000 && matches!(x.tipo, Tipo::Pulso { ligado: true, .. }))
    );
}

#[test]
fn a_pergunta_respondida_com_o_pet_fora_sai_no_idle_prompt() {
    // A resposta e o Stop se perderam com o pet fora: a espera volta e segue
    // até o idle_prompt, que nunca sai com um diálogo na tela (decisão 0094).
    let linha = linha_do_tempo("reinicio-com-pergunta-respondida-fora");
    assert_eq!(restauracao(&linha), (1, 1, 0, None));
    assert_eq!(chamadas(&linha), 1);
    assert_eq!(niveis(&linha).last(), Some(&(90_000, 0)));
    assert!(so(&linha, "escalada").iter().any(|x| x.t_ms == 90_000
        && matches!(
            x.tipo,
            Tipo::Escalada {
                motivo: "andou",
                ..
            }
        )));
    assert!(festas(&linha).is_empty(), "o turno de antes não festeja");
    assert!(tocou_entre(&linha, 90_001, 100_000).is_empty());
    assert!(
        so(&linha, "base")
            .iter()
            .any(|x| x.t_ms == 90_000 && matches!(x.tipo, Tipo::Base { estado: "idle", .. }))
    );
}

#[test]
fn o_pronto_e_o_erro_voltam_sem_festa_nem_susto_e_o_clique_leva_aos_terminais() {
    let linha = linha_do_tempo("reinicio-com-pronto-e-erro");
    assert_eq!(restauracao(&linha), (2, 2, 0, None));
    // O susto só na hora do erro, uma vez; a festa, uma.
    let sustos = so(&linha, "reacao")
        .iter()
        .filter(|x| matches!(&x.tipo, Tipo::Reacao { nome, .. } if nome == "error"))
        .count();
    assert_eq!(sustos, 1);
    assert_eq!(festas(&linha).len(), 1);
    // O erro segura a base até os 60 s dele; depois, o pronto, até 2 min
    // depois da festa de antes.
    let bases: Vec<(u64, &str)> = so(&linha, "base")
        .iter()
        .map(|x| match x.tipo {
            Tipo::Base { estado, .. } => (x.t_ms, estado),
            _ => unreachable!(),
        })
        .filter(|(t, _)| *t >= 70_000)
        .collect();
    assert_eq!(
        bases,
        vec![(70_000, "error"), (100_000, "ready"), (130_500, "idle")]
    );
    let cliques: Vec<(u64, &str, Option<String>)> = so(&linha, "clique")
        .iter()
        .map(|x| match &x.tipo {
            Tipo::Clique { resultado, sid8 } => (x.t_ms, *resultado, sid8.clone()),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        cliques,
        vec![
            (120_000, "focou", Some("s2".to_owned())),
            (130_000, "focou", Some("s1".to_owned()))
        ],
        "o erro primeiro, depois o pronto, cada um no terminal de antes"
    );
}

#[test]
fn depois_de_13_h_as_duas_sessoes_voltam_e_ficam_na_lista() {
    // A vida de uma sessão sem evento é de uma semana (decisão 0096): a de 13
    // h e a de 11 h voltam e continuam abertas uma hora depois.
    let linha = linha_do_tempo("reinicio-depois-de-13-h");
    assert_eq!(restauracao(&linha), (2, 0, 0, None));
    assert_eq!(
        listas(&linha),
        vec![
            (
                46_820_000,
                vec![
                    "api: parado (11 h)".to_owned(),
                    "velha: parado (13 h)".to_owned()
                ]
            ),
            (
                50_410_000,
                vec![
                    "api: parado (12 h)".to_owned(),
                    "velha: parado (14 h)".to_owned()
                ]
            )
        ]
    );
}

#[test]
fn a_maquina_que_reiniciou_e_o_arquivo_ruim_nao_trazem_nada() {
    for (nome, motivo, clique) in [
        ("reinicio-da-maquina", "maquina_reiniciou", 160_000),
        ("reinicio-com-arquivo-corrompido", "arquivo_ruim", 30_000),
    ] {
        let linha = linha_do_tempo(nome);
        assert_eq!(restauracao(&linha), (0, 0, 0, Some(motivo)), "{nome}");
        assert!(
            listas(&linha).contains(&(clique, vec!["nenhuma sessão do Claude aberta".to_owned()])),
            "{nome}"
        );
    }
    let linha = linha_do_tempo("reinicio-da-maquina");
    assert!(
        so(&linha, "escalada").iter().all(|x| x.t_ms < 60_000),
        "a escalada de antes não volta"
    );
}

#[test]
fn noutro_compositor_a_sessao_volta_sem_a_janela_ate_o_proximo_prompt() {
    let linha = linha_do_tempo("reinicio-com-outro-compositor");
    assert_eq!(restauracao(&linha), (1, 1, 0, None));
    assert!(
        baloes(&linha)
            .iter()
            .any(|(t, linhas)| *t == 60_000 && linhas.iter().any(|l| l == "a janela dela fechou"))
    );
    let cliques: Vec<(u64, &str)> = so(&linha, "clique")
        .iter()
        .map(|x| match &x.tipo {
            Tipo::Clique { resultado, .. } => (x.t_ms, *resultado),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(cliques, vec![(60_000, "nao_focou"), (90_000, "focou")]);
}

#[test]
fn com_o_pet_fora_do_ar_so_um_evento_se_perde() {
    let c = ler(
        "x",
        r#"{"cenario": "x", "padrao": {"sid": "s1", "ent": "cli"}}
{"t": 0, "evento": {"e": "UserPromptSubmit", "turno": "p1"}}
{"t": 1000, "reinicio": {"parado_ms": 5000}}
{"t": 2000, "evento": {"e": "Stop", "turno": "p1"}}
{"t": 3000, "clique": "esquerdo"}
{"t": 9000, "fim": true}"#,
    )
    .unwrap();
    let e = rodar(&c, None).unwrap_err();
    assert!(e.contains("fora do ar"), "{e}");
}

// --- a revisão da memória (decisão 0095) --------------------------------------

/// O que a restauração anotou além das contas: (velha, sossego).
fn restauracao_extra(linha: &[Intencao]) -> (bool, Vec<&'static str>) {
    match &so(linha, "restauracao")[0].tipo {
        Tipo::Restauracao { velha, sossego, .. } => (*velha, sossego.clone()),
        _ => unreachable!(),
    }
}

/// As rajadas, os voos e o pulso que liga entre `de` e `ate`.
fn barulho_entre(linha: &[Intencao], de: u64, ate: u64) -> Vec<&Intencao> {
    linha
        .iter()
        .filter(|x| {
            (de..ate).contains(&x.t_ms)
                && matches!(
                    x.tipo,
                    Tipo::Rajada { .. } | Tipo::Voo { .. } | Tipo::Pulso { ligado: true, .. }
                )
        })
        .collect()
}

#[test]
fn o_sossego_volta_com_a_memoria_e_segura_a_escalada_ate_acabar() {
    // O "não perturbe" até o próximo evento (aos 400 s, sem o dnd), a soneca
    // até 30 min depois do clique (1 812 s) e a discrição até 5 min depois do
    // último sinal (316 s): nada acima da L1 até lá; depois, a L4 do aviso de
    // antes, na hora (decisão 0095).
    for (nome, sossego, fim) in [
        ("reinicio-com-nao-perturbe", "nao_perturbe", 400_000),
        ("reinicio-na-soneca", "soneca", 1_812_000),
        ("reinicio-compartilhando", "discricao", 316_000),
    ] {
        let linha = linha_do_tempo(nome);
        assert_eq!(restauracao(&linha), (1, 1, 0, None), "{nome}");
        assert_eq!(restauracao_extra(&linha), (false, vec![sossego]), "{nome}");
        let antes = barulho_entre(&linha, 0, fim);
        assert!(antes.is_empty(), "{nome}: {antes:?}");
        assert!(
            niveis(&linha).contains(&(fim, 4)),
            "{nome}: {:?}",
            niveis(&linha)
        );
        assert!(!barulho_entre(&linha, fim, u64::MAX).is_empty(), "{nome}");
    }
    // Na discrição, a festa de outra sessão sai sem o nome do projeto.
    let linha = linha_do_tempo("reinicio-compartilhando");
    assert!(baloes(&linha).contains(&(110_800, vec!["Prontinho!".to_owned()])));
    assert!(
        so(&linha, "discricao")
            .iter()
            .any(|x| x.t_ms == 316_000 && matches!(x.tipo, Tipo::Discricao { ligada: false, .. }))
    );
}

#[test]
fn a_memoria_velha_volta_com_a_espera_quieta_e_sem_a_janela() {
    // 10 min fora: a resposta, o Stop e o idle_prompt se perderam. A espera
    // volta dada como vista (nada acima da L1, sem a pose), e o clique não
    // foca a janela de antes, que pode ser de outra agora (decisão 0095).
    let linha = linha_do_tempo("reinicio-depois-de-uma-pausa");
    assert_eq!(restauracao(&linha), (1, 1, 0, None));
    assert_eq!(restauracao_extra(&linha), (true, Vec::new()));
    assert!(barulho_entre(&linha, 0, u64::MAX).is_empty());
    assert!(
        !so(&linha, "base").iter().any(|x| x.t_ms >= 620_000
            && matches!(
                x.tipo,
                Tipo::Base {
                    estado: "waiting",
                    ..
                }
            )),
        "a pose de espera não volta"
    );
    assert!(
        baloes(&linha)
            .iter()
            .any(|(t, linhas)| *t == 630_000 && linhas.iter().any(|l| l == "não vi a janela dela"))
    );
    // O prompt seguinte tira a espera e casa a janela: o clique no pronto
    // dele foca.
    let cliques: Vec<(u64, &str)> = so(&linha, "clique")
        .iter()
        .map(|x| match &x.tipo {
            Tipo::Clique { resultado, .. } => (x.t_ms, *resultado),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(cliques, vec![(630_000, "nao_focou"), (1_815_000, "focou")]);
}

#[test]
fn a_parada_na_acomodacao_do_stop_traz_o_pronto_sem_festa() {
    let linha = linha_do_tempo("reinicio-na-acomodacao");
    assert_eq!(restauracao(&linha), (1, 1, 0, None));
    assert!(
        festas(&linha).is_empty(),
        "a festa ficou na acomodação perdida"
    );
    assert!(so(&linha, "base").iter().any(|x| x.t_ms == 11_500
        && matches!(
            x.tipo,
            Tipo::Base {
                estado: "ready",
                ..
            }
        )));
    assert!(so(&linha, "clique").iter().any(|x| x.t_ms == 20_000
        && matches!(
            x.tipo,
            Tipo::Clique {
                resultado: "focou",
                ..
            }
        )));
}

// --- todas as sessões abertas (decisão 0096) -----------------------------------

#[test]
fn a_espera_de_uma_noite_sai_em_12_h_e_a_sessao_fica() {
    // Ninguém responde e a sessão não manda mais nada: a escalada até o teto;
    // 12 h depois do último evento, a espera sai (`expirou`) e a sessão
    // continua na lista do clique (antes, ela inteira saía em 12 h).
    let linha = linha_do_tempo("espera-de-uma-noite");
    assert_eq!(niveis(&linha).last(), Some(&(43_210_000, 0)));
    assert!(so(&linha, "escalada").iter().any(|x| x.t_ms == 43_210_000
        && matches!(
            x.tipo,
            Tipo::Escalada {
                motivo: "expirou",
                ..
            }
        )));
    assert!(barulho_entre(&linha, 2_110_000, u64::MAX).is_empty());
    assert_eq!(
        listas(&linha),
        vec![(46_800_000, vec!["api: parado (59 min)".to_owned()])]
    );
}

// --- a revisão final do M5 (decisões 0097 e 0098) ------------------------------

#[test]
fn outro_dialogo_depois_da_volta_chama_como_no_pet_de_pe() {
    // A resposta da pergunta se perdeu com o pet fora (2 min: a memória
    // velha). A permissão do Bash, o primeiro evento depois da volta, é outro
    // diálogo: chama na hora e escala, como no pet de pé (decisão 0097).
    let linha = linha_do_tempo("reinicio-com-outro-dialogo");
    assert_eq!(restauracao(&linha), (1, 1, 0, None));
    assert_eq!(restauracao_extra(&linha), (true, Vec::new()));
    assert_eq!(chamadas(&linha), 2, "a da pergunta e a do Bash");
    assert!(baloes(&linha).contains(&(
        150_000,
        vec![
            "Ô, meu camarada!".to_owned(),
            "api precisa de você".to_owned()
        ]
    )));
    assert_eq!(
        niveis(&linha),
        vec![
            (10_000, 1),
            (140_000, 1),
            (150_000, 0),
            (150_000, 1),
            (180_000, 2),
            (240_000, 3),
            (260_000, 0)
        ]
    );
    assert_eq!(so(&linha, "rajada").len(), 5);
    assert_eq!(so(&linha, "voo").len(), 1);
}
