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
use crate::cerebro::{EstadoSessao, ResumoSessao, TipoAviso, TipoEspera};
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
/// Datilografia (M6): quanto cada caractere demora a aparecer. O texto vai
/// surgindo da esquerda para a direita, linha a linha; a moldura já vem inteira
/// (decisão 0121). Na grade de 34 ms do desenho sai ~2 caracteres por quadro.
pub const CHAR_MS: u64 = 16;
/// Pop de entrada (M6, decisão 0122): o balão cresce do rabinho até o tamanho
/// cheio nesse tempo (dois quadros da grade de 34 ms), e só então datilografa.
pub const POP_MS: u64 = 68;

/// Creme do fundo, a tinta da borda e do texto (BGRA pré-multiplicado,
/// opacos).
const FUNDO: [u8; 4] = [0xE3, 0xF1, 0xF7, 0xFF];
const TINTA: [u8; 4] = [0x2A, 0x1B, 0x1D, 0xFF];
/// O brilho interno do topo da moldura 9-slice (decisão 0124): um creme quase
/// branco, dando um ar de adesivo. BGRA pré-multiplicado, opaco.
const BRILHO: [u8; 4] = [0xF4, 0xFB, 0xFE, 0xFF];
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

/// Um balão na tela de `desde_ms` até `ate_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balao {
    pub linhas: Vec<String>,
    pub desde_ms: u64,
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
            desde_ms: agora_ms,
            ate_ms: agora_ms + duracao,
        }
    }

    /// Quantos caracteres cabem no total (contando os espaços, para o ritmo da
    /// datilografia ficar parelho).
    pub fn total_de_caracteres(&self) -> usize {
        self.linhas.iter().map(|l| l.chars().count()).sum()
    }

    /// Quantos caracteres revelar agora na datilografia: `Some(n)` enquanto
    /// digita, `None` quando já apareceu tudo (decisão 0121). A datilografia só
    /// começa depois do pop de entrada (decisão 0122): durante o pop, `Some(0)`.
    pub fn revelar_em(&self, agora_ms: u64) -> Option<usize> {
        let apos_pop = agora_ms
            .saturating_sub(self.desde_ms)
            .saturating_sub(POP_MS);
        let passados = (apos_pop / CHAR_MS) as usize;
        (passados < self.total_de_caracteres()).then_some(passados)
    }

    /// A escala do pop de entrada (decisão 0122): `Some((n, d))` com `n/d < 1`
    /// enquanto o balão cresce do rabinho, `None` quando já está no tamanho
    /// cheio. Em dois passos na grade de 34 ms, para o pop caber no orçamento.
    pub fn escala_em(&self, agora_ms: u64) -> Option<(i32, i32)> {
        match agora_ms.saturating_sub(self.desde_ms) {
            0..34 => Some((60, 100)),
            34..POP_MS => Some((84, 100)),
            _ => None,
        }
    }
}

/// O tamanho de um pixel da fonte, em pixels do dispositivo: metade do D,
/// para cima.
pub fn dt(d: i32) -> i32 {
    ((d + 1) / 2).max(1)
}

