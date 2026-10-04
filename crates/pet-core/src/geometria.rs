//! Geometria em pixels inteiros do monitor.
//!
//! Nitidez por construção (decisão 0004): cada pixel de arte vira um bloco
//! D×D de pixels do monitor, com D inteiro por monitor, e toda posição é um
//! pixel inteiro do monitor. Nada aqui usa ponto flutuante para posicionar;
//! a escala fracionária só entra para converter tamanhos lógicos.

use serde::Serialize;

/// Retângulo em pixels (do monitor ou de arte, conforme o contexto).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
pub struct Ret {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Ret {
    pub const fn novo(x: i32, y: i32, w: i32, h: i32) -> Ret {
        Ret { x, y, w, h }
    }

    pub fn vazio(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    pub fn direita(&self) -> i32 {
        self.x + self.w
    }

    pub fn baixo(&self) -> i32 {
        self.y + self.h
    }

    pub fn area(&self) -> i64 {
        if self.vazio() {
            0
        } else {
            self.w as i64 * self.h as i64
        }
    }

    pub fn contem(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.direita() && y >= self.y && y < self.baixo()
    }

    /// Interseção; `None` se não se tocam.
    pub fn intersecao(&self, outro: &Ret) -> Option<Ret> {
        let x0 = self.x.max(outro.x);
        let y0 = self.y.max(outro.y);
        let x1 = self.direita().min(outro.direita());
        let y1 = self.baixo().min(outro.baixo());
        (x1 > x0 && y1 > y0).then(|| Ret::novo(x0, y0, x1 - x0, y1 - y0))
    }

    /// Deslocado por (dx, dy).
    pub fn deslocado(&self, dx: i32, dy: i32) -> Ret {
        Ret::novo(self.x + dx, self.y + dy, self.w, self.h)
    }
}

/// Fração da altura lógica do monitor que o corpo do pet ocupa.
pub const FRACAO_CORPO: f64 = 0.12;
/// Limites do corpo, em pixels lógicos.
pub const CORPO_MIN: f64 = 80.0;
pub const CORPO_MAX: f64 = 160.0;
/// Margem padrão até a borda do monitor, em pixels lógicos.
pub const MARGEM_LOGICA: i32 = 16;

/// D: pixels do monitor por pixel de arte, inteiro e por monitor.
///
/// O alvo é o corpo ocupar ~12% da altura lógica do monitor, entre 80 e 160
/// pixels lógicos; `corpo_px` é a altura do corpo na arte.
pub fn calcular_d(altura_logica: u32, escala: f64, corpo_px: u32) -> i32 {
    let alvo = (altura_logica as f64 * FRACAO_CORPO).clamp(CORPO_MIN, CORPO_MAX);
    let d = (alvo * escala / corpo_px.max(1) as f64).round();
    (d as i32).max(1)
}

/// Pixels lógicos → pixels do monitor, arredondado.
pub fn para_dispositivo(logico: i32, escala: f64) -> i32 {
    (logico as f64 * escala).round() as i32
}

/// Retângulo em pixels do monitor → coordenadas lógicas da superfície,
/// arredondando **para fora** (a região de input nunca fica menor que o
/// corpo desenhado).
pub fn para_logico_por_fora(r: Ret, escala: f64) -> Ret {
    if r.vazio() || escala <= 0.0 {
        return Ret::default();
    }
    let x0 = (r.x as f64 / escala).floor() as i32;
    let y0 = (r.y as f64 / escala).floor() as i32;
    let x1 = (r.direita() as f64 / escala).ceil() as i32;
    let y1 = (r.baixo() as f64 / escala).ceil() as i32;
    Ret::novo(x0, y0, x1 - x0, y1 - y0)
}

/// Âncoras da skin em pixels de arte, dentro da célula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ancoras {
    pub celula: (i32, i32),
    /// Ponto do chão: x no meio dos pés, y na linha onde os pés pisam.
    pub pe: (i32, i32),
    /// Área clicável (corpo), em pixels de arte.
    pub toque: Ret,
}

impl Ancoras {
    /// A área clicável, espelhada se o pet olha para o outro lado.
    pub fn toque(&self, espelhar: bool) -> Ret {
        if espelhar {
            Ret::novo(
                self.celula.0 - self.toque.direita(),
                self.toque.y,
                self.toque.w,
                self.toque.h,
            )
        } else {
            self.toque
        }
    }

