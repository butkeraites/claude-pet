//! Os selos ao lado do pet (decisão 0083): o selo do aviso de espera ("!"),
//! o "+N" das outras sessões, o "…" da corrente de agentes e as bandeirinhas
//! dos prontos, na cor do projeto. Desenhos parados, no estilo do selo "zZ"
//! da soneca: um pixel do selo vale `dt` (a metade do D, como a fonte dos
//! balões), em blocos inteiros, com a tinta e o creme do balão.
//!
//! Ficam numa fileira ao lado do corpo, na altura da cabeça: à esquerda (o
//! pet mora no canto inferior direito), ou à direita se não couber; nunca em
//! cima da área de toque e nunca fora da área útil (as bandeirinhas que não
//! cabem saem, da mais nova para a mais velha). Do corpo para fora: o aviso,
//! o "+N", o "…" e as bandeirinhas, da mais velha para a mais nova.
//!
//! O selo do aviso pulsa na L4 (decisão 0075): troca de cor uma vez por
//! segundo (o amarelo e o vermelho), um commit por segundo.

use crate::cena::Elemento;
use crate::fonte;
use crate::geometria::Ret;

/// Tinta e creme do balão (BGRA pré-multiplicado, opacos).
const TINTA: [u8; 4] = [0x2A, 0x1B, 0x1D, 0xFF];
const CREME: [u8; 4] = [0xE3, 0xF1, 0xF7, 0xFF];
/// O amarelo do aviso (#FFD23F) e o vermelho do pulso aceso (#E5394B).
const AMARELO: [u8; 4] = [0x3F, 0xD2, 0xFF, 0xFF];
const VERMELHO: [u8; 4] = [0x4B, 0x39, 0xE5, 0xFF];

/// As 8 cores dos projetos (`tela::cor`), vivas para ler no tema escuro, com
/// a borda de tinta para ler no claro: vermelho, laranja, amarelo, verde,
/// turquesa, azul, roxo e rosa (BGRA).
pub const PALETA: [[u8; 4]; 8] = [
    [0x4B, 0x39, 0xE5, 0xFF],
    [0x28, 0x8C, 0xF2, 0xFF],
    [0x3F, 0xD2, 0xFF, 0xFF],
    [0x5A, 0xC3, 0x4C, 0xFF],
    [0xB6, 0xC4, 0x2E, 0xFF],
    [0xFF, 0x86, 0x3A, 0xFF],
    [0xE5, 0x5D, 0x9B, 0xFF],
    [0xB6, 0x7A, 0xFF, 0xFF],
];

/// O "!" do aviso, em pixels do selo: `#` tinta, `y` o recheio (amarelo, ou
/// vermelho no pulso aceso).
const AVISO: [&str; 14] = [
    ".####.", //
    "#yyyy#", //
    "#yyyy#", //
    "#yyyy#", //
    "#yyyy#", //
    "#yyyy#", //
    ".#yy#.", //
    ".#yy#.", //
    "..##..", //
    "......", //
    ".####.", //
    "#yyyy#", //
    "#yyyy#", //
    ".####.", //
];

/// A bandeirinha: `p` o mastro (creme), `f` o pano (a cor do projeto). Leva
/// a sombra de tinta um pixel para baixo e para a direita.
const BANDEIRA: [&str; 7] = [
    "pffff", //
    "pffff", //
    "pffff", //
    "p....", //
    "p....", //
    "p....", //
    "p....", //
];

/// Entre o corpo e a fileira, e entre dois selos, em pixels do selo.
const VAO: i32 = 2;
/// A linha do meio da fileira, em pixels do selo abaixo do topo do corpo (o
/// meio do "!").
const MEIO: i32 = 7;

/// O selo do aviso de espera na fileira.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aviso {
    /// O "!" amarelo.
    Normal,
    /// O "!" vermelho (a metade acesa do pulso da L4).
    Aceso,
}

/// O que vai na fileira.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fileira {
    pub aviso: Option<Aviso>,
    /// O "+N" (0: nenhum).
    pub mais: u32,
    pub corrente: bool,
    /// As cores das bandeirinhas (0 a 7), da mais velha para a mais nova.
    pub bandeiras: Vec<u8>,
}

