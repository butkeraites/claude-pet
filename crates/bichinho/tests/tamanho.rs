//! O tamanho do pet no config (decisão 0042) e o nome do arquivo de config
//! (decisão 0041), no daemon de verdade (sem compositor: o D só aparece com
//! a janela; aqui confere-se a config que o pet leu).

mod comum;

use comum::Daemon;
use serde_json::json;

#[test]
fn tamanho_vem_do_config_e_vale_de_novo_a_cada_aprovacao() {
    let d = Daemon::subir(false);
    let tamanho =
        |d: &Daemon| d.get_json("/v1/estado")["config"]["chaves"]["aparencia.tamanho"].clone();
    assert_eq!(tamanho(&d), json!({"valor": "normal", "origem": "padrao"}));
    let pasta = d.pasta.join("config");
    std::fs::create_dir_all(&pasta).unwrap();
    std::fs::write(
        pasta.join("bichinho.toml"),
        "[aparencia]\ntamanho = \"pequeno\"\n",
    )
    .unwrap();
    // O config é relido a cada aprovação ou revogação (decisão 0029).
    let (status, corpo) = d.post_json("/v1/comando", r#"{"cmd":"revogar_skin","arg":"zeca"}"#);
    assert_eq!(status, 200, "{corpo}");
    assert_eq!(
        tamanho(&d),
        json!({"valor": "pequeno", "origem": "arquivo"})
    );
    assert!(
        d.log()
            .contains("config: aparencia.tamanho mudou de «normal» para «pequeno»"),
        "{}",
        d.log()
    );
    assert_eq!(
        d.get_json("/v1/estado")["config"]["chaves"]["aparencia.skin"]["valor"],
        "zeca",
        "a skin não muda"
    );
}

#[test]
fn o_config_de_nome_antigo_ainda_vale_com_aviso() {
    let pasta = comum::pasta_temporaria("config-antigo");
    std::fs::write(
        pasta.join("claude-pet.toml"),
        "[aparencia]\ntamanho = \"grande\"\n",
    )
    .unwrap();
    let caminho = pasta.display().to_string();
    let d = Daemon::subir_com(false, &[("PET_CONFIG", caminho.as_str())]);
    let estado = d.get_json("/v1/estado");
    assert_eq!(
        estado["config"]["chaves"]["aparencia.tamanho"],
        json!({"valor": "grande", "origem": "arquivo"})
    );
    assert!(d.log().contains("lendo o nome antigo"), "{}", d.log());
    // Com os dois, vale o novo.
    std::fs::write(
        pasta.join("bichinho.toml"),
        "[aparencia]\ntamanho = \"pequeno\"\n",
    )
    .unwrap();
    let (status, _) = d.post_json("/v1/comando", r#"{"cmd":"revogar_skin","arg":"zeca"}"#);
    assert_eq!(status, 200);
    assert_eq!(
        d.get_json("/v1/estado")["config"]["chaves"]["aparencia.tamanho"]["valor"],
        "pequeno"
    );
    drop(d);
    let _ = std::fs::remove_dir_all(&pasta);
}
