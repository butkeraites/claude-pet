//! Quadros dourados: a skin `_teste` renderizada em pixels do monitor, com
//! o SHA-256 do buffer BGRA guardado em `tests/ouro/*.sha256` (e uma prévia
//! PNG ao lado, só para olhos humanos).
//!
//! Mudou o raster ou a skin de propósito? Regere com
//! `PET_ATUALIZAR_OURO=1 cargo test -p pet-core --test ouro` e confira as
//! prévias antes de commitar.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pet_core::cena::{Elemento, rgba_do_sprite};
use pet_core::geometria::Ret;
use pet_core::raster::Alvo;
use pet_core::skin::{Skin, codificar_png};
use sha2::{Digest, Sha256};

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skin_teste() -> Skin {
    Skin::carregar(&raiz().join("skins/_teste")).expect("skins/_teste (cargo xtask skin-teste)")
}

/// Renderiza `elemento` num buffer `largura`x`altura` e devolve o BGRA.
fn renderizar(skin: &Skin, elemento: Elemento, largura: i32, altura: i32) -> Vec<u8> {
    let mut dados = vec![0u8; (largura * altura * 4) as usize];
    let mut alvo = Alvo::novo(&mut dados, largura, altura);
    let tudo = alvo.limites();
    elemento.desenhar(&mut alvo, skin, tudo);
    dados
}

fn conferir(nome: &str, skin: &Skin, elemento: Elemento, largura: i32, altura: i32) {
    let dados = renderizar(skin, elemento, largura, altura);
    let mut hash = String::new();
    for byte in Sha256::digest(&dados).iter() {
        write!(hash, "{byte:02x}").unwrap();
    }
    let pasta = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ouro");
    let arquivo = pasta.join(format!("{nome}.sha256"));
    if std::env::var("PET_ATUALIZAR_OURO").as_deref() == Ok("1") {
        std::fs::create_dir_all(&pasta).unwrap();
        std::fs::write(&arquivo, format!("{hash}\n")).unwrap();
        let rgba = rgba_do_sprite(skin, &elemento, Ret::novo(0, 0, largura, altura));
        let png = codificar_png(largura as u32, altura as u32, &rgba).unwrap();
        std::fs::write(pasta.join(format!("{nome}.png")), png).unwrap();
        return;
    }
    let esperado = std::fs::read_to_string(&arquivo)
        .unwrap_or_else(|_| panic!("falta {} (PET_ATUALIZAR_OURO=1)", arquivo.display()));
    assert_eq!(
        hash,
        esperado.trim(),
        "quadro dourado «{nome}» mudou; se foi de propósito, regere com PET_ATUALIZAR_OURO=1"
    );
}

#[test]
fn pose_parada_em_d6() {
    let skin = skin_teste();
    let pose = skin.tags_do_estado("idle")[0];
    let quadro = skin.tags[pose].de;
    let sprite = Elemento::Sprite {
        quadro,
        x: 0,
        y: 0,
        d: 6,
        espelhar: false,
    };
    conferir("teste-idle0-d6", &skin, sprite, 288, 288);
}

#[test]
fn espelhado_em_d6() {
    let skin = skin_teste();
    let quadro = skin.tag("wave").expect("tag wave").de + 1;
    let sprite = Elemento::Sprite {
        quadro,
        x: 0,
        y: 0,
        d: 6,
        espelhar: true,
    };
    conferir("teste-wave1-espelhado-d6", &skin, sprite, 288, 288);
}

#[test]
fn recortado_na_borda_em_d5() {
    let skin = skin_teste();
    let quadro = skin.tag("done_big").expect("tag done_big").de + 2;
    let sprite = Elemento::Sprite {
        quadro,
        x: -37,
        y: 101,
        d: 5,
        espelhar: false,
    };
    conferir("teste-done_big2-recortado-d5", &skin, sprite, 160, 240);
}

#[test]
fn cada_pixel_de_arte_e_um_bloco_uniforme() {
    let skin = skin_teste();
    let d = 6;
    let dados = renderizar(
        &skin,
        Elemento::Sprite {
            quadro: 0,
            x: 0,
            y: 0,
            d,
            espelhar: false,
        },
        288,
        288,
    );
    let pixel = |x: i32, y: i32| {
        let i = ((y * 288 + x) * 4) as usize;
        &dados[i..i + 4]
    };
    for by in 0..48 {
        for bx in 0..48 {
            let canto = pixel(bx * d, by * d);
            for dy in 0..d {
                for dx in 0..d {
                    assert_eq!(pixel(bx * d + dx, by * d + dy), canto, "bloco ({bx},{by})");
                }
            }
            assert_eq!(canto, skin.folha.bgra_em(bx, by), "bloco ({bx},{by})");
        }
    }
}
