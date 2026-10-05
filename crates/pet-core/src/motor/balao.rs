//! O balão mínimo (M4, decisão 0052): um quadro de bordas arredondadas em
//! pixel art com o rabinho apontando para a cabeça do pet, o texto na fonte
//! monogram ([`crate::fonte`]) e nada mais. Some sozinho. O M6 acrescenta o
//! pop, a datilografia e as frases.
//!
//! Tudo em blocos inteiros: um pixel da fonte (e da borda) vale `dt` pixels
//! do dispositivo, metade do D do pet arredondada para cima (o texto fica
//! legível sem ficar do tamanho do bicho). O balão fica em cima do corpo,
//! preso dentro da área útil do palco; sem espaço em cima, embaixo (com o
//! rabinho virado para cima).

use crate::cena::Elemento;
use crate::cerebro::{EstadoSessao, ResumoSessao, TipoAviso};
use crate::fonte;
use crate::geometria::Ret;

/// Linhas no balão; o que passar vira "+ N".
pub const MAX_LINHAS: usize = 6;
/// Caracteres por linha (cortada com "…").
pub const MAX_CARACTERES: usize = 36;
/// Caracteres do nome do projeto numa linha da lista: até tanto, e menos
/// (até o mínimo) para o estado e o tempo caberem na linha.
pub const MAX_PROJETO: usize = 16;
pub const MIN_PROJETO: usize = 6;
/// Quanto o balão fica: uma base e um tanto por linha, com teto.
pub const BASE_MS: u64 = 5_000;
pub const POR_LINHA_MS: u64 = 1_000;
pub const TETO_MS: u64 = 12_000;

/// Creme do fundo, a tinta da borda e do texto (BGRA pré-multiplicado,
/// opacos).
const FUNDO: [u8; 4] = [0xE3, 0xF1, 0xF7, 0xFF];
const TINTA: [u8; 4] = [0x2A, 0x1B, 0x1D, 0xFF];
/// O vermelho do coração.
const VERMELHO: [u8; 4] = [0x4F, 0x3B, 0xE2, 0xFF];

/// O coração da risadinha (decisão 0057), em pixels da fonte: `#` tinta,
/// `r` vermelho, `w` o brilho creme.
const CORACAO: [&str; 6] = [
    ".##.##.", //
    "#wr#rr#", //
    "#rrrrr#", //
    ".#rrr#.", //
    "..#r#..", //
    "...#...", //
];

/// Folgas em pixels da fonte: dos lados, em cima e embaixo do texto (as
/// linhas da monogram já têm o espaço dos acentos em cima).
const FOLGA_LADO: i32 = 3;
const FOLGA_CIMA: i32 = 0;
const FOLGA_BAIXO: i32 = 1;
/// Altura de uma linha de texto: as 12 linhas do glifo menos uma que só os
/// descendentes mais fundos usam.
const ALTURA_LINHA: i32 = 11;
/// O rabinho: 2 linhas abaixo da borda.
const RABO: i32 = 2;
/// Distância entre a ponta do rabinho e o corpo, e até a borda da área.
const VAO: i32 = 1;

/// Um balão na tela até `ate_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balao {
    pub linhas: Vec<String>,
    pub ate_ms: u64,
}

impl Balao {
    /// O balão com as linhas cortadas e o prazo pelo tamanho.
    pub fn novo(linhas: Vec<String>, agora_ms: u64) -> Balao {
        let mut linhas: Vec<String> = linhas
            .into_iter()
            .map(|l| fonte::cortar(&l, MAX_CARACTERES))
            .collect();
        if linhas.len() > MAX_LINHAS {
            let resto = linhas.len() - (MAX_LINHAS - 1);
            linhas.truncate(MAX_LINHAS - 1);
            linhas.push(format!("+ {resto}"));
        }
        let duracao = (BASE_MS + POR_LINHA_MS * linhas.len() as u64).min(TETO_MS);
        Balao {
            linhas,
            ate_ms: agora_ms + duracao,
        }
    }
}

/// O tamanho de um pixel da fonte, em pixels do dispositivo: metade do D,
/// para cima.
pub fn dt(d: i32) -> i32 {
    ((d + 1) / 2).max(1)
}

