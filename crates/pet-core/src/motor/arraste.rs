//! Arrastar e clicar (M4, decisão 0048): a máquina do ponteiro, pura.
//!
//! Tudo no palco (pixels do dispositivo do monitor, decisão 0044). O botão
//! esquerdo apertado no corpo do pet vira **arraste** quando o ponteiro anda
//! mais que o limiar (4 pixels lógicos) ou depois de [`SEGURAR_MS`]
//! segurando; solto antes disso, é um **clique**. O botão direito só clica. A
//! célula do pet anda com o ponteiro em múltiplos de D a partir de onde
//! estava quando o arraste começou (o deslocamento de pixel art, decisão
//! 0004); quem prende o corpo dentro da área útil é o Motor.
//!
//! **Fail-safe:** [`SEM_PONTEIRO_MS`] sem evento do ponteiro com o botão
//! apertado (o compositor perdeu o ponteiro, numa área de trabalho vazia, ou
//! o botão foi solto em outro monitor sem a pegada implícita) solta o pet
//! onde ele está. O Motor também cancela ao esconder, quando a janela fecha e
//! antes de uma viagem.

use crate::plataforma::{Botao, EventoPonteiro};

/// O ponteiro anda mais que isto (em pixels lógicos) com o botão esquerdo
/// apertado: começa o arraste.
pub const LIMIAR_LOGICO: f64 = 4.0;
/// Segurando o botão esquerdo isto, o arraste começa sem andar.
pub const SEGURAR_MS: u64 = 250;
/// Sem evento do ponteiro por isto com o botão apertado: solta.
pub const SEM_PONTEIRO_MS: u64 = 5_000;

/// O que a máquina pede ao Motor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesto {
    Nada,
    /// Apertou no corpo: o cursor de agarrar.
    Apertou,
    /// O arraste começou: o pet fica pendurado e a área de toque cresce para
    /// o palco inteiro.
    Comecou,
    /// O ponteiro andou arrastando: a célula vai para o [`Arraste::alvo`].
    Moveu,
    /// Soltou o arraste com o ponteiro em (x, y) do palco (fora dele, se
    /// soltou em outro monitor); a célula vai para `celula` (antes de prender
    /// na área). `pegada`: onde o ponteiro pegou o pet, relativo à célula.
    Soltou {
        x: i32,
        y: i32,
        celula: (i32, i32),
        pegada: (i32, i32),
    },
    /// Apertou e soltou sem arrastar.
    Clique(Botao),
    /// O arraste acabou sem soltar (o fail-safe): o pet pousa onde está.
    Cancelou,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Estado {
    Livre,
    Apertado {
        botao: Botao,
        x0: i32,
        y0: i32,
        desde_ms: u64,
        /// Andou além do limiar (com o botão direito: não clica mais).
        andou: bool,
        celula: (i32, i32),
    },
    Arrastando {
        x0: i32,
        y0: i32,
        celula0: (i32, i32),
    },
}

/// A máquina do ponteiro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arraste {
    estado: Estado,
    /// Onde o ponteiro esteve por último, no palco.
    ultimo: (i32, i32),
    /// Quando chegou o último evento do ponteiro.
    ultimo_ms: u64,
}

impl Default for Arraste {
    fn default() -> Arraste {
        Arraste {
            estado: Estado::Livre,
            ultimo: (0, 0),
            ultimo_ms: 0,
        }
    }
}

/// `v` arredondado para o múltiplo de `d` mais perto.
fn em_multiplos(v: i32, d: i32) -> i32 {
    let d = d.max(1);
    ((v as f64 / d as f64).round() as i32) * d
}

