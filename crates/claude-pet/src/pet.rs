//! O pet na tela: personagem, onde ele fica e que quadro mostra agora.
//!
//! No M1 o pet só sabe ficar parado (pose fixa com rajadas, para caber no
//! orçamento de commits) no canto inferior direito. Cérebro, arraste e
//! reações chegam nos marcos seguintes.

use std::rc::Rc;

use pet_core::animador::Repouso;
use pet_core::cena::Elemento;
use pet_core::geometria;
use pet_core::skin::Skin;

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
    repouso: Repouso,
}

impl Pet {
    pub fn novo(skin: Rc<Skin>, agora_ms: u64) -> Pet {
        let repouso = Repouso::novo(&skin, agora_ms);
        Pet { skin, repouso }
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

    /// O sprite agora e o instante da próxima troca de quadro.
    pub fn sprite(&self, palco: &Palco, agora_ms: u64) -> (Elemento, u64) {
        let (quadro, proxima) = self.repouso.em(agora_ms);
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
}