/// Os elementos do balão com `linhas` sobre o `corpo` (no palco), preso na
/// `area`, com pixel de fonte `dt`.
pub fn elementos(linhas: &[String], corpo: Ret, area: Ret, dt: i32) -> Vec<Elemento> {
    let dt = dt.max(1);
    let texto_w = linhas.iter().map(|l| fonte::largura(l)).max().unwrap_or(0);
    let texto_h = linhas.len() as i32 * ALTURA_LINHA;
    // O quadro em pixels da fonte, com a borda de 1.
    let w = texto_w + 2 * FOLGA_LADO + 2;
    let h = texto_h + FOLGA_CIMA + FOLGA_BAIXO + 2;
    let total_h = h + RABO;
    let cx = corpo.x + corpo.w / 2;
    let x0 = (cx - w * dt / 2)
        .min(area.direita() - (w + VAO) * dt)
        .max(area.x + VAO * dt);
    let acima = corpo.y - (total_h + VAO) * dt >= area.y + VAO * dt;
    let y0 = if acima {
        corpo.y - (total_h + VAO) * dt
    } else {
        (corpo.baixo() + (RABO + VAO) * dt).min(area.baixo() - (h + VAO) * dt)
    };
    // O rabinho na direção do meio do corpo, sem encostar nos cantos.
    let tx = ((cx - x0) / dt - 1).clamp(2, (w - 5).max(2));
    let mut saida = Vec::new();
    let mut bloco = |x: i32, y: i32, bw: i32, bh: i32, cor: [u8; 4]| {
        if bw > 0 && bh > 0 {
            saida.push(Elemento::Bloco {
                ret: Ret::novo(x0 + x * dt, y0 + y * dt, bw * dt, bh * dt),
                cor,
            });
        }
    };
    // Fundo e borda, com os cantos de um pixel cortados.
    bloco(1, 1, w - 2, h - 2, FUNDO);
    bloco(0, 1, 1, h - 2, TINTA);
    bloco(w - 1, 1, 1, h - 2, TINTA);
    // A borda do lado do rabinho tem um vão de 3 por onde ele sai.
    let (lado_rabo, outro_lado) = if acima { (h - 1, 0) } else { (0, h - 1) };
    bloco(1, outro_lado, w - 2, 1, TINTA);
    bloco(1, lado_rabo, tx - 1, 1, TINTA);
    bloco(tx, lado_rabo, 3, 1, FUNDO);
    bloco(tx + 3, lado_rabo, w - tx - 4, 1, TINTA);
    // O rabinho: tinta, fundo, tinta; e a ponta.
    let (r1, r2) = if acima { (h, h + 1) } else { (-1, -2) };
    bloco(tx, r1, 1, 1, TINTA);
    bloco(tx + 1, r1, 1, 1, FUNDO);
    bloco(tx + 2, r1, 1, 1, TINTA);
    bloco(tx + 1, r2, 1, 1, TINTA);
    // O texto.
    for (i, linha) in linhas.iter().enumerate() {
        let y = y0 + (1 + FOLGA_CIMA + i as i32 * ALTURA_LINHA) * dt;
        for (j, c) in linha.chars().enumerate() {
            if c == ' ' {
                continue;
            }
            saida.push(Elemento::Glifo {
                c,
                x: x0 + (1 + FOLGA_LADO + j as i32 * fonte::AVANCO) * dt,
                y,
                d: dt,
                cor: TINTA,
            });
        }
    }
    saida
}