impl Arraste {
    /// Um evento do ponteiro, no palco. `celula`: onde a célula do pet está
    /// agora; `no_corpo`: o ponto do evento cai no corpo do pet (só importa
    /// no `Apertou`); `limiar`: o [`LIMIAR_LOGICO`] em pixels do dispositivo;
    /// `d`: pixels do dispositivo por pixel de arte.
    pub fn ponteiro(
        &mut self,
        evento: EventoPonteiro,
        agora_ms: u64,
        celula: (i32, i32),
        no_corpo: bool,
        limiar: i32,
        d: i32,
    ) -> Gesto {
        self.ultimo_ms = agora_ms;
        match evento {
            EventoPonteiro::Entrou { x, y } => {
                self.ultimo = (x, y);
                Gesto::Nada
            }
            EventoPonteiro::Saiu => {
                // A pegada implícita só manda o `leave` depois de soltar: um
                // `leave` com o botão apertado (sem arrastar ainda) desiste do
                // clique. Arrastando, quem decide é o soltar ou o fail-safe.
                if matches!(self.estado, Estado::Apertado { .. }) {
                    self.estado = Estado::Livre;
                }
                Gesto::Nada
            }
            EventoPonteiro::Apertou { botao, x, y } => {
                self.ultimo = (x, y);
                if self.estado != Estado::Livre || !no_corpo {
                    return Gesto::Nada;
                }
                if !matches!(botao, Botao::Esquerdo | Botao::Direito) {
                    return Gesto::Nada;
                }
                self.estado = Estado::Apertado {
                    botao,
                    x0: x,
                    y0: y,
                    desde_ms: agora_ms,
                    andou: false,
                    celula,
                };
                Gesto::Apertou
            }
            EventoPonteiro::Moveu { x, y } => {
                self.ultimo = (x, y);
                match &mut self.estado {
                    Estado::Apertado {
                        botao,
                        x0,
                        y0,
                        andou,
                        celula,
                        ..
                    } => {
                        let (dx, dy) = ((x - *x0) as i64, (y - *y0) as i64);
                        let limiar = limiar.max(1) as i64;
                        if dx * dx + dy * dy <= limiar * limiar {
                            return Gesto::Nada;
                        }
                        *andou = true;
                        if *botao != Botao::Esquerdo {
                            return Gesto::Nada;
                        }
                        self.estado = Estado::Arrastando {
                            x0: *x0,
                            y0: *y0,
                            celula0: *celula,
                        };
                        Gesto::Comecou
                    }
                    Estado::Arrastando { .. } => Gesto::Moveu,
                    Estado::Livre => Gesto::Nada,
                }
            }
            EventoPonteiro::Soltou { botao, x, y } => {
                self.ultimo = (x, y);
                match self.estado {
                    Estado::Apertado {
                        botao: apertado,
                        andou,
                        ..
                    } if apertado == botao => {
                        self.estado = Estado::Livre;
                        if andou {
                            Gesto::Nada
                        } else {
                            Gesto::Clique(botao)
                        }
                    }
                    Estado::Arrastando { .. } if botao == Botao::Esquerdo => {
                        let celula = self.alvo(d).unwrap_or(celula);
                        let pegada = self.pegada().unwrap_or_default();
                        self.estado = Estado::Livre;
                        Gesto::Soltou {
                            x,
                            y,
                            celula,
                            pegada,
                        }
                    }
                    _ => Gesto::Nada,
                }
            }
        }
    }

    /// O próximo prazo da máquina: o fim do "segurar" (o arraste começa) ou
    /// o fail-safe.
    pub fn prazo(&self) -> Option<u64> {
        match self.estado {
            Estado::Livre => None,
            Estado::Apertado {
                botao: Botao::Esquerdo,
                desde_ms,
                andou: false,
                ..
            } => Some(desde_ms + SEGURAR_MS),
            Estado::Apertado { .. } | Estado::Arrastando { .. } => {
                Some(self.ultimo_ms + SEM_PONTEIRO_MS)
            }
        }
    }

    /// Venceu o prazo: segurou o bastante (o arraste começa no lugar) ou o
    /// ponteiro sumiu (fail-safe).
    pub fn vencer(&mut self, agora_ms: u64) -> Gesto {
        let Some(prazo) = self.prazo() else {
            return Gesto::Nada;
        };
        if agora_ms < prazo {
            return Gesto::Nada;
        }
        match self.estado {
            Estado::Apertado {
                botao: Botao::Esquerdo,
                x0,
                y0,
                andou: false,
                celula,
                ..
            } => {
                self.estado = Estado::Arrastando {
                    x0,
                    y0,
                    celula0: celula,
                };
                Gesto::Comecou
            }
            Estado::Arrastando { .. } => {
                self.estado = Estado::Livre;
                Gesto::Cancelou
            }
            _ => {
                self.estado = Estado::Livre;
                Gesto::Nada
            }
        }
    }

    /// Onde a célula deve ficar enquanto arrasta (antes de prender na área):
    /// a de quando o arraste começou, mais o quanto o ponteiro andou, em
    /// múltiplos de `d`.
    pub fn alvo(&self, d: i32) -> Option<(i32, i32)> {
        match self.estado {
            Estado::Arrastando { x0, y0, celula0 } => Some((
                celula0.0 + em_multiplos(self.ultimo.0 - x0, d),
                celula0.1 + em_multiplos(self.ultimo.1 - y0, d),
            )),
            _ => None,
        }
    }

    /// Onde o ponteiro pegou o pet, relativo à célula de quando o arraste
    /// começou (para pousar com o mesmo ponto debaixo do ponteiro em outro
    /// monitor).
    pub fn pegada(&self) -> Option<(i32, i32)> {
        match self.estado {
            Estado::Arrastando { x0, y0, celula0 } => Some((x0 - celula0.0, y0 - celula0.1)),
            _ => None,
        }
    }