    /// Área clicável em pixels do monitor, com a célula em (x, y).
    pub fn toque_no_monitor(&self, x: i32, y: i32, d: i32, espelhar: bool) -> Ret {
        let t = self.toque(espelhar);
        Ret::novo(x + t.x * d, y + t.y * d, t.w * d, t.h * d)
    }
}

/// Posição padrão da célula (canto superior esquerdo, pixels do monitor):
/// canto inferior direito, com a borda direita do corpo e os pés a 16
/// pixels lógicos das bordas, presa para o corpo ficar inteiro na tela.
pub fn posicao_padrao(tela: (i32, i32), escala: f64, d: i32, ancoras: &Ancoras) -> (i32, i32) {
    let margem = para_dispositivo(MARGEM_LOGICA, escala);
    let toque = ancoras.toque(false);
    let x = tela.0 - margem - toque.direita() * d;
    let y = tela.1 - margem - ancoras.pe.1 * d;
    prender(x, y, tela, d, ancoras, false)
}

/// Prende a célula para a área clicável ficar inteira dentro da tela.
pub fn prender(
    x: i32,
    y: i32,
    tela: (i32, i32),
    d: i32,
    ancoras: &Ancoras,
    espelhar: bool,
) -> (i32, i32) {
    let t = ancoras.toque(espelhar);
    let x_min = -t.x * d;
    let x_max = tela.0 - t.direita() * d;
    let y_min = -t.y * d;
    let y_max = tela.1 - t.baixo() * d;
    (
        x.min(x_max).max(x_min.min(x_max)),
        y.min(y_max).max(y_min.min(y_max)),
    )
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ancoras() -> Ancoras {
        Ancoras {
            celula: (48, 48),
            pe: (24, 45),
            toque: Ret::novo(10, 13, 28, 32),
        }
    }

    #[test]
    fn intersecao_e_vazio() {
        let a = Ret::novo(0, 0, 10, 10);
        assert_eq!(
            a.intersecao(&Ret::novo(5, 5, 10, 10)),
            Some(Ret::novo(5, 5, 5, 5))
        );
        assert_eq!(a.intersecao(&Ret::novo(10, 0, 5, 5)), None, "só encosta");
        assert!(Ret::novo(0, 0, 0, 5).vazio());
        assert_eq!(Ret::novo(1, 2, 3, 4).area(), 12);
        assert!(a.contem(9, 9) && !a.contem(10, 0));
    }

    #[test]
    fn d_no_notebook_no_4k_e_nos_limites() {
        // eDP-1: 800 lógicos → 96 px de corpo × 1.5 = 144 / 32 = 4.5 → 5.
        assert_eq!(calcular_d(800, 1.5, 32), 5);
        // 4K: 1440 lógicos → 160 (teto) × 1.5 = 240 / 32 = 7.5 → 8.
        assert_eq!(calcular_d(1440, 1.5, 32), 8);
        // Monitor pequeno: piso de 80 lógicos.
        assert_eq!(calcular_d(400, 1.0, 40), 2);
        // Nunca menos que 1.
        assert_eq!(calcular_d(800, 1.0, 1000), 1);
    }

    #[test]
    fn logico_por_fora() {
        // 1706..1896 em pixels do monitor a 1.5 → 1137.33..1264 → 1137..1264.
        let r = para_logico_por_fora(Ret::novo(1706, 1000, 190, 160), 1.5);
        assert_eq!(r, Ret::novo(1137, 666, 127, 108));
        assert_eq!(
            para_logico_por_fora(Ret::novo(0, 0, 6, 6), 1.5),
            Ret::novo(0, 0, 4, 4)
        );
    }

    #[test]
    fn posicao_padrao_no_canto_inferior_direito() {
        let a = ancoras();
        let (x, y) = posicao_padrao((1920, 1200), 1.5, 5, &a);
        // Corpo termina a 24 px (16 lógicos) da direita; pés a 24 px do chão.
        assert_eq!(x + 38 * 5, 1920 - 24);
        assert_eq!(y + 45 * 5, 1200 - 24);
        let toque = a.toque_no_monitor(x, y, 5, false);
        assert!(toque.x >= 0 && toque.direita() <= 1920 && toque.baixo() <= 1200);
    }

    #[test]
    fn prender_deixa_o_corpo_inteiro() {
        let a = ancoras();
        assert_eq!(prender(-500, -500, (1920, 1200), 5, &a, false), (-50, -65));
        let (x, y) = prender(5000, 5000, (1920, 1200), 5, &a, false);
        let t = a.toque_no_monitor(x, y, 5, false);
        assert_eq!((t.direita(), t.baixo()), (1920, 1200));
    }

    #[test]
    fn toque_espelhado() {
        let a = ancoras();
        assert_eq!(a.toque(true), Ret::novo(10, 13, 28, 32), "corpo centrado");
        let torto = Ancoras {
            toque: Ret::novo(4, 0, 10, 8),
            ..a
        };
        assert_eq!(torto.toque(true), Ret::novo(34, 0, 10, 8));
    }
}
