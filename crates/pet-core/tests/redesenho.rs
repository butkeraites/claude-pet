//! O desenho incremental (só as regiões com dano) tem de dar exatamente o
//! mesmo buffer que redesenhar a tela inteira, quadro a quadro, nas
//! sequências que o daemon produz de verdade: o repouso da skin `_teste`,
//! confete (inclusive acima de 128 retângulos, quando o dano vira
//! ladrilhos), o quadro vazio de esconder e a volta do pet. Um erro aqui
//! deixaria pixels velhos na tela (rastro ou fantasma).

use std::path::Path;

use pet_core::animador::Repouso;
use pet_core::cena::{self, Elemento};
use pet_core::confete::{Chuva, Grade};
use pet_core::geometria::Ret;
use pet_core::raster::{self, Alvo};
use pet_core::skin::Skin;

const LARGURA: i32 = 640;
const ALTURA: i32 = 400;
const D: i32 = 5;
/// Célula de 240x240 que passa das bordas direita e de baixo (recorte).
const CELULA: (i32, i32) = (LARGURA - 220, ALTURA - 230);

fn skin_teste() -> Skin {
    Skin::carregar(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skins/_teste"))
        .expect("skins/_teste (cargo xtask skin-teste)")
}

fn sprite(quadro: usize) -> Elemento {
    Elemento::Sprite {
        quadro,
        x: CELULA.0,
        y: CELULA.1,
        d: D,
        espelhar: false,
    }
}

fn do_zero(cena: &[Elemento], skin: &Skin) -> Vec<u8> {
    let mut dados = vec![0u8; (LARGURA * ALTURA * 4) as usize];
    let mut alvo = Alvo::novo(&mut dados, LARGURA, ALTURA);
    let tudo = alvo.limites();
    cena::redesenhar(&mut alvo, cena, skin, &[tudo]);
    dados
}

/// Aplica as cenas em ordem num buffer só, como o daemon, e confere cada
/// quadro contra o desenho do zero. Devolve quantos quadros usaram ladrilhos.
fn conferir(cenas: &[Vec<Elemento>], skin: &Skin) -> usize {
    let tela = Ret::novo(0, 0, LARGURA, ALTURA);
    let mut buffer = vec![0u8; (LARGURA * ALTURA * 4) as usize];
    let mut anterior: Option<&[Elemento]> = None;
    let mut com_ladrilhos = 0;
    for (n, cena) in cenas.iter().enumerate() {
        let regioes = match anterior {
            None => vec![tela],
            Some(antes) => {
                let brutos = cena::danos(antes, cena, skin);
                let regioes = raster::consolidar_danos(&brutos, tela);
                if brutos.len() > raster::MAX_RETANGULOS {
                    com_ladrilhos += 1;
                }
                regioes
            }
        };
        {
            let mut alvo = Alvo::novo(&mut buffer, LARGURA, ALTURA);
            cena::redesenhar(&mut alvo, cena, skin, &regioes);
        }
        assert!(
            buffer == do_zero(cena, skin),
            "quadro {n}: o incremental diverge do desenho do zero"
        );
        anterior = Some(cena);
    }
    com_ladrilhos
}

#[test]
fn repouso_confete_esconder_e_voltar() {
    let skin = skin_teste();
    let mut cenas: Vec<Vec<Elemento>> = Vec::new();

    // Dois períodos do repouso, seguindo os prazos que o animador devolve.
    let repouso = Repouso::novo(&skin, 0);
    let (mut quadro, mut t) = repouso.em(0);
    cenas.push(vec![sprite(quadro)]);
    while t < 40_000 {
        let (q, proxima) = repouso.em(t);
        if q != quadro {
            cenas.push(vec![sprite(q)]);
            quadro = q;
        }
        t = proxima;
    }
    assert!(cenas.len() > 6, "o repouso trocou de quadro");

    // Confete por cima do pet: 40 pedaços (lista) e 100 (ladrilhos).
    let grade = Grade {
        x: CELULA.0,
        y: CELULA.1,
        d: D,
    };
    for (quantos, semente) in [(40, 7), (100, 11)] {
        let mut chuva = Chuva::nova(quantos, (LARGURA, ALTURA), grade, 3, semente);
        for _ in 0..30 {
            let mut cena = vec![sprite(quadro)];
            cena.extend(chuva.elementos());
            cenas.push(cena);
            chuva.passo();
        }
    }

    // Esconder (quadro vazio) e voltar.
    cenas.push(Vec::new());
    cenas.push(vec![sprite(quadro)]);

    let com_ladrilhos = conferir(&cenas, &skin);
    assert!(com_ladrilhos > 0, "nenhum quadro passou pelos ladrilhos");
}

#[test]
fn quadro_vazio_limpa_tudo_que_o_pet_pintou() {
    let skin = skin_teste();
    let cheio = vec![sprite(0)];
    let vazio: Vec<Elemento> = Vec::new();
    conferir(&[cheio, vazio.clone()], &skin);
    assert!(
        do_zero(&vazio, &skin).iter().all(|&b| b == 0),
        "esconder deixa o buffer transparente"
    );
}
