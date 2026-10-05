//! O pet na tela: personagem, onde ele fica e que quadro mostra agora.
//!
//! Na base que a tela segura (M5: o estado da skin e o ritmo dele, decisão
//! 0082; parado, pose fixa com rajadas, para caber no orçamento de commits)
//! no canto inferior direito (ou onde foi arrastado), tocando uma reação de
//! cada vez (M3: o aceno do T0, o pulinho do T1 e o `tocar`) e pendurado em
//! laço enquanto é arrastado (M4).

use std::rc::Rc;

use crate::animador::{self, Animador, Base};
use crate::cena::Elemento;
use crate::geometria::{self, Ret, Tamanho};
use crate::plataforma::Monitor;
use crate::skin::Skin;

/// Onde e em que escala o pet é desenhado no palco (os pixels do
/// dispositivo do monitor, decisão 0044).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palco {
    /// O palco inteiro: o monitor em pixels do dispositivo.
    pub tela: (i32, i32),
    pub escala: f64,
    /// Pixels do monitor por pixel de arte.
    pub d: i32,
    /// Canto superior esquerdo da célula, no palco.
    pub x: i32,
    pub y: i32,
    /// A área útil do monitor no palco: o corpo do pet fica sempre dentro
    /// dela (na posição padrão e no arraste).
    pub area: Ret,
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

    /// O pet parado, com o sorteio das micro-ações na `semente`.
    pub fn com_semente(skin: Rc<Skin>, agora_ms: u64, semente: u64) -> Pet {
        let animador = Animador::com_semente(&skin, agora_ms, semente);
        Pet { skin, animador }
    }

    /// Segura a `base` (o estado da skin e o ritmo); `true` se mudou.
    pub fn definir_base(&mut self, base: Base, agora_ms: u64) -> bool {
        self.animador.definir_base(&self.skin, base, agora_ms)
    }

    /// A base que o pet segura.
    pub fn base(&self) -> &Base {
        self.animador.base()
    }

    /// Toca a reação uma vez e volta à pose; `false` se a skin não sabe
    /// tocá-la.
    pub fn tocar(&mut self, reacao: &str, agora_ms: u64) -> bool {
        self.animador.tocar(&self.skin, reacao, agora_ms)
    }

    /// A reação (ou o estado segurado) tocando agora, se houver.
    pub fn reacao(&self, agora_ms: u64) -> Option<&str> {
        self.animador.reacao(agora_ms)
    }

    /// Segura um estado em laço (o `dangle` enquanto arrasta); `false` se a
    /// skin não sabe tocá-lo.
    pub fn segurar(&mut self, estado: &str, agora_ms: u64) -> bool {
        self.animador.segurar(&self.skin, estado, agora_ms)
    }

    /// Larga o estado segurado.
    pub fn largar(&mut self, agora_ms: u64) {
        self.animador.largar(&self.skin, agora_ms);
    }

    pub fn skin(&self) -> &Skin {
        &self.skin
    }

    /// D e posição padrão num monitor, no tamanho pedido
    /// (`aparencia.tamanho`): o canto inferior direito da área útil dele.
    pub fn palco(&self, monitor: &Monitor, tamanho: Tamanho) -> Palco {
        let escala = monitor.escala;
        let d = geometria::calcular_d_com(monitor.logico.1, escala, self.skin.corpo_px, tamanho);
        let area = monitor.area_util();
        let (x, y) = geometria::posicao_padrao(area, escala, d, &self.skin.ancoras);
        Palco {
            tela: monitor.buffer(),
            escala,
            d,
            x,
            y,
            area,
        }
    }

    /// A célula em (x, y), presa para o corpo ficar inteiro dentro da área
    /// útil do palco.
    pub fn prender(&self, palco: &Palco, x: i32, y: i32) -> (i32, i32) {
        geometria::prender(x, y, palco.area, palco.d, &self.skin.ancoras, false)
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

    /// A área de toque (o corpo, pela skin) no palco, recortada a ele: o
    /// clique fora dela atravessa o pet. A janela converte para as
    /// coordenadas dela (no Wayland, lógicas e arredondadas para fora).
    pub fn toque_no_palco(&self, palco: &Palco) -> Option<Ret> {
        self.skin
            .ancoras
            .toque_no_monitor(palco.x, palco.y, palco.d, false)
            .intersecao(&Ret::novo(0, 0, palco.tela.0, palco.tela.1))
    }

    /// O ponto (x, y) do palco cai no corpo do pet: é por aqui que o clique
    /// e o arraste do M4 começam, em qualquer janela.
    pub fn acerta(&self, palco: &Palco, x: i32, y: i32) -> bool {
        self.toque_no_palco(palco).is_some_and(|t| t.contem(x, y))
    }

    /// O sprite agora e o instante da próxima troca de quadro (`None`: não
    /// troca mais sozinho, como a pose do sono profundo).
    pub fn sprite(&self, palco: &Palco, agora_ms: u64) -> (Elemento, Option<u64>) {
        let (quadro, proxima) = self.animador.em(agora_ms);
        let proxima = (proxima != animador::NUNCA).then_some(proxima);
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
        (vec![sprite], proxima)
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

    fn edp() -> Monitor {
        Monitor {
            nome: Some("eDP-1".into()),
            logico: (1280, 800),
            escala: 1.5,
            ..Monitor::default()
        }
    }

    #[test]
    fn palco_no_notebook() {
        let pet = pet();
        let palco = pet.palco(&edp(), Tamanho::Normal);
        assert_eq!(palco.d, 5);
        assert_eq!(palco.tela, (1920, 1200));
        // Corpo (toque 10..38) termina a 24 px da direita; pés a 24 px do chão.
        assert_eq!(palco.x + 38 * 5, 1896);
        assert_eq!(palco.y + 45 * 5, 1176);
    }

    #[test]
    fn sprite_disp_e_toque_no_palco() {
        let pet = pet();
        let palco = pet.palco(&edp(), Tamanho::Normal);
        // Célula 240x240 em (1706, 951): passa 26 px da borda direita.
        assert_eq!(
            pet.sprite_disp(&palco),
            Some(Ret::novo(1706, 951, 214, 240))
        );
        // Toque [10,13,28,32] × 5 = (1756, 1016, 140, 160) no palco; a camada
        // do Wayland o pede em (1170.67.., 677.33.., …) lógicos, arredondado
        // para fora.
        let toque = pet.toque_no_palco(&palco).unwrap();
        assert_eq!(toque, Ret::novo(1756, 1016, 140, 160));
        assert_eq!(
            geometria::para_logico_por_fora(toque, 1.5),
            Ret::novo(1170, 677, 94, 107)
        );
        assert!(pet.acerta(&palco, 1756, 1016) && pet.acerta(&palco, 1895, 1175));
        assert!(
            !pet.acerta(&palco, 1755, 1016),
            "ao lado do corpo atravessa"
        );
        assert!(!pet.acerta(&palco, 1896, 1100));
    }

    #[test]
    fn cena_parada_e_um_sprite_com_proxima_troca() {
        let pet = pet();
        let palco = pet.palco(&edp(), Tamanho::Normal);
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
        let palco = pet.palco(&edp(), Tamanho::Normal);
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