/// O selo "zZ" da soneca (decisão 0053): as duas letras da monogram em
/// creme com sombra de tinta (legível no tema escuro e no claro), no alto,
/// à direita do corpo, presas na área útil. Parado: nenhum commit a mais.
pub fn selo_zz(corpo: Ret, area: Ret, dt: i32) -> Vec<Elemento> {
    let dt = dt.max(1);
    let w = (2 * fonte::AVANCO) * dt;
    let h = fonte::ALTURA * dt;
    let x = (corpo.direita() - w / 2)
        .min(area.direita() - w - dt)
        .max(area.x);
    let y = (corpo.y - h + 2 * dt).max(area.y);
    let mut saida = Vec::with_capacity(4);
    for (i, c) in ['z', 'Z'].into_iter().enumerate() {
        // A segunda letra um pouco mais alta, como quem cochila.
        let cx = x + i as i32 * fonte::AVANCO * dt;
        let cy = y - i as i32 * 2 * dt;
        saida.push(Elemento::Glifo {
            c,
            x: cx + dt,
            y: cy + dt,
            d: dt,
            cor: TINTA,
        });
        saida.push(Elemento::Glifo {
            c,
            x: cx,
            y: cy,
            d: dt,
            cor: FUNDO,
        });
    }
    saida
}

/// O coração da risadinha do clique que leva ao terminal (decisão 0057): 7
/// por 6 pixels da fonte, no alto, à esquerda do corpo (o selo "zZ" fica à
/// direita), preso na área útil. Parado: um commit para aparecer e um para
/// sumir.
pub fn coracao(corpo: Ret, area: Ret, dt: i32) -> Vec<Elemento> {
    let dt = dt.max(1);
    let w = CORACAO[0].len() as i32 * dt;
    let h = CORACAO.len() as i32 * dt;
    // Metade para fora do corpo, na grade do corpo (múltiplos de `dt`).
    let x0 = (corpo.x - (CORACAO[0].len() as i32 / 2) * dt)
        .min(area.direita() - w)
        .max(area.x);
    let y0 = (corpo.y - h + 2 * dt).max(area.y);
    let mut saida = Vec::new();
    for (j, linha) in CORACAO.iter().enumerate() {
        let bytes = linha.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            let mut fim = i + 1;
            while fim < bytes.len() && bytes[fim] == c {
                fim += 1;
            }
            let cor = match c {
                b'#' => Some(TINTA),
                b'r' => Some(VERMELHO),
                b'w' => Some(FUNDO),
                _ => None,
            };
            if let Some(cor) = cor {
                saida.push(Elemento::Bloco {
                    ret: Ret::novo(
                        x0 + i as i32 * dt,
                        y0 + j as i32 * dt,
                        (fim - i) as i32 * dt,
                        dt,
                    ),
                    cor,
                });
            }
            i = fim;
        }
    }
    saida
}

/// Como uma sessão aparece na lista: o estado em palavras e desde quando.
/// Parada com o pronto pendente é "pronto", desde o Stop (decisão 0057).
pub fn estado(sessao: &ResumoSessao) -> (&'static str, u64) {
    if sessao.estado == EstadoSessao::Parada
        && let Some(aviso) = sessao.aviso
        && aviso.tipo == TipoAviso::Pronto
    {
        return ("pronto", aviso.desde_ms);
    }
    let palavra = match sessao.estado {
        EstadoSessao::Parada => "parado",
        EstadoSessao::Pensando => "pensando",
        EstadoSessao::Trabalhando => "trabalhando",
        EstadoSessao::Esperando => "esperando você",
        EstadoSessao::Compactando => "compactando",
        EstadoSessao::Erro => "erro",
        EstadoSessao::Cansado => "cansado",
    };
    (palavra, sessao.estado_desde_ms)
}

/// O começo do balão de um aviso que o clique não levou ao terminal
/// (decisão 0057): a sessão com o aviso e o porquê.
pub fn linhas_sem_foco(proj: Option<&str>, tipo: TipoAviso, motivo: &str) -> Vec<String> {
    let nome = fonte::cortar(proj.unwrap_or("sem pasta"), MAX_PROJETO);
    vec![format!("{nome}: {}", tipo.nome()), motivo.to_owned()]
}

/// Uma frase com o nome do projeto no fim, se há um ("Deu ruim... api").
pub fn com_projeto(frase: &str, proj: Option<&str>) -> String {
    match proj {
        Some(proj) => format!("{frase} {}", fonte::cortar(proj, MAX_PROJETO)),
        None => frase.to_owned(),
    }
}

/// "há quanto tempo", curto: 40 s, 3 min, 2 h.
pub fn duracao(ms: u64) -> String {
    let s = ms / 1000;
    match s {
        0..60 => format!("{s} s"),
        60..3_600 => format!("{} min", s / 60),
        _ => format!("{} h", s / 3_600),
    }
}

