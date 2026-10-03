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
