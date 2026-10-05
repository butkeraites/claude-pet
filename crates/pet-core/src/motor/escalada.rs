//! A escalada do "precisa de você" (decisões 0010 e 0075): só visual, com
//! teto, e só para quem não está olhando o terminal do Claude.
//!
//! Uma máquina pura, no relógio do laço, para o aviso de espera que o Motor
//! mostra (o mais velho; os outros viram o "+N"). A fase vem do tempo desde
//! o aviso; o que cada fase faz depende de o Renan precisar ser chamado
//! ([`Contexto::chama`]: não está olhando o terminal da sessão que espera, ou
//! está sem mexer há 60 s ou mais; e não viu o diálogo lá, decisão 0090):
//!
//! | Fase | Desde | Com o Renan a chamar |
//! |---|---|---|
//! | L1 | 0 s | (o Motor toca a chamada e o balão na hora) |
//! | L2 | 30 s | uma rajada a cada 6 s, até 5 |
//! | L3 | 90 s | o voo até o alto-centro e de volta, até 3, com 60 s entre eles |
//! | L4 | 5 min | o selo pulsando e uma rajada a cada 60 s, por 30 min |
//!
//! Sem precisar chamar (olhando o terminal da sessão que espera e mexendo),
//! fica em L1. Com o "não perturbe" ou a soneca ([`Contexto::teto_l1`]),
//! nunca passa de L1. Escondido ([`Contexto::visivel`] falso), nada toca, e o
//! relógio anda. Quando o Renan volta (`ocioso` de verdadeiro para falso), um
//! voo na hora, ou assim que o pet puder aparecer (a proteção de tela fecha
//! depois da volta), até [`VOLTA_VALE_MS`] depois; os voos da volta têm a
//! conta deles ([`VOLTAS_MAX`]), fora dos da L3 (decisão 0090). Um prazo que
//! esta máquina devolve sempre muda alguma coisa quando vence, ou é futuro:
//! nunca um prazo vencido que fica.

/// A L2 começa aqui.
pub const L2_APOS_MS: u64 = 30_000;
/// Entre as rajadas da L2.
pub const RAJADA_L2_CADA_MS: u64 = 6_000;
/// Rajadas da L2 (30 s).
pub const RAJADAS_L2: u8 = 5;
/// A L3 começa aqui.
pub const L3_APOS_MS: u64 = 90_000;
/// Voos por aviso.
pub const VOOS_MAX: u8 = 3;
/// Entre dois voos.
pub const VOO_CADA_MS: u64 = 60_000;
/// O teto: L4 daqui em diante.
pub const L4_APOS_MS: u64 = 5 * 60 * 1000;
/// Entre as rajadas da L4.
pub const RAJADA_L4_CADA_MS: u64 = 60_000;
/// A L4 pulsa e rajada por isto; depois, o selo parado.
pub const L4_DURA_MS: u64 = 30 * 60 * 1000;
/// Sem mexer por isto, o Renan precisa ser chamado mesmo olhando o terminal.
pub const PARADO_PARA_CHAMAR_MS: u64 = 60_000;
/// Voos da volta do Renan por aviso, fora dos [`VOOS_MAX`] da L3: os da L3
/// podem ter saído com ele longe (decisão 0090).
pub const VOLTAS_MAX: u8 = 3;
/// A volta com o pet fora da tela (a proteção de tela ainda aberta) espera
/// por isto o pet poder aparecer.
pub const VOLTA_VALE_MS: u64 = 2 * 60 * 1000;

/// O que o Motor sabe agora (decisão 0075).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Contexto {
    /// O Renan precisa ser chamado: não está olhando um terminal do Claude,
    /// ou está sem mexer há [`PARADO_PARA_CHAMAR_MS`] ou mais.
    pub chama: bool,
    /// Quando ele passa a precisar (o sem mexer chegando aos 60 s), se ainda
    /// não precisa.
    pub chama_em: Option<u64>,
    /// O "não perturbe" ou a soneca: nunca acima de L1.
    pub teto_l1: bool,
    /// O pet pode aparecer (não escondido, sem a proteção de tela).
    pub visivel: bool,
}