impl Fileira {
    pub fn vazia(&self) -> bool {
        self.aviso.is_none() && self.mais == 0 && !self.corrente && self.bandeiras.is_empty()
    }
}

/// Um selo antes de ir para a tela: a largura e a altura em pixels do selo e
/// quanto ele desce da linha de cima da fileira.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Item {
    Aviso(Aviso),
    Texto(String),
    Bandeira(u8),
}

impl Item {
    fn largura(&self) -> i32 {
        match self {
            Item::Aviso(_) => AVISO[0].len() as i32,
            // O texto com a sombra de um pixel.
            Item::Texto(t) => fonte::largura(t) + 1,
            Item::Bandeira(_) => BANDEIRA[0].len() as i32 + 1,
        }
    }

    /// O topo do item em relação ao topo da fileira (pixels do selo), para o
    /// meio de todos ficar na mesma linha.
    fn desce(&self) -> i32 {
        match self {
            Item::Aviso(_) => MEIO - AVISO.len() as i32 / 2,
            // As letras da monogram (o "+", os algarismos) acendem perto das
            // linhas 2 a 9; o "…", na 9.
            Item::Texto(t) if t == "…" => MEIO - 9,
            Item::Texto(_) => MEIO - 6,
            Item::Bandeira(_) => MEIO - BANDEIRA.len() as i32 / 2,
        }
    }

    /// Desenha o item com o canto (de cima, à esquerda) em (x, y).
    fn desenhar(&self, x: i32, y: i32, dt: i32, saida: &mut Vec<Elemento>) {
        match self {
            Item::Aviso(aviso) => {
                let recheio = match aviso {
                    Aviso::Normal => AMARELO,
                    Aviso::Aceso => VERMELHO,
                };
                bitmap(&AVISO, x, y, dt, saida, |c| match c {
                    b'#' => Some(TINTA),
                    b'y' => Some(recheio),
                    _ => None,
                });
            }
            Item::Texto(texto) => {
                // A sombra de tinta primeiro, um pixel para baixo e para a
                // direita; o creme por cima (como o "zZ").
                for (dx, cor) in [(dt, TINTA), (0, CREME)] {
                    for (i, c) in texto.chars().enumerate() {
                        saida.push(Elemento::Glifo {
                            c,
                            x: x + i as i32 * fonte::AVANCO * dt + dx,
                            y: y + dx,
                            d: dt,
                            cor,
                        });
                    }
                }
            }
            Item::Bandeira(indice) => {
                let pano = PALETA[usize::from(*indice) % PALETA.len()];
                // A sombra de tinta, um pixel para baixo e para a direita.
                bitmap(&BANDEIRA, x + dt, y + dt, dt, saida, |c| {
                    (c != b'.').then_some(TINTA)
                });
                bitmap(&BANDEIRA, x, y, dt, saida, |c| match c {
                    b'p' => Some(CREME),
                    b'f' => Some(pano),
                    _ => None,
                });
            }
        }
    }
}