    pub fn arrastando(&self) -> bool {
        matches!(self.estado, Estado::Arrastando { .. })
    }

    /// Botão apertado no pet (arrastando ou ainda não).
    pub fn segurando(&self) -> bool {
        self.estado != Estado::Livre
    }

    /// Larga tudo (esconder, janela fechada, viagem). `true` se arrastava.
    pub fn cancelar(&mut self) -> bool {
        let arrastava = self.arrastando();
        self.estado = Estado::Livre;
        arrastava
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const CELULA: (i32, i32) = (1700, 950);
    /// 4 lógicos a 1,5.
    const LIMIAR: i32 = 6;

    fn apertou(botao: Botao, x: i32, y: i32) -> EventoPonteiro {
        EventoPonteiro::Apertou { botao, x, y }
    }

    fn soltou(botao: Botao, x: i32, y: i32) -> EventoPonteiro {
        EventoPonteiro::Soltou { botao, x, y }
    }

    fn moveu(x: i32, y: i32) -> EventoPonteiro {
        EventoPonteiro::Moveu { x, y }
    }

    #[test]
    fn apertar_e_soltar_parado_e_clique() {
        let mut a = Arraste::default();
        let g = a.ponteiro(
            apertou(Botao::Esquerdo, 1800, 1050),
            0,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        assert_eq!(g, Gesto::Apertou);
        assert_eq!(a.prazo(), Some(SEGURAR_MS));
        // Tremer dentro do limiar não arrasta.
        assert_eq!(
            a.ponteiro(moveu(1803, 1052), 50, CELULA, false, LIMIAR, 6),
            Gesto::Nada
        );
        let g = a.ponteiro(
            soltou(Botao::Esquerdo, 1803, 1052),
            120,
            CELULA,
            false,
            LIMIAR,
            6,
        );
        assert_eq!(g, Gesto::Clique(Botao::Esquerdo));
        assert!(!a.segurando() && a.prazo().is_none());
    }

    #[test]
    fn apertar_fora_do_corpo_nao_faz_nada() {
        let mut a = Arraste::default();
        let g = a.ponteiro(
            apertou(Botao::Esquerdo, 10, 10),
            0,
            CELULA,
            false,
            LIMIAR,
            6,
        );
        assert_eq!(g, Gesto::Nada);
        assert!(!a.segurando());
        // O botão do meio não clica nem arrasta.
        let g = a.ponteiro(apertou(Botao::Meio, 1800, 1050), 0, CELULA, true, LIMIAR, 6);
        assert_eq!(g, Gesto::Nada);
    }

    #[test]
    fn andar_alem_do_limiar_arrasta_em_multiplos_de_d() {
        let mut a = Arraste::default();
        a.ponteiro(
            apertou(Botao::Esquerdo, 1800, 1050),
            0,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        let g = a.ponteiro(moveu(1790, 1050), 30, CELULA, false, LIMIAR, 6);
        assert_eq!(g, Gesto::Comecou);
        assert!(a.arrastando());
        // 10 pixels para a esquerda com D = 6: um bloco (6), não 10.
        assert_eq!(
            a.alvo(6),
            Some((CELULA.0 - 12, CELULA.1)),
            "−10/6 = −1,67 → −2 blocos"
        );
        assert_eq!(
            a.ponteiro(moveu(1700, 1000), 40, CELULA, false, LIMIAR, 6),
            Gesto::Moveu
        );
        assert_eq!(a.alvo(6), Some((CELULA.0 - 102, CELULA.1 - 48)));
        assert_eq!(a.pegada(), Some((100, 100)));
        let g = a.ponteiro(
            soltou(Botao::Esquerdo, 1690, 1000),
            60,
            CELULA,
            false,
            LIMIAR,
            6,
        );
        assert_eq!(
            g,
            Gesto::Soltou {
                x: 1690,
                y: 1000,
                celula: (CELULA.0 - 108, CELULA.1 - 48),
                pegada: (100, 100),
            },
            "o soltar leva a célula até o ponto onde soltou"
        );
        assert!(!a.arrastando() && a.alvo(6).is_none());
    }

    #[test]
    fn segurar_250_ms_arrasta_sem_andar() {
        let mut a = Arraste::default();
        a.ponteiro(
            apertou(Botao::Esquerdo, 1800, 1050),
            1_000,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        assert_eq!(a.vencer(1_249), Gesto::Nada, "antes do prazo");
        assert_eq!(a.vencer(1_250), Gesto::Comecou);
        assert_eq!(a.alvo(6), Some(CELULA), "no lugar");
        // Soltar sem andar: solta, não clica.
        let g = a.ponteiro(
            soltou(Botao::Esquerdo, 1800, 1050),
            1_400,
            CELULA,
            false,
            LIMIAR,
            6,
        );
        assert_eq!(
            g,
            Gesto::Soltou {
                x: 1800,
                y: 1050,
                celula: CELULA,
                pegada: (100, 100),
            }
        );
    }

    #[test]
    fn botao_direito_so_clica_e_desiste_se_andar() {
        let mut a = Arraste::default();
        a.ponteiro(
            apertou(Botao::Direito, 1800, 1050),
            0,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        assert!(a.prazo().is_some(), "só o fail-safe");
        assert_eq!(a.vencer(SEGURAR_MS), Gesto::Nada, "o direito não arrasta");
        assert!(a.segurando());
        let g = a.ponteiro(
            soltou(Botao::Direito, 1800, 1050),
            300,
            CELULA,
            false,
            LIMIAR,
            6,
        );
        assert_eq!(g, Gesto::Clique(Botao::Direito));
        a.ponteiro(
            apertou(Botao::Direito, 1800, 1050),
            400,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        assert_eq!(
            a.ponteiro(moveu(1900, 1050), 450, CELULA, false, LIMIAR, 6),
            Gesto::Nada
        );
        let g = a.ponteiro(
            soltou(Botao::Direito, 1900, 1050),
            500,
            CELULA,
            false,
            LIMIAR,
            6,
        );
        assert_eq!(g, Gesto::Nada, "andou: não clica");
    }

    #[test]
    fn soltar_outro_botao_ou_sair_com_o_botao_apertado() {
        let mut a = Arraste::default();
        a.ponteiro(
            apertou(Botao::Esquerdo, 1800, 1050),
            0,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        let g = a.ponteiro(
            soltou(Botao::Direito, 1800, 1050),
            10,
            CELULA,
            false,
            LIMIAR,
            6,
        );
        assert_eq!(g, Gesto::Nada, "outro botão");
        assert_eq!(
            a.ponteiro(EventoPonteiro::Saiu, 20, CELULA, false, LIMIAR, 6),
            Gesto::Nada
        );
        assert!(!a.segurando(), "saiu antes de arrastar: desiste");
        // Arrastando, sair não solta (a pegada implícita manda o leave depois).
        a.ponteiro(
            apertou(Botao::Esquerdo, 1800, 1050),
            100,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        a.ponteiro(moveu(1700, 1050), 110, CELULA, false, LIMIAR, 6);
        a.ponteiro(EventoPonteiro::Saiu, 120, CELULA, false, LIMIAR, 6);
        assert!(a.arrastando());
        // Outro aperto durante o arraste não muda nada.
        let g = a.ponteiro(
            apertou(Botao::Direito, 1700, 1050),
            130,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        assert_eq!(g, Gesto::Nada);
    }

    #[test]
    fn fail_safe_solta_depois_de_5_s_sem_ponteiro() {
        let mut a = Arraste::default();
        a.ponteiro(
            apertou(Botao::Esquerdo, 1800, 1050),
            0,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        a.ponteiro(moveu(1700, 1050), 100, CELULA, false, LIMIAR, 6);
        assert_eq!(a.prazo(), Some(100 + SEM_PONTEIRO_MS));
        a.ponteiro(moveu(1600, 1050), 4_000, CELULA, false, LIMIAR, 6);
        assert_eq!(a.vencer(5_100), Gesto::Nada, "cada movimento adia");
        assert_eq!(a.vencer(9_000), Gesto::Cancelou);
        assert!(!a.segurando());
        // Botão direito esquecido apertado: só larga.
        a.ponteiro(
            apertou(Botao::Direito, 1800, 1050),
            10_000,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        assert_eq!(a.vencer(15_000), Gesto::Nada);
        assert!(!a.segurando());
    }

    #[test]
    fn cancelar_larga_tudo() {
        let mut a = Arraste::default();
        assert!(!a.cancelar());
        a.ponteiro(
            apertou(Botao::Esquerdo, 1800, 1050),
            0,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        assert!(!a.cancelar(), "apertado, sem arrastar");
        a.ponteiro(
            apertou(Botao::Esquerdo, 1800, 1050),
            0,
            CELULA,
            true,
            LIMIAR,
            6,
        );
        a.vencer(SEGURAR_MS);
        assert!(a.cancelar());
        assert!(!a.segurando() && a.prazo().is_none());
    }

    #[test]
    fn multiplos_de_d() {
        assert_eq!(em_multiplos(10, 6), 12);
        assert_eq!(em_multiplos(8, 6), 6);
        assert_eq!(em_multiplos(-10, 6), -12);
        assert_eq!(em_multiplos(2, 6), 0);
        assert_eq!(em_multiplos(7, 0), 7, "D nunca é 0, mas não divide por 0");
    }
}