/// O que a escalada pede.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Passo {
    /// O nível mudou (1 a 4).
    Nivel(u8),
    /// Uma rajada (a chamada tocada de novo).
    Rajada,
    /// O voo até o alto-centro do monitor e de volta; `volta`: porque o
    /// Renan voltou.
    Voo { volta: bool },
    /// O selo pulsando (uma troca de cor por segundo; decisão 0083) liga ou
    /// desliga.
    Pulso(bool),
}

/// A escalada de um aviso de espera.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Escalada {
    /// Quando o aviso abriu (relógio do laço).
    pub desde: u64,
    /// O nível anunciado.
    pub nivel: u8,
    rajadas_l2: u8,
    proxima_l2: Option<u64>,
    pub voos: u8,
    ultimo_voo: Option<u64>,
    proxima_l4: Option<u64>,
    pub pulso: bool,
    /// Os voos da volta do Renan.
    pub voltas: u8,
    /// A volta que espera o pet poder aparecer, até este instante.
    volta_pendente: Option<u64>,
}

impl Escalada {
    /// A escalada de um aviso aberto em `desde`, em L1.
    pub fn nova(desde: u64) -> Escalada {
        Escalada {
            desde,
            nivel: 1,
            rajadas_l2: 0,
            proxima_l2: None,
            voos: 0,
            ultimo_voo: None,
            proxima_l4: None,
            pulso: false,
            voltas: 0,
            volta_pendente: None,
        }
    }

    /// O voo da volta, agora; o próximo voo da L3 espera os 60 s dele.
    fn voar_na_volta(&mut self, agora: u64, passos: &mut Vec<Passo>) {
        self.volta_pendente = None;
        self.subir(3, passos);
        passos.push(Passo::Voo { volta: true });
        self.voltas += 1;
        self.ultimo_voo = Some(agora);
    }

    fn fase(&self, agora: u64) -> u8 {
        let e = agora.saturating_sub(self.desde);
        if e >= L4_APOS_MS {
            4
        } else if e >= L3_APOS_MS {
            3
        } else if e >= L2_APOS_MS {
            2
        } else {
            1
        }
    }

    fn fim_l4(&self) -> u64 {
        self.desde + L4_APOS_MS + L4_DURA_MS
    }

    fn subir(&mut self, nivel: u8, passos: &mut Vec<Passo>) {
        if nivel > self.nivel {
            self.nivel = nivel;
            passos.push(Passo::Nivel(nivel));
        }
    }

    fn pulsar(&mut self, ligado: bool, passos: &mut Vec<Passo>) {
        if self.pulso != ligado {
            self.pulso = ligado;
            passos.push(Passo::Pulso(ligado));
        }
    }

    /// O que vence agora.
    pub fn vencer(&mut self, agora: u64, ctx: Contexto) -> Vec<Passo> {
        let mut passos = Vec::new();
        // A volta que esperava o pet aparecer (a proteção de tela fechou), ou
        // que acabou o prazo, ou que o "não perturbe" e a soneca cancelam.
        if let Some(ate) = self.volta_pendente {
            if agora >= ate || ctx.teto_l1 {
                self.volta_pendente = None;
            } else if ctx.visivel {
                self.voar_na_volta(agora, &mut passos);
            }
        }
        let fase = self.fase(agora);
        let pode = ctx.chama && ctx.visivel && !ctx.teto_l1;
        // O selo só pulsa na L4, com o Renan a chamar, nos 30 min.
        let pulsa = pode && fase == 4 && agora < self.fim_l4();
        self.pulsar(pulsa, &mut passos);
        if !pode {
            return passos;
        }
        match fase {
            2 if self.rajadas_l2 < RAJADAS_L2 => {
                self.subir(2, &mut passos);
                if self.proxima_l2.is_none_or(|p| p <= agora) {
                    passos.push(Passo::Rajada);
                    self.rajadas_l2 += 1;
                    self.proxima_l2 = Some(agora + RAJADA_L2_CADA_MS);
                }
            }
            3 if self.voos < VOOS_MAX => {
                if self.ultimo_voo.is_none_or(|u| agora >= u + VOO_CADA_MS) {
                    self.subir(3, &mut passos);
                    passos.push(Passo::Voo { volta: false });
                    self.voos += 1;
                    self.ultimo_voo = Some(agora);
                }
            }
            4 if agora < self.fim_l4() => {
                self.subir(4, &mut passos);
                if self.proxima_l4.is_none_or(|p| p <= agora) {
                    passos.push(Passo::Rajada);
                    self.proxima_l4 = Some(agora + RAJADA_L4_CADA_MS);
                }
            }
            _ => {}
        }
        passos
    }

