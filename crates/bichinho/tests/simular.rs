//! `bin/pet simular` e `bin/pet eventos --salvar` de ponta a ponta (decisão
//! 0078): o cenário no relógio falso pelo `bichinho` que o cargo acabou de
//! compilar, e o cenário gravado de um daemon de debug numa porta própria,
//! com os pseudônimos. Um daemon sem debug é recusado antes de qualquer
//! pedido de eventos.

mod comum;

use std::process::{Command, Output};

use comum::Daemon;

/// O `bichinho` que o cargo acabou de compilar (o `bin/pet` o usa pelo
/// `PET_BICHINHO`, no lugar do `cargo run`).
const BICHINHO: &str = env!("CARGO_BIN_EXE_bichinho");

fn pet(argumentos: &[&str], porta: Option<u16>) -> Output {
    let mut cmd = Command::new(comum::raiz().join("bin/pet"));
    cmd.args(argumentos).env("PET_BICHINHO", BICHINHO);
    if let Some(porta) = porta {
        cmd.env("PET_PORTA", porta.to_string());
    }
    cmd.output().expect("rodar bin/pet")
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn simular_imprime_o_dourado() {
    let saida = pet(&["simular", "pergunta"], None);
    assert!(saida.status.success(), "{}", texto(&saida.stderr));
    let esperado =
        std::fs::read_to_string(comum::raiz().join("cenarios/pergunta.esperado.jsonl")).unwrap();
    assert_eq!(texto(&saida.stdout), esperado);
    let nada = pet(&["simular", "nao-existe"], None);
    assert_eq!(nada.status.code(), Some(1));
    assert!(texto(&nada.stderr).contains("não achei o cenário"));
    let sem = Command::new(BICHINHO).arg("simular").output().unwrap();
    assert_eq!(sem.status.code(), Some(2), "sem o arquivo: uso");
}

#[test]
fn eventos_salvar_grava_o_cenario_com_pseudonimos() {
    let d = Daemon::subir(true);
    let sid = "aaaaaaaa-1111-4222-8333-444444444444";
    for corpo in [
        format!(
            r#"{{"v":1,"e":"UserPromptSubmit","sid":"{sid}","turno":"turno-secreto","ent":"cli","proj":"agenda-presidencial","orig":"comum"}}"#
        ),
        format!(
            r#"{{"v":1,"e":"PostToolUse","sid":"{sid}","turno":"turno-secreto","ent":"cli","proj":"agenda-presidencial","tool":"Edit","arq":"9f86d081884c","dur":30}}"#
        ),
        format!(
            r#"{{"v":1,"e":"Stop","sid":"{sid}","turno":"turno-secreto","ent":"cli","proj":"agenda-presidencial","bg":0}}"#
        ),
    ] {
        let (status, resposta) = d.post_json("/v1/evento", &corpo);
        assert_eq!(status, 204, "{resposta}");
    }
    d.esperar_estado("os três eventos", |e| e["eventos"]["aceitos"] == 3);
    let pasta = comum::pasta_temporaria("eventos-salvar");
    let arquivo = pasta.join("meu-dia.jsonl");
    let saida = pet(
        &["eventos", "--salvar", arquivo.to_str().unwrap()],
        Some(d.porta),
    );
    assert!(saida.status.success(), "{}", texto(&saida.stderr));
    assert!(
        texto(&saida.stdout).contains("3 eventos"),
        "{}",
        texto(&saida.stdout)
    );
    let gravado = std::fs::read_to_string(&arquivo).unwrap();
    for proibido in [sid, "turno-secreto", "agenda", "9f86d081884c"] {
        assert!(
            !gravado.contains(proibido),
            "«{proibido}» vazou:\n{gravado}"
        );
    }
    assert!(gravado.contains(r#""cenario":"meu-dia""#));
    assert!(gravado.contains(r#""sid":"s1""#) && gravado.contains(r#""turno":"p1""#));
    // O cenário gravado roda: a edição dá o pulinho.
    let simulado = pet(&["simular", arquivo.to_str().unwrap()], None);
    assert!(simulado.status.success(), "{}", texto(&simulado.stderr));
    assert!(
        texto(&simulado.stdout).contains(r#""i":"festa","sid8":"s1","nivel":"T1""#),
        "{}",
        texto(&simulado.stdout)
    );
    let _ = std::fs::remove_dir_all(&pasta);
}

#[test]
fn eventos_recusa_um_pet_sem_debug() {
    let d = Daemon::subir(false);
    let pasta = comum::pasta_temporaria("eventos-recusa");
    let arquivo = pasta.join("nada.jsonl");
    let saida = pet(
        &["eventos", "--salvar", arquivo.to_str().unwrap()],
        Some(d.porta),
    );
    assert_eq!(saida.status.code(), Some(1));
    assert!(
        texto(&saida.stderr).contains("não está em debug"),
        "{}",
        texto(&saida.stderr)
    );
    assert!(!arquivo.exists(), "nada gravado");
    let _ = std::fs::remove_dir_all(&pasta);
}
