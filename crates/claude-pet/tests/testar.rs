//! `bin/pet testar` de ponta a ponta: o CLI manda os eventos sintéticos
//! pelo `avisar.sh` de verdade (curl de verdade) para um daemon numa porta
//! própria, e o cérebro escolhe a reação.

mod comum;

use std::process::Command;

use comum::Daemon;

fn testar(d: &Daemon, cenario: &str) -> (bool, String, String) {
    let saida = Command::new(comum::raiz().join("bin/pet"))
        .args(["testar", cenario])
        .env("PET_PORTA", d.porta.to_string())
        .output()
        .expect("rodar bin/pet");
    (
        saida.status.success(),
        String::from_utf8_lossy(&saida.stdout).into_owned(),
        String::from_utf8_lossy(&saida.stderr).into_owned(),
    )
}

#[test]
fn rapido_acena_e_pequeno_pula() {
    let d = Daemon::subir(false);
    let (ok, saida, erro) = testar(&d, "rapido");
    assert!(ok, "rapido falhou: {saida}{erro}");
    assert!(saida.contains("✓ rapido → nod"), "{saida}");
    let (ok, saida, erro) = testar(&d, "pequeno");
    assert!(ok, "pequeno falhou: {saida}{erro}");
    assert!(saida.contains("✓ pequeno → done_small"), "{saida}");
    assert!(saida.contains(r#""nivel":"T1","trabalho":2"#), "{saida}");
    let estado = d.get_json("/v1/estado");
    let sessoes = estado["sessoes"].as_array().unwrap();
    assert_eq!(sessoes.len(), 2, "uma sessão por execução: {estado:#}");
    assert!(sessoes.iter().all(|s| s["teste"] == true), "{estado:#}");
    assert!(sessoes.iter().all(|s| s["proj"] == "pet-testar"));
    assert_eq!(estado["ultima_reacao"]["teste"], true);
}

#[test]
fn testar_acha_a_propria_reacao_com_sessoes_reais_reagindo() {
    // A `ultima_reacao` é de quem reagiu por último: com sessões reais
    // reagindo a cada 50 ms, o `bin/pet testar` ainda acha a reação dele,
    // pelo registro do próprio turno (decisão 0034).
    let d = Daemon::subir(false);
    let porta = d.porta;
    let parar = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let fundo = {
        let parar = std::sync::Arc::clone(&parar);
        std::thread::spawn(move || {
            let mut i = 0u32;
            while !parar.load(std::sync::atomic::Ordering::Relaxed) {
                for e in ["UserPromptSubmit", "Stop"] {
                    let corpo = format!(
                        r#"{{"v":1,"e":"{e}","sid":"{i:08x}-real","turno":"p{i}","ent":"cli"}}"#
                    );
                    let pedido = format!(
                        "POST /v1/evento HTTP/1.1\r\nHost: 127.0.0.1:{porta}\r\nX-Pet: 1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{corpo}",
                        corpo.len()
                    );
                    assert_eq!(comum::bruto(porta, &pedido).0, 204);
                }
                i += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            i
        })
    };
    let (ok, saida, erro) = testar(&d, "pequeno");
    parar.store(true, std::sync::atomic::Ordering::Relaxed);
    let reais = fundo.join().unwrap();
    assert!(ok, "pequeno falhou: {saida}{erro}");
    assert!(saida.contains("✓ pequeno → done_small"), "{saida}");
    assert!(reais >= 10, "só {reais} sessões reais");
    let estado = d.esperar_estado("as reais reagiram", |e| {
        e["ultima_reacao"]["teste"] == false
    });
    assert_eq!(estado["ultima_reacao"]["nome"], "nod");
}

#[test]
fn proxy_e_curlrc_nao_desviam_o_cli() {
    // O `bin/pet` e o `avisar.sh` só falam com o pet do 127.0.0.1, mesmo com
    // proxy no ambiente e um curlrc desviando tudo (decisão 0031).
    let d = Daemon::subir(false);
    let pasta = d.pasta.join("config-curl");
    std::fs::create_dir_all(&pasta).unwrap();
    for nome in [".curlrc", "curlrc"] {
        std::fs::write(
            pasta.join(nome),
            "proxy = \"http://127.0.0.1:9\"\nconnect-to = \"::127.0.0.1:9\"\n",
        )
        .unwrap();
    }
    let mut cmd = Command::new(comum::raiz().join("bin/pet"));
    cmd.args(["testar", "rapido"])
        .env("PET_PORTA", d.porta.to_string())
        .env("CURL_HOME", &pasta)
        .env("XDG_CONFIG_HOME", &pasta);
    for variavel in [
        "http_proxy",
        "HTTP_PROXY",
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
    ] {
        cmd.env(variavel, "http://127.0.0.1:9");
    }
    let saida = cmd.output().expect("rodar bin/pet");
    let texto = String::from_utf8_lossy(&saida.stdout);
    assert!(
        saida.status.success(),
        "{texto}{}",
        String::from_utf8_lossy(&saida.stderr)
    );
    assert!(texto.contains("✓ rapido → nod"), "{texto}");
}

#[test]
fn cenario_desconhecido_e_pet_desligado() {
    let d = Daemon::subir(false);
    let (ok, _, erro) = testar(&d, "gigante");
    assert!(!ok);
    assert!(erro.contains("uso: bin/pet testar"), "{erro}");
    let porta = d.porta;
    drop(d);
    let saida = Command::new(comum::raiz().join("bin/pet"))
        .args(["testar", "rapido"])
        .env("PET_PORTA", porta.to_string())
        .output()
        .unwrap();
    assert!(!saida.status.success());
    assert!(
        String::from_utf8_lossy(&saida.stderr).contains("o pet não respondeu"),
        "{saida:?}"
    );
}

/// `bin/pet <args>` contra o daemon `d`: (sucesso, saída, erro).
fn pet(d: &Daemon, args: &[&str]) -> (bool, String, String) {
    let saida = Command::new(comum::raiz().join("bin/pet"))
        .args(args)
        .env("PET_PORTA", d.porta.to_string())
        .output()
        .expect("rodar bin/pet");
    (
        saida.status.success(),
        String::from_utf8_lossy(&saida.stdout).into_owned(),
        String::from_utf8_lossy(&saida.stderr).into_owned(),
    )
}

#[test]
fn reacoes_e_aprovacoes_pelo_mesmo_comando() {
    // O CLI fala com o mesmo `/v1/comando` para as reações do M3 e as
    // aprovações do M2 (decisão 0030).
    let d = Daemon::subir(true);
    for args in [["esconder"], ["mostrar"]] {
        let (ok, saida, erro) = pet(&d, &args);
        assert!(ok, "{args:?}: {saida}{erro}");
    }
    // O `tocar` conta o que o pet fez (decisão 0033): sem compositor, a
    // reação não aparece, e o CLI sai com erro dizendo por quê.
    let (ok, _, erro) = pet(&d, &["tocar", "nod"]);
    assert!(!ok);
    assert!(
        erro.contains("«nod» não apareceu: sem compositor (tag wave)"),
        "{erro}"
    );
    let (ok, _, erro) = pet(&d, &["tocar", "Nod!"]);
    assert!(!ok);
    assert!(erro.contains("o pet recusou (400)"), "{erro}");
    let (ok, _, erro) = pet(&d, &["tocar", "nada_disso"]);
    assert!(!ok);
    assert!(
        erro.contains("o pet recusou (400): a skin «_teste» não tem «nada_disso»"),
        "{erro}"
    );
    let (ok, saida, erro) = pet(&d, &["skin-revogar", "zeca"]);
    assert!(ok, "{saida}{erro}");
    assert!(saida.contains("«zeca» não estava aprovada"), "{saida}");
    let porta = d.porta;
    drop(d);
    let saida = Command::new(comum::raiz().join("bin/pet"))
        .args(["tocar", "nod"])
        .env("PET_PORTA", porta.to_string())
        .output()
        .unwrap();
    assert!(!saida.status.success());
    assert!(
        String::from_utf8_lossy(&saida.stderr).contains("o pet não respondeu"),
        "{saida:?}"
    );
}
