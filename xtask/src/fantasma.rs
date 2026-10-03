//! `cargo xtask fantasma`: confere, em recortes do grim, que o pet não deixa
//! pixels velhos na tela.
//!
//! O `cargo xtask nitidez` só olha os pixels opacos do quadro esperado, então
//! não vê um rastro de quadro antigo nem o fantasma de um pet escondido: os
//! dois aparecem onde o pet deveria ser transparente. Aqui a referência é a
//! tela **sem o pet**:
//!
//! - `--base` (uma ou mais): recortes da área do pet com ele escondido. Só
//!   contam os pixels iguais (±tolerância) em todas as bases: o que mudou
//!   entre elas (cursor piscando, relógio) é ignorado;
//! - `--depois`: o recorte a conferir, da mesma área;
//! - `--esperado` (opcional): o quadro esperado com o pet na tela; aí só os
//!   pixels de alfa 0 dele são conferidos (ali tem de aparecer o fundo).
//!   Sem ele (pet escondido), a área inteira tem de voltar a ser o fundo.
//!
//! Passa se os pixels diferentes ficam abaixo do limite (padrão: 0,5% dos
//! conferidos, no mínimo 50): um fantasma de verdade é o pet inteiro, milhares
//! de pixels; o limite só absorve o que mudou no fundo durante o teste.

use std::path::Path;

use pet_core::skin::{Imagem, decodificar_png};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Relatorio {
    /// Pixels estáveis em todas as bases e conferidos.
    pub conferidos: usize,
    /// Pixels que mudaram entre as bases (ignorados).
    pub instaveis: usize,
    /// Conferidos que diferem do fundo.
    pub diferentes: usize,
    pub maior_desvio: u8,
}

impl Relatorio {
    pub fn limite(&self) -> usize {
        (self.conferidos / 200).max(50)
    }

    pub fn passou(&self) -> bool {
        self.conferidos > 0 && self.diferentes <= self.limite()
    }
}

fn rgb(img: &Imagem, i: usize) -> [u8; 3] {
    [img.rgba[i * 4], img.rgba[i * 4 + 1], img.rgba[i * 4 + 2]]
}

fn desvio(a: [u8; 3], b: [u8; 3]) -> u8 {
    (0..3).map(|k| a[k].abs_diff(b[k])).max().unwrap_or(0)
}

pub fn comparar(
    bases: &[Imagem],
    depois: &Imagem,
    esperado: Option<&Imagem>,
    tolerancia: u8,
) -> Result<Relatorio, String> {
    let Some(primeira) = bases.first() else {
        return Err("precisa de pelo menos uma --base".into());
    };
    let tamanho = (primeira.largura, primeira.altura);
    let todas = bases.iter().chain([depois]).chain(esperado);
    if let Some(outra) = todas.into_iter().find(|i| (i.largura, i.altura) != tamanho) {
        return Err(format!(
            "tamanhos diferentes: {}x{} e {}x{}",
            tamanho.0, tamanho.1, outra.largura, outra.altura
        ));
    }
    let mut r = Relatorio::default();
    for i in 0..(tamanho.0 * tamanho.1) as usize {
        if esperado.is_some_and(|e| e.rgba[i * 4 + 3] != 0) {
            continue;
        }
        let fundo = rgb(primeira, i);
        if bases[1..]
            .iter()
            .any(|b| desvio(rgb(b, i), fundo) > tolerancia)
        {
            r.instaveis += 1;
            continue;
        }
        r.conferidos += 1;
        let d = desvio(rgb(depois, i), fundo);
        r.maior_desvio = r.maior_desvio.max(d);
        if d > tolerancia {
            r.diferentes += 1;
        }
    }
    Ok(r)
}

