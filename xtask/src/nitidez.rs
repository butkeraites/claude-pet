//! `cargo xtask nitidez`: confere, numa captura do grim, que o pet está
//! nítido (portão do M1).
//!
//! Entradas: a captura do **monitor inteiro** (`grim -o`, transformação
//! identidade: pixel do monitor = pixel da imagem), o quadro esperado do
//! `/v1/debug/quadro` (RGBA em pixels do monitor), onde ele fica na captura
//! e a grade de blocos D×D (o canto da célula do sprite).
//!
//! Duas checagens, só nos pixels de alfa 255 do esperado (os transparentes
//! mostram o que estiver atrás do pet):
//! 1. **cor**: cada canal dentro de ±2 do esperado (folga para gestão de
//!    cor ou luz noturna);
//! 2. **blocos**: todo bloco D×D inteiramente opaco no esperado é uniforme
//!    na captura (máximo − mínimo ≤ 2 por canal). Um filtro bilinear ou meio
//!    pixel de deslocamento quebram isto mesmo com as cores certas.

use std::path::Path;

use pet_core::skin::{Imagem, decodificar_png};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parametros {
    pub x: i32,
    pub y: i32,
    pub d: i32,
    /// Origem da grade de blocos, em coordenadas da captura.
    pub grade: (i32, i32),
    pub tolerancia: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Relatorio {
    pub pixels_opacos: usize,
    pub pixels_fora: usize,
    pub maior_desvio: u8,
    /// Soma dos desvios com sinal por canal (R, G, B), para ver um viés.
    pub soma_desvio: [i64; 3],
    pub blocos: usize,
    pub blocos_irregulares: usize,
}

impl Relatorio {
    pub fn passou(&self) -> bool {
        self.pixels_opacos > 0
            && self.pixels_fora == 0
            && self.blocos > 0
            && self.blocos_irregulares == 0
    }

    pub fn resumo(&self, p: &Parametros) -> String {
        let vies: Vec<String> = self
            .soma_desvio
            .iter()
            .map(|s| format!("{:+.1}", *s as f64 / self.pixels_opacos.max(1) as f64))
            .collect();
        format!(
            "cor: {} pixels opacos, {} fora de ±{} (maior desvio {}, viés médio RGB {})\n\
             blocos {}×{}: {} conferidos, {} não uniformes",
            self.pixels_opacos,
            self.pixels_fora,
            p.tolerancia,
            self.maior_desvio,
            vies.join("/"),
            p.d,
            p.d,
            self.blocos,
            self.blocos_irregulares
        )
    }
}

fn rgba(img: &Imagem, x: i32, y: i32) -> [u8; 4] {
    let i = ((y * img.largura + x) * 4) as usize;
    [
        img.rgba[i],
        img.rgba[i + 1],
        img.rgba[i + 2],
        img.rgba[i + 3],
    ]
}

pub fn comparar(captura: &Imagem, esperado: &Imagem, p: &Parametros) -> Result<Relatorio, String> {
    if p.d < 1 {
        return Err("D precisa ser pelo menos 1".into());
    }
    let cabe = p.x >= 0
        && p.y >= 0
        && p.x + esperado.largura <= captura.largura
        && p.y + esperado.altura <= captura.altura;
    if !cabe {
        return Err(format!(
            "o quadro {}x{} em ({}, {}) não cabe na captura {}x{}",
            esperado.largura, esperado.altura, p.x, p.y, captura.largura, captura.altura
        ));
    }
    let mut r = Relatorio::default();
    for j in 0..esperado.altura {
        for i in 0..esperado.largura {
            let e = rgba(esperado, i, j);
            if e[3] != 255 {
                continue;
            }
            let c = rgba(captura, p.x + i, p.y + j);
            r.pixels_opacos += 1;
            let mut fora = false;
            for k in 0..3 {
                let desvio = c[k] as i64 - e[k] as i64;
                r.soma_desvio[k] += desvio;
                let absoluto = desvio.unsigned_abs().min(255) as u8;
                r.maior_desvio = r.maior_desvio.max(absoluto);
                fora |= absoluto > p.tolerancia;
            }
            if fora {
                r.pixels_fora += 1;
            }
        }
    }
    // Blocos da grade que caem inteiros dentro do esperado.
    let local = (p.grade.0 - p.x, p.grade.1 - p.y);
    let primeiro = |origem: i32| origem.rem_euclid(p.d);
    let mut by = primeiro(local.1);
    while by + p.d <= esperado.altura {
        let mut bx = primeiro(local.0);
        while bx + p.d <= esperado.largura {
            let opaco =
                (0..p.d).all(|dy| (0..p.d).all(|dx| rgba(esperado, bx + dx, by + dy)[3] == 255));
            if opaco {
                r.blocos += 1;
                let mut min = [255u8; 3];
                let mut max = [0u8; 3];
                for dy in 0..p.d {
                    for dx in 0..p.d {
                        let c = rgba(captura, p.x + bx + dx, p.y + by + dy);
                        for k in 0..3 {
                            min[k] = min[k].min(c[k]);
                            max[k] = max[k].max(c[k]);
                        }
                    }
                }
                if (0..3).any(|k| max[k] - min[k] > p.tolerancia) {
                    r.blocos_irregulares += 1;
                }
            }
            bx += p.d;
        }
        by += p.d;
    }
    Ok(r)
}

fn ler_png(caminho: &Path) -> Result<Imagem, String> {
    let bytes = std::fs::read(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
    decodificar_png(&bytes).map_err(|e| format!("{}: {e}", caminho.display()))
}

fn numero(texto: &str, nome: &str) -> Result<i32, String> {
    texto
        .trim()
        .parse()
        .map_err(|_| format!("{nome} inválido: «{texto}»"))
}

/// Argumentos: `--captura A.png --esperado B.png --x X --y Y --d D
/// [--grade GX,GY] [--tolerancia 2]`.
pub fn executar(args: &[String]) -> Result<bool, String> {
    let valor = |nome: &str| -> Option<&str> {
        args.iter()
            .position(|a| a == nome)
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
    };
    let obrigatorio = |nome: &str| valor(nome).ok_or_else(|| format!("falta {nome}"));
    let captura = ler_png(Path::new(obrigatorio("--captura")?))?;
    let esperado = ler_png(Path::new(obrigatorio("--esperado")?))?;
    let x = numero(obrigatorio("--x")?, "--x")?;
    let y = numero(obrigatorio("--y")?, "--y")?;
    let d = numero(obrigatorio("--d")?, "--d")?;
    let grade = match valor("--grade") {
        Some(texto) => {
            let (gx, gy) = texto
                .split_once(',')
                .ok_or_else(|| format!("--grade inválida: «{texto}» (use X,Y)"))?;
            (numero(gx, "--grade")?, numero(gy, "--grade")?)
        }
        None => (x, y),
    };
    let tolerancia = match valor("--tolerancia") {
        Some(t) => t
            .parse()
            .map_err(|_| format!("--tolerancia inválida: «{t}»"))?,
        None => 2,
    };
    let p = Parametros {
        x,
        y,
        d,
        grade,
        tolerancia,
    };
    let r = comparar(&captura, &esperado, &p)?;
    println!("{}", r.resumo(&p));
    if r.passou() {
        println!("nitidez: OK");
    } else {
        println!("nitidez: FALHOU");
    }
    Ok(r.passou())
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Esperado 12x12 com D=3: blocos de cores distintas, um canto
    /// transparente e um pixel meio transparente.
    fn esperado() -> Imagem {
        let mut rgba = Vec::new();
        for y in 0..12 {
            for x in 0..12 {
                let (bx, by) = (x / 3, y / 3);
                let alfa = match (bx, by) {
                    (0, 0) => 0,
                    (3, 3) if x == 11 && y == 11 => 128,
                    _ => 255,
                };
                rgba.extend_from_slice(&[(bx * 60) as u8, (by * 60) as u8, 100, alfa]);
            }
        }
        Imagem::de_rgba(12, 12, rgba)
    }

    /// Captura 40x30 com fundo variado e o esperado colado em (ox, oy);
    /// `ajuste` mexe em cada pixel colado.
    fn captura(
        e: &Imagem,
        ox: i32,
        oy: i32,
        ajuste: impl Fn(i32, i32, [u8; 4]) -> [u8; 4],
    ) -> Imagem {
        let mut rgba = Vec::new();
        for y in 0..30 {
            for x in 0..40 {
                let dentro = x >= ox && y >= oy && x < ox + 12 && y < oy + 12;
                let px = if dentro && rgba_ok(e, x - ox, y - oy) {
                    ajuste(x - ox, y - oy, super::rgba(e, x - ox, y - oy))
                } else {
                    [(x * 7) as u8, (y * 11) as u8, ((x + y) * 5) as u8, 255]
                };
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        Imagem::de_rgba(40, 30, rgba)
    }

    fn rgba_ok(e: &Imagem, x: i32, y: i32) -> bool {
        super::rgba(e, x, y)[3] == 255
    }

    fn params(x: i32, y: i32) -> Parametros {
        Parametros {
            x,
            y,
            d: 3,
            grade: (x, y),
            tolerancia: 2,
        }
    }

    #[test]
    fn captura_exata_passa() {
        let e = esperado();
        let c = captura(&e, 7, 5, |_, _, p| p);
        let r = comparar(&c, &e, &params(7, 5)).unwrap();
        assert!(r.passou(), "{r:?}");
        assert_eq!(
            r.blocos,
            16 - 2,
            "sem o bloco transparente e o meio-transparente"
        );
    }

    #[test]
    fn deslocada_um_pixel_falha() {
        let e = esperado();
        let c = captura(&e, 8, 5, |_, _, p| p);
        let r = comparar(&c, &e, &params(7, 5)).unwrap();
        assert!(!r.passou());
        assert!(r.blocos_irregulares > 0);
    }

    #[test]
    fn cor_desviada_falha_mas_blocos_seguem_uniformes() {
        let e = esperado();
        let c = captura(&e, 7, 5, |_, _, p| {
            [p[0], p[1], p[2].saturating_sub(9), p[3]]
        });
        let r = comparar(&c, &e, &params(7, 5)).unwrap();
        assert!(!r.passou());
        assert_eq!(r.blocos_irregulares, 0);
        assert_eq!(r.maior_desvio, 9);
        assert!(r.soma_desvio[2] < 0, "viés no azul");
    }

    #[test]
    fn borrado_falha_nos_blocos() {
        let e = esperado();
        // Bilinear de meio pixel: cada pixel vira a média com o vizinho.
        let c = captura(&e, 7, 5, |x, y, p| {
            let vizinho = if x + 1 < 12 && rgba_ok(&e, x + 1, y) {
                super::rgba(&e, x + 1, y)
            } else {
                p
            };
            [
                ((p[0] as u16 + vizinho[0] as u16) / 2) as u8,
                ((p[1] as u16 + vizinho[1] as u16) / 2) as u8,
                p[2],
                255,
            ]
        });
        let r = comparar(&c, &e, &params(7, 5)).unwrap();
        assert!(r.blocos_irregulares > 0);
        assert!(!r.passou());
    }

    /// Ponta a ponta com o quadro de verdade: a pose da skin `_teste` em
    /// D=5, do mesmo jeito que o /v1/debug/quadro a entrega, composta sobre
    /// uma "tela" de fundo variado como o compositor faria. Passa no lugar
    /// certo; falha deslocada 1 pixel e redimensionada 0,5% (o filtro
    /// bilinear de um `grim -g`).
    #[test]
    fn quadro_real_da_skin_de_teste_ponta_a_ponta() {
        use pet_core::cena::{Elemento, rgba_do_sprite};
        use pet_core::geometria::Ret;
        use pet_core::skin::Skin;

        const D: i32 = 5;
        const CELULA: (i32, i32) = (123, 77);
        const TELA: (i32, i32) = (400, 330);
        let skin = Skin::carregar(&crate::raiz().join("skins/_teste")).unwrap();
        let pose = skin.tags[skin.tags_do_estado("idle")[0]].de;
        let sprite = Elemento::Sprite {
            quadro: pose,
            x: CELULA.0,
            y: CELULA.1,
            d: D,
            espelhar: false,
        };
        let area = Ret::novo(CELULA.0, CELULA.1, 48 * D, 48 * D);
        let esperado = Imagem::de_rgba(area.w, area.h, rgba_do_sprite(&skin, &sprite, area));

        // A tela com o pet composto ("over", alfa direto) em CELULA + desvio.
        let tela = |dx: i32, dy: i32| {
            let mut rgba = Vec::with_capacity((TELA.0 * TELA.1 * 4) as usize);
            for y in 0..TELA.1 {
                for x in 0..TELA.0 {
                    let fundo = [(x * 3) as u8, (y * 5) as u8, ((x ^ y) & 255) as u8];
                    let (ex, ey) = (x - CELULA.0 - dx, y - CELULA.1 - dy);
                    let e = if (0..area.w).contains(&ex) && (0..area.h).contains(&ey) {
                        super::rgba(&esperado, ex, ey)
                    } else {
                        [0; 4]
                    };
                    let a = e[3] as u32;
                    for k in 0..3 {
                        rgba.push(((e[k] as u32 * a + fundo[k] as u32 * (255 - a)) / 255) as u8);
                    }
                    rgba.push(255);
                }
            }
            Imagem::de_rgba(TELA.0, TELA.1, rgba)
        };
        // Bilinear em volta da origem, como uma escala de 1.005.
        let redimensionada = |img: &Imagem, s: f64| {
            let mut rgba = Vec::with_capacity(img.rgba.len());
            for y in 0..img.altura {
                for x in 0..img.largura {
                    let (fx, fy) = (x as f64 / s, y as f64 / s);
                    let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
                    let (tx, ty) = (fx - x0 as f64, fy - y0 as f64);
                    let px = |i: i32, j: i32| {
                        super::rgba(img, i.min(img.largura - 1), j.min(img.altura - 1))
                    };
                    for k in 0..3 {
                        let v = px(x0, y0)[k] as f64 * (1.0 - tx) * (1.0 - ty)
                            + px(x0 + 1, y0)[k] as f64 * tx * (1.0 - ty)
                            + px(x0, y0 + 1)[k] as f64 * (1.0 - tx) * ty
                            + px(x0 + 1, y0 + 1)[k] as f64 * tx * ty;
                        rgba.push(v.round() as u8);
                    }
                    rgba.push(255);
                }
            }
            Imagem::de_rgba(img.largura, img.altura, rgba)
        };

        let p = Parametros {
            x: CELULA.0,
            y: CELULA.1,
            d: D,
            grade: CELULA,
            tolerancia: 2,
        };
        let certa = comparar(&tela(0, 0), &esperado, &p).unwrap();
        assert!(certa.passou(), "{certa:?}");
        assert!(
            certa.pixels_opacos > 20_000 && certa.blocos > 800,
            "{certa:?}"
        );
        assert_eq!(certa.maior_desvio, 0);
        let deslocada = comparar(&tela(1, 0), &esperado, &p).unwrap();
        assert!(!deslocada.passou() && deslocada.blocos_irregulares > 100);
        let borrada = comparar(&redimensionada(&tela(0, 0), 1.005), &esperado, &p).unwrap();
        assert!(!borrada.passou() && borrada.blocos_irregulares > 100);
    }

    #[test]
    fn grade_com_fase_e_area_fora_da_captura() {
        let e = esperado();
        let c = captura(&e, 7, 5, |_, _, p| p);
        // A grade começa 1 px antes: nenhum bloco inteiro cai alinhado com as
        // cores, e blocos atravessam fronteiras de cor.
        let mut p = params(7, 5);
        p.grade = (6, 5);
        let r = comparar(&c, &e, &p).unwrap();
        assert!(r.blocos_irregulares > 0);
        assert!(comparar(&c, &e, &params(35, 5)).is_err());
    }
}