/// Uma linha da lista de sessões: "projeto: estado (tempo)". O nome do
/// projeto encolhe para o tempo caber na linha (o corte do balão é no fim).
pub fn linha_da_sessao(
    proj: Option<&str>,
    estado: &str,
    desde_ms: u64,
    agora_parede_ms: u64,
    teste: bool,
) -> String {
    let marca = if teste { " (teste)" } else { "" };
    let resto = format!(
        "{marca}: {estado} ({})",
        duracao(agora_parede_ms.saturating_sub(desde_ms))
    );
    let cabe = MAX_CARACTERES
        .saturating_sub(resto.chars().count())
        .clamp(MIN_PROJETO, MAX_PROJETO);
    format!(
        "{}{resto}",
        fonte::cortar(proj.unwrap_or("sem pasta"), cabe)
    )
}

#[cfg(test)]
mod testes {
    use super::*;

    fn area() -> Ret {
        Ret::novo(0, 0, 1920, 1200)
    }

    fn limites(elementos: &[Elemento]) -> Ret {
        let mut x0 = i32::MAX;
        let mut y0 = i32::MAX;
        let mut x1 = i32::MIN;
        let mut y1 = i32::MIN;
        for e in elementos {
            let r = match *e {
                Elemento::Bloco { ret, .. } => ret,
                Elemento::Glifo { x, y, d, .. } => {
                    Ret::novo(x, y, fonte::AVANCO * d, ALTURA_LINHA * d)
                }
                Elemento::Sprite { .. } => continue,
            };
            x0 = x0.min(r.x);
            y0 = y0.min(r.y);
            x1 = x1.max(r.direita());
            y1 = y1.max(r.baixo());
        }
        Ret::novo(x0, y0, x1 - x0, y1 - y0)
    }

    #[test]
    fn em_cima_do_corpo_em_blocos_inteiros_e_dentro_da_area() {
        let corpo = Ret::novo(1_782, 1_062, 114, 114);
        let linhas = vec!["claude-pet: pensando (3 min)".to_owned()];
        let e = elementos(&linhas, corpo, area(), 3);
        let caixa = limites(&e);
        assert!(caixa.baixo() <= corpo.y, "em cima do corpo: {caixa:?}");
        assert!(
            area().intersecao(&caixa) == Some(caixa),
            "dentro: {caixa:?}"
        );
        for el in &e {
            match *el {
                Elemento::Bloco { ret, .. } => {
                    assert_eq!((ret.w % 3, ret.h % 3), (0, 0), "blocos de 3");
                }
                Elemento::Glifo { d, .. } => assert_eq!(d, 3),
                Elemento::Sprite { .. } => panic!("sem sprite"),
            }
        }
        let letras = e
            .iter()
            .filter(|x| matches!(x, Elemento::Glifo { .. }))
            .count();
        assert_eq!(letras, linhas[0].chars().filter(|c| *c != ' ').count());
    }

    #[test]
    fn sem_espaco_em_cima_vai_para_baixo_e_o_rabinho_vira() {
        let corpo = Ret::novo(100, 10, 114, 114);
        let e = elementos(&["oi".to_owned()], corpo, area(), 3);
        let caixa = limites(&e);
        assert!(caixa.y >= corpo.baixo(), "embaixo: {caixa:?}");
        // Encostado na esquerda: preso dentro da área.
        let colado = Ret::novo(0, 900, 60, 60);
        let caixa = limites(&elementos(
            &["uma linha bem comprida".into()],
            colado,
            area(),
            3,
        ));
        assert!(caixa.x >= 0);
    }

