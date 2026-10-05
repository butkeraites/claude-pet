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
            mais: 0,
            bandeiras: vec![crate::motor::tela::cor("api")],
            corrente: false
        })),
        "e vira a bandeirinha"
    );
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
        vec![(12_000, true, true), (40_000, false, false)],
        "a captura de 1 s não liga; 2 s depois de começar, o balão com nome sai"
    );
    for (t, linhas) in baloes(&linha) {
        let texto = linhas.join(" ");
        if (12_000..40_000).contains(&t) {
            assert!(
                !texto.contains("agenda") && !texto.contains("web"),
                "{t}: {texto}"
            );
        }
    }
    assert!(baloes(&linha).contains(&(50_800, vec!["Prontinho! web".to_owned()])));
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