/// Os elementos do balão com `linhas` sobre o `corpo` (no palco), preso na
/// `area`, com pixel de fonte `dt`. `revelar` é a datilografia: `Some(n)` mostra
/// só os `n` primeiros caracteres (contando os espaços, em ordem de leitura);
/// `None` mostra tudo (decisão 0121). `escala` é o pop de entrada: `Some((n, d))`
/// encolhe a moldura para `n/d` do tamanho, crescendo a partir do rabinho (sem
/// texto, pois a datilografia só começa depois do pop); `None` é o tamanho cheio
/// (decisão 0122).
pub fn elementos(
    linhas: &[String],
    corpo: Ret,
    area: Ret,
    dt: i32,
    revelar: Option<usize>,
    escala: Option<(i32, i32)>,
) -> Vec<Elemento> {
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
    // Moldura 9-slice (decisão 0124): fundo arredondado com cantos de 2px, um
    // brilho de adesivo no topo interno, e a borda com os cantos diagonais.
    let (lado_rabo, outro_lado) = if acima { (h - 1, 0) } else { (0, h - 1) };
    // Fundo: o brilho no topo interno, o miolo e a base interna (cantos de fora).
    bloco(2, 1, w - 4, 1, BRILHO);
    bloco(1, 2, w - 2, h - 4, FUNDO);
    bloco(2, h - 2, w - 4, 1, FUNDO);
    // Bordas laterais (sem os cantos).
    bloco(0, 2, 1, h - 4, TINTA);
    bloco(w - 1, 2, 1, h - 4, TINTA);
    // Os quatro cantos diagonais arredondam em 2px.
    bloco(1, 1, 1, 1, TINTA);
    bloco(w - 2, 1, 1, 1, TINTA);
    bloco(1, h - 2, 1, 1, TINTA);
    bloco(w - 2, h - 2, 1, 1, TINTA);
    // Borda de cima e de baixo (entre os cantos); a do rabinho tem um vão de 3.
    bloco(2, outro_lado, w - 4, 1, TINTA);
    bloco(2, lado_rabo, tx - 2, 1, TINTA);
    bloco(tx, lado_rabo, 3, 1, FUNDO);
    bloco(tx + 3, lado_rabo, w - 5 - tx, 1, TINTA);
    // O rabinho: tinta, fundo, tinta; e a ponta.
    let (r1, r2) = if acima { (h, h + 1) } else { (-1, -2) };
    bloco(tx, r1, 1, 1, TINTA);
    bloco(tx + 1, r1, 1, 1, FUNDO);
    bloco(tx + 2, r1, 1, 1, TINTA);
    bloco(tx + 1, r2, 1, 1, TINTA);
    // O texto, revelado da esquerda para a direita (a datilografia conta cada
    // caractere, inclusive os espaços, para o ritmo ficar parelho).
    let mut idx = 0usize;
    for (i, linha) in linhas.iter().enumerate() {
        let y = y0 + (1 + FOLGA_CIMA + i as i32 * ALTURA_LINHA) * dt;
        for (j, c) in linha.chars().enumerate() {
            let mostrar = revelar.is_none_or(|n| idx < n);
            idx += 1;
            if c == ' ' || !mostrar {
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
    // O pop de entrada (decisão 0122): encolhe a moldura para `n/d`, crescendo a
    // partir da ponta do rabinho (do lado do corpo), como se saísse do Zeca. Só
    // os blocos — durante o pop não há texto. É um surto transitório, então os
    // blocos podem sair da grade D×D por um átimo (como o confete e o voo).
    if let Some((n, d)) = escala.filter(|(n, d)| n < d && *d > 0) {
        let ax = x0 + (tx + 1) * dt + dt / 2;
        let ay = if acima { y0 + (h + 2) * dt } else { y0 - dt };
        for e in saida.iter_mut() {
            if let Elemento::Bloco { ret, .. } = e {
                ret.x = ax + (ret.x - ax) * n / d;
                ret.y = ay + (ret.y - ay) * n / d;
                ret.w = (ret.w * n / d).max(1);
                ret.h = (ret.h * n / d).max(1);
            }
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

/// Quantas variantes de fala cada banco de malandro tem (decisão 0123).
pub const VARIANTES_DE_FALA: usize = 3;

/// Índice de variante estável para uma fala, a partir de uma marca dela (o
/// `desde_ms` do aviso, o `estado_desde_ms` da sessão): a mesma fala mostra
/// sempre a mesma variante, falas diferentes variam (decisão 0123). Um
/// embaralhado leve distribui marcas próximas.
pub fn variante(marca: u64) -> usize {
    ((marca ^ (marca >> 7)).wrapping_mul(0x9E37_79B9) >> 24) as usize % VARIANTES_DE_FALA
}

/// O balão da chamada de um aviso de espera (decisão 0075), pelo tipo, numa das
/// falas de malandro do banco (`variante`, decisão 0123). Sem `proj` (sem pasta,
/// ou a tela compartilhada), a frase sem o nome.
pub fn linhas_da_espera(
    espera: Option<TipoEspera>,
    proj: Option<&str>,
    variante: usize,
) -> Vec<String> {
    let v = variante % VARIANTES_DE_FALA;
    let nome = proj.map(|p| fonte::cortar(p, MAX_PROJETO));
    let com_nome = |sem: &str, com: &str| match &nome {
        Some(nome) => format!("{nome}{com}"),
        None => sem.to_owned(),
    };
    match espera {
        Some(TipoEspera::Pergunta) => vec![match v {
            0 => com_nome("Pergunta pra você", ": pergunta pra você"),
            1 => com_nome("Tem pergunta pra você", ": tem pergunta!"),
            _ => com_nome("Querem saber de você", ": querem saber"),
        }],
        Some(TipoEspera::Plano) => vec![com_projeto(
            match v {
                0 => "Plano pra aprovar!",
                1 => "Aprova o plano?",
                _ => "Plano na mesa!",
            },
            proj,
        )],
        Some(TipoEspera::Permissao | TipoEspera::Elicitacao) | None => {
            let saudacao = match v {
                0 => "Ô, meu camarada!",
                1 => "Psiu, chefia!",
                _ => "Bora, parceiro!",
            };
            let corpo = match v {
                1 => com_nome("tão te chamando", " tá te chamando"),
                _ => com_nome("precisa de você", " precisa de você"),
            };
            vec![saudacao.to_owned(), corpo]
        }
    }
}

/// O balão do erro, numa das falas de malandro do banco (`variante`, decisão
/// 0123), com o nome do projeto no fim se há um.
pub fn linhas_de_erro(proj: Option<&str>, variante: usize) -> String {
    let frase = match variante % VARIANTES_DE_FALA {
        0 => "Deu ruim...",
        1 => "Xi, deu ruim...",
        _ => "Complicou aqui...",
    };
    com_projeto(frase, proj)
}

/// "há quanto tempo", curto: 40 s, 3 min, 2 h, 2 d. Uma sessão sem evento
/// fica até uma semana na lista (decisão 0096): de um dia em diante, em dias
/// (decisão 0099).
pub fn duracao(ms: u64) -> String {
    let s = ms / 1000;
    match s {
        0..60 => format!("{s} s"),
        60..3_600 => format!("{} min", s / 60),
        3_600..86_400 => format!("{} h", s / 3_600),
        _ => format!("{} d", s / 86_400),
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
    fn moldura_9slice_arredonda_os_cantos_e_tem_brilho() {
        let corpo = Ret::novo(900, 700, 114, 114);
        let e = elementos(&["ô, camarada".to_owned()], corpo, area(), 3, None, None);
        let caixa = limites(&e);
        let cobre = |px: i32, py: i32| {
            e.iter().any(|el| {
                matches!(*el, Elemento::Bloco { ret, .. }
                    if ret.x <= px && px < ret.direita() && ret.y <= py && py < ret.baixo())
            })
        };
        // Os quatro cantos de fora ficam vazios (arredondados em 2px).
        assert!(!cobre(caixa.x, caixa.y), "canto superior esquerdo redondo");
        assert!(
            !cobre(caixa.direita() - 1, caixa.y),
            "canto superior direito redondo"
        );
        assert!(
            !cobre(caixa.x, caixa.baixo() - 1),
            "canto inferior esquerdo redondo"
        );
        // Tem o brilho de adesivo no topo.
        assert!(
            e.iter()
                .any(|el| matches!(el, Elemento::Bloco { cor, .. } if *cor == BRILHO)),
            "o brilho do topo"
        );
    }

    #[test]
    fn em_cima_do_corpo_em_blocos_inteiros_e_dentro_da_area() {
        let corpo = Ret::novo(1_782, 1_062, 114, 114);
        let linhas = vec!["claude-pet: pensando (3 min)".to_owned()];
        let e = elementos(&linhas, corpo, area(), 3, None, None);
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
        let e = elementos(&["oi".to_owned()], corpo, area(), 3, None, None);
        let caixa = limites(&e);
        assert!(caixa.y >= corpo.baixo(), "embaixo: {caixa:?}");
        // Encostado na esquerda: preso dentro da área.
        let colado = Ret::novo(0, 900, 60, 60);
        let caixa = limites(&elementos(
            &["uma linha bem comprida".into()],
            colado,
            area(),
            3,
            None,
            None,
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
    fn datilografia_revela_da_esquerda_para_a_direita_e_depois_tudo() {
        let b = Balao::novo(vec!["abc".to_owned(), "de".to_owned()], 1_000);
        assert_eq!(b.total_de_caracteres(), 5);
        // A datilografia só começa depois do pop (decisão 0122): durante o pop,
        // nada; depois, um a um (contando os espaços); no fim, tudo.
        let base = 1_000 + POP_MS;
        assert_eq!(b.revelar_em(1_000), Some(0), "durante o pop, sem texto");
        assert_eq!(b.revelar_em(base), Some(0));
        assert_eq!(b.revelar_em(base + CHAR_MS), Some(1));
        assert_eq!(b.revelar_em(base + 4 * CHAR_MS), Some(4));
        assert_eq!(b.revelar_em(base + 5 * CHAR_MS), None, "tudo revelado");
        assert_eq!(b.revelar_em(10_000), None);
        // A moldura é inteira desde o começo; só os glifos entram aos poucos.
        let corpo = Ret::novo(900, 600, 60, 60);
        let cheio = elementos(&b.linhas, corpo, area(), 3, None, None);
        let vazio = elementos(&b.linhas, corpo, area(), 3, Some(0), None);
        let so_dois = elementos(&b.linhas, corpo, area(), 3, Some(2), None);
        let glifos = |e: &[Elemento]| {
            e.iter()
                .filter(|x| matches!(x, Elemento::Glifo { .. }))
                .count()
        };
        let blocos = |e: &[Elemento]| {
            e.iter()
                .filter(|x| matches!(x, Elemento::Bloco { .. }))
                .count()
        };
        assert_eq!(glifos(&cheio), 5, "a,b,c,d,e");
        assert_eq!(glifos(&vazio), 0, "nenhum glifo ainda");
        assert_eq!(glifos(&so_dois), 2, "só os dois primeiros");
        assert_eq!(blocos(&vazio), blocos(&cheio), "a moldura já vem inteira");
    }

    #[test]
    fn pop_de_entrada_cresce_do_rabinho_ate_o_tamanho_cheio() {
        let b = Balao::novo(vec!["oi".to_owned()], 1_000);
        // Dois passos crescendo e depois o tamanho cheio (decisão 0122).
        assert_eq!(b.escala_em(1_000), Some((60, 100)));
        assert_eq!(b.escala_em(1_000 + 34), Some((84, 100)));
        assert_eq!(b.escala_em(1_000 + POP_MS), None, "cheio");
        assert_eq!(b.escala_em(5_000), None);
        // A moldura do pop cabe dentro da cheia e é menor (cresce do rabinho,
        // perto do corpo). Sem texto durante o pop.
        let corpo = Ret::novo(900, 600, 60, 60);
        let cheio = limites(&elementos(&b.linhas, corpo, area(), 3, Some(0), None));
        let popando = limites(&elementos(
            &b.linhas,
            corpo,
            area(),
            3,
            Some(0),
            Some((60, 100)),
        ));
        assert!(popando.w < cheio.w && popando.h < cheio.h, "menor no pop");
        assert!(
            area().intersecao(&popando) == Some(popando),
            "dentro da área: {popando:?}"
        );
        // A ponta do rabinho (perto do corpo) fica ancorada: o pop cresce dela.
        assert!(
            popando.baixo() <= cheio.baixo() + 1 && popando.baixo() >= cheio.baixo() - 3,
            "ancorado no rabinho: {popando:?} vs {cheio:?}"
        );
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
        // Uma sessão sem evento fica até uma semana na lista (decisão 0096):
        // de um dia em diante, em dias (decisão 0099; antes, até "167 h").
        const H: u64 = 60 * 60 * 1000;
        assert_eq!(duracao(24 * H - 1), "23 h");
        assert_eq!(duracao(24 * H), "1 d");
        assert_eq!(duracao(2 * 24 * H + 5 * H), "2 d");
        assert_eq!(duracao(7 * 24 * H - 1), "6 d");
        assert_eq!(duracao(7 * 24 * H), "7 d");
        assert_eq!(
            linha_da_sessao(Some("claude-pet"), "parado", 0, 3 * 24 * H, false),
            "claude-pet: parado (3 d)"
        );
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

    #[test]
    fn a_chamada_pelo_tipo_da_espera_cabe_no_balao() {
        // A variante 0 é a fala de sempre (decisão 0123: o banco é um superset).
        assert_eq!(
            linhas_da_espera(Some(TipoEspera::Pergunta), Some("api"), 0),
            vec!["api: pergunta pra você"]
        );
        assert_eq!(
            linhas_da_espera(Some(TipoEspera::Plano), Some("api"), 0),
            vec!["Plano pra aprovar! api"]
        );
        assert_eq!(
            linhas_da_espera(Some(TipoEspera::Permissao), Some("api"), 0),
            vec!["Ô, meu camarada!", "api precisa de você"]
        );
        assert_eq!(
            linhas_da_espera(Some(TipoEspera::Elicitacao), None, 0),
            vec!["Ô, meu camarada!", "precisa de você"]
        );
        // Toda variante, de todo tipo, com nome longo ou sem, cabe no balão e só
        // usa glifos da fonte (malandro, mas legível).
        let longo = Some("agenda-presidencial-2026");
        for v in 0..VARIANTES_DE_FALA {
            for espera in [
                None,
                Some(TipoEspera::Permissao),
                Some(TipoEspera::Elicitacao),
                Some(TipoEspera::Plano),
                Some(TipoEspera::Pergunta),
            ] {
                for proj in [longo, Some("api"), None] {
                    for linha in linhas_da_espera(espera, proj, v) {
                        assert!(
                            linha.chars().count() <= MAX_CARACTERES,
                            "v{v} {linha:?}: o balão cortaria ({} chars)",
                            linha.chars().count()
                        );
                        assert!(linha.chars().all(fonte::tem), "v{v} {linha:?}: glifo");
                    }
                }
            }
            // O erro também varia e cabe.
            for proj in [longo, None] {
                let l = linhas_de_erro(proj, v);
                assert!(l.chars().count() <= MAX_CARACTERES, "erro v{v} {l:?}");
                assert!(l.chars().all(fonte::tem), "erro v{v} {l:?}");
            }
        }
        // A variante é estável para a mesma marca e varia entre marcas.
        assert_eq!(variante(1_000), variante(1_000));
        let vistas: std::collections::BTreeSet<_> =
            (0..50u64).map(|i| variante(i * 7_000 + 3)).collect();
        assert!(vistas.len() >= 2, "o banco varia entre chamadas");
    }
}