    /// O Renan voltou ao teclado ou ao mouse com o aviso de pé: um voo na
    /// hora (decisão 0075), com a conta dos voos da volta (decisão 0090). Com
    /// o pet fora da tela (a proteção de tela fecha logo depois da volta), o
    /// voo espera até [`VOLTA_VALE_MS`] o pet poder aparecer.
    pub fn voltou(&mut self, agora: u64, ctx: Contexto) -> Vec<Passo> {
        let mut passos = Vec::new();
        if ctx.teto_l1 || self.voltas >= VOLTAS_MAX {
            return passos;
        }
        if !ctx.visivel {
            self.volta_pendente = Some(agora + VOLTA_VALE_MS);
            return passos;
        }
        self.voar_na_volta(agora, &mut passos);
        passos
    }

    /// Quando chamar [`Self::vencer`] de novo: um instante em que algo muda.
    /// Olhado de novo nesse instante, ainda é ele (o Motor recalcula o prazo
    /// na hora antes de vencer). Sem poder chamar, só o pulso que desliga e a
    /// hora em que o Renan passa a precisar ser chamado; o resto (o "não
    /// perturbe", a soneca, esconder) muda com uma chamada ao Motor, que olha
    /// de novo.
    pub fn proximo(&self, agora: u64, ctx: Contexto) -> Option<u64> {
        let pode = ctx.chama && ctx.visivel && !ctx.teto_l1;
        let fase = self.fase(agora);
        let mut c: Vec<u64> = Vec::new();
        // A volta pendente voa (ou sai) já com o pet na tela ou um teto;
        // senão, no fim do prazo dela.
        if let Some(ate) = self.volta_pendente {
            c.push(if ctx.visivel || ctx.teto_l1 {
                agora
            } else {
                ate
            });
        }
        // O pulso muda já: o fim da L4, o Renan olhou o terminal, o "não
        // perturbe" ligou.
        if self.pulso != (pode && fase == 4 && agora < self.fim_l4()) {
            c.push(agora);
        }
        if pode {
            // A próxima fase (o nível e o pulso mudam nela).
            for limite in [L2_APOS_MS, L3_APOS_MS, L4_APOS_MS] {
                if agora < self.desde + limite {
                    c.push(self.desde + limite);
                    break;
                }
            }
            if fase == 4 && agora < self.fim_l4() {
                c.push(self.fim_l4());
            }
            match fase {
                2 if self.rajadas_l2 < RAJADAS_L2 => c.push(self.proxima_l2.unwrap_or(agora)),
                3 if self.voos < VOOS_MAX => {
                    c.push(self.ultimo_voo.map_or(agora, |u| u + VOO_CADA_MS));
                }
                4 if agora < self.fim_l4() => c.push(self.proxima_l4.unwrap_or(agora)),
                _ => {}
            }
        } else if let Some(t) = ctx.chama_em.filter(|t| *t > agora) {
            c.push(t);
        }
        c.into_iter().min().map(|p| p.max(agora))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const CHAMA: Contexto = Contexto {
        chama: true,
        chama_em: None,
        teto_l1: false,
        visivel: true,
    };

    /// Roda a escalada seguindo os prazos até `ate`, com o contexto fixo,
    /// como o Motor: o prazo é recalculado na hora em que vence, e só então
    /// a escalada vence. Devolve (instante, passo).
    fn rodar(e: &mut Escalada, de: u64, ate: u64, ctx: Contexto) -> Vec<(u64, Passo)> {
        let mut saida = Vec::new();
        let mut agora = de;
        let mut voltas = 0;
        while let Some(p) = e.proximo(agora, ctx).filter(|p| *p <= ate) {
            voltas += 1;
            assert!(voltas < 10_000, "prazo que não anda em {p}");
            agora = p;
            let devido = e.proximo(agora, ctx) == Some(agora);
            let antes = e.clone();
            let passos = e.vencer(agora, ctx);
            assert!(
                passos.is_empty() || devido,
                "em {agora} a escalada tinha {passos:?}, mas o prazo olhado na hora não venceu"
            );
            assert!(
                *e != antes || e.proximo(agora, ctx).is_none_or(|q| q > agora),
                "o prazo {p} venceu sem mudar nada e continua armado"
            );
            for passo in passos {
                saida.push((agora, passo));
            }
        }
        saida
    }

    #[test]
    fn escala_ate_o_teto_e_para_em_30_min() {
        let mut e = Escalada::nova(0);
        let saida = rodar(&mut e, 0, 2 * 60 * 60 * 1000, CHAMA);
        let rajadas_l2: Vec<u64> = saida
            .iter()
            .filter(|(t, p)| *p == Passo::Rajada && *t < L3_APOS_MS)
            .map(|(t, _)| *t)
            .collect();
        assert_eq!(rajadas_l2, vec![30_000, 36_000, 42_000, 48_000, 54_000]);
        let voos: Vec<u64> = saida
            .iter()
            .filter(|(_, p)| matches!(p, Passo::Voo { .. }))
            .map(|(t, _)| *t)
            .collect();
        assert_eq!(voos, vec![90_000, 150_000, 210_000], "até 3, de 60 em 60 s");
        let niveis: Vec<(u64, u8)> = saida
            .iter()
            .filter_map(|(t, p)| match p {
                Passo::Nivel(n) => Some((*t, *n)),
                _ => None,
            })
            .collect();
        assert_eq!(niveis, vec![(30_000, 2), (90_000, 3), (300_000, 4)]);
        let pulsos: Vec<(u64, bool)> = saida
            .iter()
            .filter_map(|(t, p)| match p {
                Passo::Pulso(l) => Some((*t, *l)),
                _ => None,
            })
            .collect();
        assert_eq!(
            pulsos,
            vec![(300_000, true), (L4_APOS_MS + L4_DURA_MS, false)]
        );
        let rajadas_l4 = saida
            .iter()
            .filter(|(t, p)| *p == Passo::Rajada && *t >= L4_APOS_MS)
            .count();
        assert_eq!(rajadas_l4, 30, "uma por minuto, por 30 min");
        assert_eq!(e.proximo(L4_APOS_MS + L4_DURA_MS, CHAMA), None, "parado");
    }

    #[test]
    fn olhando_o_terminal_e_mexendo_fica_em_l1() {
        let mut e = Escalada::nova(0);
        let olhando = Contexto {
            chama: false,
            ..CHAMA
        };
        let saida = rodar(&mut e, 0, 60 * 60 * 1000, olhando);
        assert!(saida.is_empty(), "{saida:?}");
        assert_eq!(e.nivel, 1);
        // Sem mexer: chama aos 60 s parado.
        let mut e = Escalada::nova(0);
        let parado = Contexto {
            chama: false,
            chama_em: Some(70_000),
            ..CHAMA
        };
        assert_eq!(e.proximo(31_000, parado), Some(70_000));
        assert_eq!(
            e.vencer(70_000, CHAMA),
            vec![Passo::Nivel(2), Passo::Rajada]
        );
    }

    #[test]
    fn nao_perturbe_e_soneca_nunca_passam_de_l1() {
        let mut e = Escalada::nova(0);
        let quieto = Contexto {
            teto_l1: true,
            ..CHAMA
        };
        assert!(rodar(&mut e, 0, 60 * 60 * 1000, quieto).is_empty());
        assert!(e.voltou(10_000, quieto).is_empty(), "nem o voo da volta");
        // Ligado no meio da L4: o pulso desliga na hora.
        let mut e = Escalada::nova(0);
        rodar(&mut e, 0, 400_000, CHAMA);
        assert!(e.pulso);
        assert_eq!(e.proximo(400_000, quieto), Some(400_000));
        assert_eq!(e.vencer(400_000, quieto), vec![Passo::Pulso(false)]);
        assert_eq!(e.proximo(400_000, quieto), None, "nada muda mais sozinho");
    }

    #[test]
    fn a_volta_do_renan_voa_na_hora_ate_3_vezes() {
        let mut e = Escalada::nova(0);
        assert_eq!(
            e.voltou(10_000, CHAMA),
            vec![Passo::Nivel(3), Passo::Voo { volta: true }]
        );
        assert_eq!(e.voltou(20_000, CHAMA), vec![Passo::Voo { volta: true }]);
        assert_eq!(e.voltou(25_000, CHAMA), vec![Passo::Voo { volta: true }]);
        assert!(e.voltou(26_000, CHAMA).is_empty(), "acabaram os da volta");
        // Os voos da L3 têm a conta deles (decisão 0090): os três saem, o
        // primeiro 60 s depois do último da volta.
        let saida = rodar(&mut e, 26_000, 299_999, CHAMA);
        let voos: Vec<u64> = saida
            .iter()
            .filter(|(_, p)| matches!(p, Passo::Voo { volta: false }))
            .map(|(t, _)| *t)
            .collect();
        assert_eq!(voos, vec![90_000, 150_000, 210_000]);
        // E os da L3 que saíram com o Renan longe não gastam os da volta.
        let mut e = Escalada::nova(0);
        rodar(&mut e, 0, 299_999, CHAMA);
        assert_eq!(e.voos, VOOS_MAX);
        assert_eq!(e.voltou(400_000, CHAMA), vec![Passo::Voo { volta: true }]);
        // Escondido, nada toca; o relógio anda.
        let mut e = Escalada::nova(0);
        let escondido = Contexto {
            visivel: false,
            ..CHAMA
        };
        assert!(rodar(&mut e, 0, 400_000, escondido).is_empty());
    }

    #[test]
    fn a_volta_com_a_protecao_de_tela_espera_o_pet_aparecer() {
        // O primeiro toque acorda o Renan, mas a proteção de tela só fecha
        // depois: o voo da volta sai quando o pet pode aparecer (decisão
        // 0090), dentro do prazo.
        let escondido = Contexto {
            visivel: false,
            ..CHAMA
        };
        let mut e = Escalada::nova(0);
        rodar(&mut e, 0, 400_000, escondido);
        assert!(e.voltou(400_000, escondido).is_empty());
        assert_eq!(e.proximo(400_000, escondido), Some(400_000 + VOLTA_VALE_MS));
        assert_eq!(e.proximo(401_500, CHAMA), Some(401_500), "o pet apareceu");
        assert_eq!(
            e.vencer(401_500, CHAMA),
            vec![
                Passo::Nivel(3),
                Passo::Voo { volta: true },
                Passo::Pulso(true),
                Passo::Nivel(4),
                Passo::Rajada
            ]
        );
        // Sem aparecer no prazo, a volta sai sem voo; o "não perturbe"
        // também a cancela.
        let mut e = Escalada::nova(0);
        assert!(e.voltou(10_000, escondido).is_empty());
        assert!(e.vencer(10_000 + VOLTA_VALE_MS, escondido).is_empty());
        assert!(
            rodar(
                &mut e,
                10_000 + VOLTA_VALE_MS,
                20_000 + VOLTA_VALE_MS,
                CHAMA
            )
            .iter()
            .all(|(_, p)| !matches!(p, Passo::Voo { volta: true }))
        );
        let mut e = Escalada::nova(0);
        assert!(e.voltou(10_000, escondido).is_empty());
        let quieto = Contexto {
            teto_l1: true,
            ..CHAMA
        };
        assert!(e.vencer(11_000, quieto).is_empty());
        assert_eq!(e.proximo(11_000, CHAMA), Some(30_000), "só a L2");
    }
}