fn ler_png(caminho: &Path) -> Result<Imagem, String> {
    let bytes = std::fs::read(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
    decodificar_png(&bytes).map_err(|e| format!("{}: {e}", caminho.display()))
}

/// Argumentos: `--base A.png [--base B.png …] --depois D.png
/// [--esperado E.png] [--tolerancia 2]`.
pub fn executar(args: &[String]) -> Result<bool, String> {
    let mut bases = Vec::new();
    let mut depois = None;
    let mut esperado = None;
    let mut tolerancia = 2u8;
    let mut i = 0;
    while i < args.len() {
        let valor = args
            .get(i + 1)
            .ok_or_else(|| format!("falta o valor de {}", args[i]))?;
        match args[i].as_str() {
            "--base" => bases.push(ler_png(Path::new(valor))?),
            "--depois" => depois = Some(ler_png(Path::new(valor))?),
            "--esperado" => esperado = Some(ler_png(Path::new(valor))?),
            "--tolerancia" => {
                tolerancia = valor
                    .parse()
                    .map_err(|_| format!("--tolerancia inválida: «{valor}»"))?
            }
            outro => return Err(format!("argumento desconhecido: {outro}")),
        }
        i += 2;
    }
    let depois = depois.ok_or("falta --depois")?;
    let r = comparar(&bases, &depois, esperado.as_ref(), tolerancia)?;
    println!(
        "fundo: {} pixels conferidos, {} diferentes (limite {}, maior desvio {}), {} instáveis ignorados",
        r.conferidos,
        r.diferentes,
        r.limite(),
        r.maior_desvio,
        r.instaveis
    );
    println!(
        "fantasma: {}",
        if r.passou() { "nenhum" } else { "ENCONTRADO" }
    );
    Ok(r.passou())
}

#[cfg(test)]
mod testes {
    use super::*;

    const L: i32 = 40;
    const A: i32 = 30;

    /// Fundo variado (como uma tela de verdade).
    fn fundo() -> Imagem {
        let rgba = (0..L * A)
            .flat_map(|i| {
                let (x, y) = (i % L, i / L);
                [(x * 6) as u8, (y * 8) as u8, ((x + y) * 3) as u8, 255]
            })
            .collect();
        Imagem::de_rgba(L, A, rgba)
    }

    /// `img` com o retângulo (x, y, w, h) pintado de `cor`.
    fn pintar(img: &Imagem, (x, y, w, h): (i32, i32, i32, i32), cor: [u8; 4]) -> Imagem {
        let mut rgba = img.rgba.clone();
        for j in y..y + h {
            for i in x..x + w {
                let k = ((j * L + i) * 4) as usize;
                rgba[k..k + 4].copy_from_slice(&cor);
            }
        }
        Imagem::de_rgba(L, A, rgba)
    }

    /// Esperado: o "pet" opaco em (10, 5, 20, 20), transparente no resto.
    fn esperado() -> Imagem {
        let vazio = Imagem::de_rgba(L, A, vec![0; (L * A * 4) as usize]);
        pintar(&vazio, (10, 5, 20, 20), [200, 50, 50, 255])
    }

    #[test]
    fn escondido_sem_rastro_passa() {
        let f = fundo();
        let r = comparar(&[f.clone(), f.clone()], &f, None, 2).unwrap();
        assert!(r.passou(), "{r:?}");
        assert_eq!(r.conferidos, (L * A) as usize);
    }

    #[test]
    fn fantasma_do_pet_escondido_e_achado() {
        let f = fundo();
        let fantasma = pintar(&f, (10, 5, 20, 20), [200, 50, 50, 255]);
        let r = comparar(std::slice::from_ref(&f), &fantasma, None, 2).unwrap();
        assert!(!r.passou(), "{r:?}");
        assert_eq!(r.diferentes, 400);
    }

    #[test]
    fn com_o_pet_na_tela_so_o_transparente_conta() {
        let f = fundo();
        let com_pet = pintar(&f, (10, 5, 20, 20), [200, 50, 50, 255]);
        let r = comparar(std::slice::from_ref(&f), &com_pet, Some(&esperado()), 2).unwrap();
        assert!(r.passou(), "{r:?}");
        assert_eq!(r.conferidos, (L * A - 400) as usize);
        // Um quadro velho deixado ao lado do pet (onde devia ser fundo).
        let rastro = pintar(&com_pet, (0, 0, 10, 10), [9, 9, 9, 255]);
        let r = comparar(std::slice::from_ref(&f), &rastro, Some(&esperado()), 2).unwrap();
        assert!(!r.passou(), "{r:?}");
        assert_eq!(r.diferentes, 100);
    }

    #[test]
    fn o_que_muda_entre_as_bases_e_ignorado() {
        let f = fundo();
        // Um cursor piscando no canto: muda entre as bases e no depois.
        let cursor = pintar(&f, (0, 0, 3, 8), [255, 255, 255, 255]);
        let r = comparar(&[f.clone(), cursor.clone()], &cursor, None, 2).unwrap();
        assert!(r.passou(), "{r:?}");
        assert_eq!(r.instaveis, 24);
        // Luz noturna mexendo ±2 não conta.
        let morno = pintar(&f, (5, 5, 1, 1), [32, 42, 32, 255]);
        let base = pintar(&f, (5, 5, 1, 1), [30, 40, 30, 255]);
        assert!(comparar(&[base], &morno, None, 2).unwrap().passou());
    }

    #[test]
    fn tamanhos_diferentes_dao_erro() {
        let f = fundo();
        let menor = Imagem::de_rgba(2, 2, vec![0; 16]);
        assert!(comparar(&[f], &menor, None, 2).is_err());
        assert!(comparar(&[], &fundo(), None, 2).is_err());
    }
}