/// Um desenho em texto (uma linha por fileira de pixels) em blocos `dt`, um
/// retângulo por trecho seguido de mesma cor.
fn bitmap(
    linhas: &[&str],
    x0: i32,
    y0: i32,
    dt: i32,
    saida: &mut Vec<Elemento>,
    cor_de: impl Fn(u8) -> Option<[u8; 4]>,
) {
    for (j, linha) in linhas.iter().enumerate() {
        let bytes = linha.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            let mut fim = i + 1;
            while fim < bytes.len() && bytes[fim] == c {
                fim += 1;
            }
            if let Some(cor) = cor_de(c) {
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
}

/// O texto do "+N" (até "+99").
fn mais(n: u32) -> String {
    format!("+{}", n.min(99))
}

/// Os elementos da fileira ao lado do `corpo` (a área de toque, no palco),
/// dentro da `area` útil, com pixel de selo `dt`.
pub fn elementos(fileira: &Fileira, corpo: Ret, area: Ret, dt: i32) -> Vec<Elemento> {
    let dt = dt.max(1);
    let mut itens: Vec<Item> = Vec::new();
    if let Some(aviso) = fileira.aviso {
        itens.push(Item::Aviso(aviso));
    }
    if fileira.mais > 0 {
        itens.push(Item::Texto(mais(fileira.mais)));
    }
    if fileira.corrente {
        itens.push(Item::Texto("…".into()));
    }
    itens.extend(fileira.bandeiras.iter().map(|&b| Item::Bandeira(b)));
    let largura =
        |itens: &[Item]| -> i32 { itens.iter().map(|i| (i.largura() + VAO) * dt).sum::<i32>() };
    let cabe_esquerda = |w: i32| corpo.x - w >= area.x;
    let cabe_direita = |w: i32| corpo.direita() + w <= area.direita();
    // O lado com mais espaço quando nenhum cabe tudo; as bandeirinhas mais
    // novas saem até caber.
    let esquerda = cabe_esquerda(largura(&itens))
        || (!cabe_direita(largura(&itens)) && corpo.x - area.x >= area.direita() - corpo.direita());
    while !itens.is_empty() {
        let w = largura(&itens);
        if (esquerda && cabe_esquerda(w)) || (!esquerda && cabe_direita(w)) {
            break;
        }
        itens.pop();
    }
    // A linha de cima da fileira, presa na área (a cabeça perto da borda).
    let altura = itens
        .iter()
        .map(|i| i.desce() + itens_altura(i))
        .max()
        .unwrap_or(0);
    let topo_min = itens.iter().map(Item::desce).min().unwrap_or(0);
    let topo = (corpo.y)
        .max(area.y - topo_min * dt)
        .min(area.baixo() - altura * dt);
    let mut saida = Vec::new();
    let mut borda = if esquerda { corpo.x } else { corpo.direita() };
    for item in &itens {
        let w = item.largura() * dt;
        let x = if esquerda {
            borda -= VAO * dt + w;
            borda
        } else {
            let x = borda + VAO * dt;
            borda = x + w;
            x
        };
        item.desenhar(x, topo + item.desce() * dt, dt, &mut saida);
    }
    saida
}

/// A altura de um item em pixels do selo.
fn itens_altura(item: &Item) -> i32 {
    match item {
        Item::Aviso(_) => AVISO.len() as i32,
        Item::Texto(_) => fonte::ALTURA + 1,
        Item::Bandeira(_) => BANDEIRA.len() as i32 + 1,
    }
}

/// O "!!" do voo da escalada (decisão 0084): dois "!" do aviso lado a lado,
/// no pixel `px` (o D da arte), com o canto em (x, y).
pub fn exclamacoes(x: i32, y: i32, px: i32) -> Vec<Elemento> {
    let px = px.max(1);
    let mut saida = Vec::new();
    let largura = AVISO[0].len() as i32;
    for k in 0..2 {
        bitmap(
            &AVISO,
            x + k * (largura + 1) * px,
            y,
            px,
            &mut saida,
            |c| match c {
                b'#' => Some(TINTA),
                b'y' => Some(AMARELO),
                _ => None,
            },
        );
    }
    saida
}

/// O tamanho do "!!" em pixels do dispositivo, no pixel `px`.
pub fn tamanho_das_exclamacoes(px: i32) -> (i32, i32) {
    let largura = AVISO[0].len() as i32;
    (
        (2 * largura + 1) * px.max(1),
        AVISO.len() as i32 * px.max(1),
    )
}

#[cfg(test)]
mod testes {
    use super::*;

    fn area() -> Ret {
        Ret::novo(0, 0, 1920, 1200)
    }

    /// O corpo do Zeca original no canto, com D = 4 (33 x 34 de arte).
    fn corpo() -> Ret {
        Ret::novo(1_740, 1_016, 132, 136)
    }

    fn cheia() -> Fileira {
        Fileira {
            aviso: Some(Aviso::Normal),
            mais: 3,
            corrente: true,
            bandeiras: vec![3, 5, 7],
        }
    }

    fn caixa(elementos: &[Elemento]) -> Ret {
        let mut x0 = i32::MAX;
        let mut y0 = i32::MAX;
        let mut x1 = i32::MIN;
        let mut y1 = i32::MIN;
        for e in elementos {
            let r = match *e {
                Elemento::Bloco { ret, .. } => ret,
                Elemento::Glifo { x, y, d, .. } => {
                    Ret::novo(x, y, fonte::LARGURA_MAX * d, fonte::ALTURA * d)
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
    fn a_fileira_fica_a_esquerda_do_corpo_em_blocos_inteiros_e_dentro_da_area() {
        let e = elementos(&cheia(), corpo(), area(), 2);
        assert!(!e.is_empty());
        let c = caixa(&e);
        assert!(c.direita() <= corpo().x, "à esquerda do corpo: {c:?}");
        assert_eq!(c.intersecao(&area()), Some(c), "dentro da área");
        assert!(
            c.y >= corpo().y - 2 * 2 && c.y < corpo().y + 40,
            "na altura da cabeça: {c:?}"
        );
        for el in &e {
            match *el {
                Elemento::Bloco { ret, cor } => {
                    assert_eq!((ret.w % 2, ret.h % 2), (0, 0), "blocos de 2: {ret:?}");
                    assert_eq!(cor[3], 255, "opaco");
                    assert!(ret.intersecao(&corpo()).is_none(), "nada em cima do corpo");
                }
                Elemento::Glifo { d, .. } => assert_eq!(d, 2),
                Elemento::Sprite { .. } => panic!("sem sprite"),
            }
        }
        // As cores das bandeirinhas estão lá; o "!" amarelo.
        let tem = |cor: [u8; 4]| {
            e.iter()
                .any(|x| matches!(x, Elemento::Bloco { cor: c, .. } if *c == cor))
        };
        assert!(tem(PALETA[3]) && tem(PALETA[5]) && tem(PALETA[7]));
        assert!(tem(AMARELO) && !tem(VERMELHO));
        let aceso = Fileira {
            aviso: Some(Aviso::Aceso),
            ..cheia()
        };
        let e = elementos(&aceso, corpo(), area(), 2);
        assert!(
            e.iter()
                .any(|x| matches!(x, Elemento::Bloco { cor, .. } if *cor == VERMELHO))
        );
    }

    #[test]
    fn sem_espaco_a_esquerda_vai_para_a_direita_e_as_bandeirinhas_novas_saem() {
        let colado = Ret::novo(10, 500, 132, 136);
        let e = elementos(&cheia(), colado, area(), 2);
        let c = caixa(&e);
        assert!(c.x >= colado.direita(), "à direita: {c:?}");
        assert_eq!(c.intersecao(&area()), Some(c));
        // Um monitor estreito: nem de um lado nem do outro cabe tudo; fica o
        // lado com mais espaço, e as bandeirinhas mais novas saem.
        let muitas = Fileira {
            bandeiras: vec![0, 1, 2, 3, 4, 5, 6, 7],
            ..cheia()
        };
        let estreita = Ret::novo(0, 0, 300, 400);
        let meio = Ret::novo(120, 100, 60, 60);
        let e = elementos(&muitas, meio, estreita, 2);
        let c = caixa(&e);
        assert_eq!(c.intersecao(&estreita), Some(c), "dentro: {c:?}");
        assert!(c.intersecao(&meio).is_none());
        let cores: Vec<[u8; 4]> = e
            .iter()
            .filter_map(|x| match x {
                Elemento::Bloco { cor, .. } if PALETA.contains(cor) => Some(*cor),
                _ => None,
            })
            .collect();
        assert!(cores.contains(&PALETA[1]), "a mais velha fica");
        assert!(!cores.contains(&PALETA[7]), "a mais nova sai");
    }

    #[test]
    fn a_cabeca_perto_da_borda_de_cima_puxa_a_fileira_para_dentro() {
        let alto = Ret::novo(800, 0, 132, 136);
        let e = elementos(&cheia(), alto, area(), 2);
        let c = caixa(&e);
        assert!(c.y >= 0, "{c:?}");
        assert!(elementos(&Fileira::default(), alto, area(), 2).is_empty());
        assert!(Fileira::default().vazia() && !cheia().vazia());
        assert_eq!(mais(250), "+99");
    }

    #[test]
    fn as_exclamacoes_do_voo_em_blocos_do_d() {
        let e = exclamacoes(100, 40, 4);
        let c = caixa(&e);
        assert_eq!((c.w, c.h), tamanho_das_exclamacoes(4));
        assert!(
            e.iter()
                .all(|x| matches!(x, Elemento::Bloco { ret, .. } if ret.w % 4 == 0 && ret.h == 4))
        );
    }
}
