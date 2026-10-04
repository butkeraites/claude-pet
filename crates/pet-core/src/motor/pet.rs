//! O pet na tela: personagem, onde ele fica e que quadro mostra agora.
//!
//! Parado (pose fixa com rajadas, para caber no orçamento de commits) no
//! canto inferior direito, e tocando uma reação de cada vez (M3: o aceno do
//! T0, o pulinho do T1 e o `tocar`). Arraste e seguir o monitor chegam no
//! M4.

use std::rc::Rc;

use crate::animador::Animador;
use crate::cena::Elemento;
use crate::geometria::{self, Ret};
use crate::skin::Skin;

/// Onde e em que escala o pet é desenhado numa superfície.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palco {
    /// Buffer em pixels do monitor.
    pub tela: (i32, i32),
    pub escala: f64,
    /// Pixels do monitor por pixel de arte.
    pub d: i32,
    /// Canto superior esquerdo da célula, em pixels do monitor.
    pub x: i32,
    pub y: i32,
}

pub struct Pet {
    skin: Rc<Skin>,
    animador: Animador,
}

impl Pet {
    pub fn novo(skin: Rc<Skin>, agora_ms: u64) -> Pet {
        let animador = Animador::novo(&skin, agora_ms);
        Pet { skin, animador }
    }

    /// Toca a reação uma vez e volta à pose; `false` se a skin não sabe
    /// tocá-la.
    pub fn tocar(&mut self, reacao: &str, agora_ms: u64) -> bool {
        self.animador.tocar(&self.skin, reacao, agora_ms)
    }

    /// A reação tocando agora, se houver.
    pub fn reacao(&self, agora_ms: u64) -> Option<&str> {
        self.animador.reacao(agora_ms)
    }

    pub fn skin(&self) -> &Skin {
        &self.skin
    }

    /// D e posição padrão para uma superfície de tamanho lógico `logico`.
    pub fn palco(&self, logico: (u32, u32), escala: f64, tela: (i32, i32)) -> Palco {
        let d = geometria::calcular_d(logico.1, escala, self.skin.corpo_px);
        let (x, y) = geometria::posicao_padrao(tela, escala, d, &self.skin.ancoras);
        Palco {
            tela,
            escala,
            d,
            x,
            y,
        }
    }

    /// A célula do sprite em pixels do monitor, recortada à tela: o
    /// `sprite_disp` do `/v1/estado` (a foto e a nitidez recortam por ele).
    pub fn sprite_disp(&self, palco: &Palco) -> Option<Ret> {
        let (w, h) = self.skin.ancoras.celula;
        Ret::novo(palco.x, palco.y, w * palco.d, h * palco.d).intersecao(&Ret::novo(
            0,
            0,
            palco.tela.0,
            palco.tela.1,
        ))
    }

    /// A região clicável (área de toque da skin) em coordenadas lógicas da
    /// superfície, arredondada para fora; o resto da tela continua
    /// recebendo os cliques normalmente.
    pub fn regiao_de_toque(&self, palco: &Palco) -> Option<Ret> {
        let toque = self
            .skin
            .ancoras
            .toque_no_monitor(palco.x, palco.y, palco.d, false)
            .intersecao(&Ret::novo(0, 0, palco.tela.0, palco.tela.1))?;
        Some(geometria::para_logico_por_fora(toque, palco.escala))
    }

    /// O sprite agora e o instante da próxima troca de quadro.
    pub fn sprite(&self, palco: &Palco, agora_ms: u64) -> (Elemento, u64) {
        let (quadro, proxima) = self.animador.em(agora_ms);
        let sprite = Elemento::Sprite {
            quadro,
            x: palco.x,
            y: palco.y,
            d: palco.d,
            espelhar: false,
        };
        (sprite, proxima)
    }

    /// A cena inteira agora e o instante da próxima mudança.
    pub fn cena(&self, palco: &Palco, agora_ms: u64) -> (Vec<Elemento>, Option<u64>) {
        let (sprite, proxima) = self.sprite(palco, agora_ms);
        (vec![sprite], Some(proxima))
    }
}

#[cfg(test)]
mod testes {
    use std::path::PathBuf;

    use super::*;

    fn pet() -> Pet {
        let pasta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skins/_teste");
        Pet::novo(Rc::new(Skin::carregar(&pasta).unwrap()), 0)
    }

    #[test]
    fn palco_no_notebook() {
        let pet = pet();
        let palco = pet.palco((1280, 800), 1.5, (1920, 1200));
        assert_eq!(palco.d, 5);
        // Corpo (toque 10..38) termina a 24 px da direita; pés a 24 px do chão.
        assert_eq!(palco.x + 38 * 5, 1896);
        assert_eq!(palco.y + 45 * 5, 1176);
    }

    #[test]
    fn sprite_disp_e_regiao_de_toque() {
        let pet = pet();
        let palco = pet.palco((1280, 800), 1.5, (1920, 1200));
        // Célula 240x240 em (1706, 951): passa 26 px da borda direita.
        assert_eq!(
            pet.sprite_disp(&palco),
            Some(Ret::novo(1706, 951, 214, 240))
        );
        // Toque [10,13,28,32] × 5 = (1756, 1016, 140, 160) em pixels do
        // monitor → (1170.67.., 677.33.., …) lógicos, arredondado para fora.
        assert_eq!(
            pet.regiao_de_toque(&palco),
            Some(Ret::novo(1170, 677, 94, 107))
        );
    }

    #[test]
    fn cena_parada_e_um_sprite_com_proxima_troca() {
        let pet = pet();
        let palco = pet.palco((1280, 800), 1.5, (1920, 1200));
        let (cena, proxima) = pet.cena(&palco, 0);
        assert_eq!(cena.len(), 1);
        assert!(proxima.unwrap() > 0);
        let Elemento::Sprite { quadro, .. } = cena[0] else {
            panic!("esperava sprite");
        };
        assert_eq!(quadro, 0, "pose = primeiro quadro de idle");
    }

    #[test]
    fn reacao_troca_o_quadro_e_volta() {
        let mut pet = pet();
        let palco = pet.palco((1280, 800), 1.5, (1920, 1200));
        assert!(pet.tocar("done_small", 1000));
        assert_eq!(pet.reacao(1000), Some("done_small"));
        let (cena, proxima) = pet.cena(&palco, 1000);
        let Elemento::Sprite { quadro, .. } = cena[0] else {
            panic!("esperava sprite");
        };
        assert_eq!(quadro, 29, "primeiro quadro de done_small");
        assert_eq!(proxima, Some(1200));
        assert_eq!(pet.reacao(1400), None);
        assert!(!pet.tocar("bye", 2000), "tchau sem animação no M3");
    }
}