    #[test]
    fn linhas_cortadas_mais_de_seis_viram_mais_n_e_o_prazo_tem_teto() {
        let muitas: Vec<String> = (0..9).map(|i| format!("sessão {i}")).collect();
        let b = Balao::novo(muitas, 1_000);
        assert_eq!(b.linhas.len(), MAX_LINHAS);
        assert_eq!(b.linhas.last().unwrap(), "+ 4");
        assert_eq!(b.ate_ms, 1_000 + (BASE_MS + 6 * POR_LINHA_MS).min(TETO_MS));
        let longa = Balao::novo(vec!["x".repeat(80)], 0);
        assert_eq!(longa.linhas[0].chars().count(), MAX_CARACTERES);
        assert!(longa.linhas[0].ends_with('…'));
        assert_eq!(dt(6), 3);
        assert_eq!(dt(5), 3);
        assert_eq!(dt(8), 4);
        assert_eq!(dt(1), 1);
    }

    #[test]
    fn selo_zz_no_alto_a_direita_e_dentro_da_area() {
        let corpo = Ret::novo(1_782, 1_062, 114, 114);
        let selo = selo_zz(corpo, area(), 3);
        assert_eq!(selo.len(), 4, "duas letras com sombra");
        for e in &selo {
            let Elemento::Glifo { x, y, d, .. } = *e else {
                panic!("só letras");
            };
            assert_eq!(d, 3);
            assert!(x + fonte::LARGURA_MAX * d <= 1920 + 3 && y >= 0);
            assert!(y < corpo.y, "em cima do corpo");
        }
        let canto = selo_zz(Ret::novo(1_850, 0, 70, 70), area(), 3);
        assert!(
            canto
                .iter()
                .all(|e| matches!(*e, Elemento::Glifo { y, .. } if y >= -6))
        );
    }

    #[test]
    fn coracao_em_blocos_no_alto_a_esquerda_e_dentro_da_area() {
        let corpo = Ret::novo(1_782, 1_062, 114, 114);
        let c = coracao(corpo, area(), 3);
        let caixa = limites(&c);
        assert_eq!((caixa.w, caixa.h), (21, 18), "7 por 6 pixels de 3");
        assert!(caixa.x < corpo.x && caixa.y < corpo.y, "no alto à esquerda");
        let vermelhos = c
            .iter()
            .filter(|e| matches!(e, Elemento::Bloco { cor, .. } if *cor == VERMELHO))
            .count();
        assert!(vermelhos >= 5, "uma corrida por linha com vermelho");
        for e in &c {
            let Elemento::Bloco { ret, cor } = *e else {
                panic!("só blocos");
            };
            assert_eq!((ret.x % 3, ret.w % 3, ret.h), (corpo.x % 3, 0, 3));
            assert_eq!(cor[3], 255, "opaco");
        }
        // Colado no canto de cima à esquerda: preso na área.
        let canto = limites(&coracao(Ret::novo(0, 0, 60, 60), area(), 3));
        assert!(canto.x >= 0 && canto.y >= 0);
    }

    #[test]
    fn linhas_sem_foco_e_o_pronto_na_lista() {
        assert_eq!(
            linhas_sem_foco(
                Some("claude-pet"),
                TipoAviso::Pronto,
                "a janela dela fechou"
            ),
            vec!["claude-pet: pronto", "a janela dela fechou"]
        );
        assert_eq!(
            linhas_sem_foco(None, TipoAviso::Esperando, "x")[0],
            "sem pasta: esperando você"
        );
    }

    #[test]
    fn linha_da_sessao_e_duracao() {
        assert_eq!(duracao(40_000), "40 s");
        assert_eq!(duracao(185_000), "3 min");
        assert_eq!(duracao(7_300_000), "2 h");
        assert_eq!(
            linha_da_sessao(Some("claude-pet"), "pensando", 1_000, 181_000, false),
            "claude-pet: pensando (3 min)"
        );
        assert_eq!(
            linha_da_sessao(Some("agenda-presidencial-2026"), "parado", 0, 5_000, true),
            "agenda-presid… (teste): parado (5 s)"
        );
        // O nome encolhe para o tempo caber nos 36 caracteres.
        let longa = linha_da_sessao(
            Some("agenda-presidencial-2026"),
            "esperando você",
            0,
            720_000,
            false,
        );
        assert_eq!(longa, "agenda-pre…: esperando você (12 min)");
        assert_eq!(longa.chars().count(), MAX_CARACTERES);
        assert_eq!(
            linha_da_sessao(None, "erro", 0, 0, false),
            "sem pasta: erro (0 s)"
        );
    }
}
