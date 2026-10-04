//! O Motor com relógio falso e uma janela falsa: nada de compositor.

use std::path::PathBuf;
use std::rc::Rc;

use super::*;
use crate::cerebro::Nivel;
use crate::plataforma::{CapOverlay, InfoOverlay, UltimoQuadro};

const PAREDE: u64 = 1_790_000_000_000;

fn skin_teste() -> Rc<Skin> {
    let pasta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skins/_teste");
    Rc::new(Skin::carregar(&pasta).expect("skins/_teste"))
}

fn edp() -> Monitor {
    Monitor {
        nome: Some("eDP-1".into()),
        logico: (1280, 800),
        escala: 1.5,
    }
}

/// Uma janela de mentira: guarda o que o Motor pediu e imita o ritmo de um
/// quadro em voo por vez.
#[derive(Default)]
struct Falsa {
    fase: Option<Fase>,
    pronta: Option<Monitor>,
    em_voo: bool,
    cena: Option<Vec<Elemento>>,
    regiao: Option<Ret>,
    seq: u64,
    pedidos: Vec<String>,
    eventos: Vec<EventoOverlay>,
}

impl Falsa {
    fn fase_atual(&self) -> Fase {
        self.fase.unwrap_or(Fase::Ausente)
    }

    /// O compositor mostrou o quadro em voo.
    fn mostrou(&mut self) {
        self.em_voo = false;
    }

    fn quadros(&self) -> usize {
        self.pedidos
            .iter()
            .filter(|p| p.starts_with("quadro"))
            .count()
    }
}

impl Overlay for Falsa {
    fn capacidades(&self) -> CapOverlay {
        CapOverlay::default()
    }

    fn fase(&self) -> Fase {
        self.fase_atual()
    }

    fn pronta(&self) -> Option<Monitor> {
        self.pronta.clone()
    }

    fn criar(&mut self) {
        self.pedidos.push("criar".into());
        self.fase = Some(Fase::Viva { conteudo: false });
        self.cena = None;
        self.em_voo = false;
    }

    fn cancelar_saida(&mut self) {
        self.pedidos.push("cancelar".into());
        self.fase = Some(Fase::Viva { conteudo: true });
    }

    fn apagar_e_destruir(&mut self, skin: &Skin) -> bool {
        self.pedidos.push(format!("apagar com {}", skin.id));
        self.fase = Some(Fase::Saindo);
        self.cena = Some(Vec::new());
        self.regiao = None;
        true
    }

    fn destruir(&mut self) {
        self.pedidos.push("destruir".into());
        self.fase = None;
        self.cena = None;
    }

    fn desenhar(
        &mut self,
        cena: &[Elemento],
        _skin: &Skin,
        regiao: Option<Ret>,
        forcar: bool,
    ) -> Result<Desenho, String> {
        if self.pronta.is_none() {
            return Err("janela ainda não está pronta".into());
        }
        let mudou_regiao = self.regiao != regiao;
        self.regiao = regiao;
        if self.cena.as_deref() == Some(cena) {
            return Ok(if mudou_regiao {
                Desenho::SoEstado
            } else {
                Desenho::SemMudanca
            });
        }
        if self.em_voo && !forcar {
            return Ok(Desenho::Adiado);
        }
        self.cena = Some(cena.to_vec());
        self.em_voo = true;
        self.seq += 1;
        self.fase = Some(Fase::Viva {
            conteudo: !cena.is_empty(),
        });
        self.pedidos.push(format!("quadro {}", self.seq));
        Ok(Desenho::Enviado {
            retangulos: 1,
            area: 0,
        })
    }

    fn esquecer_cena(&mut self) {
        self.pedidos.push("esquecer".into());
        self.cena = None;
    }

    fn info(&self) -> InfoOverlay {
        InfoOverlay {
            monitor: self.pronta.as_ref().and_then(|m| m.nome.clone()),
            escala: self.pronta.as_ref().map(|m| m.escala),
            regiao: self.regiao,
            visivel: self.fase_atual() == Fase::Viva { conteudo: true },
            shm_bytes: 9_216_000,
        }
    }

    fn ultimo_quadro(&self) -> Option<UltimoQuadro> {
        if !matches!(self.fase_atual(), Fase::Viva { .. }) {
            return None;
        }
        let pronta = self.pronta.as_ref()?;
        Some(UltimoQuadro {
            monitor: pronta.nome.clone().unwrap_or_default(),
            cena: self.cena.clone()?,
            seq: self.seq,
            idade_ms: 0,
        })
    }

    fn proximo_prazo(&self) -> Option<u64> {
        None
    }

    fn vencer(&mut self, _: u64) {}

    fn eventos(&mut self) -> Vec<EventoOverlay> {
        std::mem::take(&mut self.eventos)
    }

    fn encerrar(&mut self, confirmar: bool) {
        self.pedidos.push(format!("encerrar {confirmar}"));
        self.fase = None;
    }
}

/// Um Motor com a skin de teste, a janela criada e pronta no eDP-1, e o
/// primeiro quadro desenhado em `t = 0`.
fn ligado() -> (Motor, Falsa) {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.definir_skin(Some(skin_teste()));
    let mut janela = Falsa::default();
    motor.conectou(0);
    motor.aplicar_visibilidade(&mut janela, 0);
    assert_eq!(janela.pedidos, vec!["criar"]);
    janela.pronta = Some(edp());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta(edp()), 0);
    (motor, janela)
}

#[test]
fn janela_pronta_monta_o_palco_desenha_e_marca_a_proxima_troca() {
    let (mut motor, janela) = ligado();
    assert_eq!(janela.pedidos, vec!["criar", "quadro 1"]);
    let prazo = motor.prazo_da_animacao().expect("o repouso tem rajadas");
    assert!(prazo > 0, "{prazo}");
    assert_eq!(motor.proximo_prazo(), Some(prazo), "cérebro sem prazo");
    let painel = motor.painel(Some(&janela), 10);
    assert_eq!(painel.monitor.as_deref(), Some("eDP-1"));
    assert_eq!(painel.d, Some(5), "D do _teste no eDP-1 (decisão 0017)");
    assert_eq!(
        painel.sprite_disp,
        Some(Ret::novo(1706, 951, 214, 240)),
        "célula no canto inferior direito"
    );
    assert_eq!(painel.regiao_entrada, Some(Ret::novo(1170, 677, 94, 107)));
    assert_eq!((painel.commits_total, painel.commits_por_min), (1, 1));
    assert!(painel.visivel && !painel.estresse);
}

#[test]
fn quadro_em_voo_adia_sem_marcar_prazo_e_o_redesenhar_retoma() {
    let (mut motor, mut janela) = ligado();
    // Uma reação troca o quadro já: com o primeiro ainda em voo, adia.
    assert!(motor.tocar(Some(&mut janela), "done_small", 100));
    assert_eq!(janela.quadros(), 1, "adiado");
    let painel = motor.painel(Some(&janela), 100);
    assert_eq!(painel.reacao.as_deref(), Some("done_small"));
    // O compositor mostra o quadro e a janela pede o próximo.
    janela.mostrou();
    motor.evento_overlay(&mut janela, EventoOverlay::Redesenhar, 140);
    assert_eq!(janela.quadros(), 2);
    let prazo = motor.prazo_da_animacao().unwrap();
    assert_eq!(prazo, 300, "done_small do _teste: quadro de 200 ms");
    // O prazo vence: o quadro seguinte vai quando o anterior foi mostrado.
    janela.mostrou();
    motor.vencer_animacao(&mut janela, prazo - 1);
    assert_eq!(janela.quadros(), 2, "antes do prazo nada acontece");
    motor.vencer_animacao(&mut janela, prazo);
    assert_eq!(janela.quadros(), 3);
}

#[test]
fn esconder_apaga_com_a_skin_e_mostrar_cancela_a_saida() {
    let (mut motor, mut janela) = ligado();
    motor.estresse(&mut janela, 30, 5, 50);
    motor.definir_visivel(false);
    motor.aplicar_visibilidade(&mut janela, 60);
    assert_eq!(janela.pedidos.last().unwrap(), "apagar com _teste");
    assert_eq!(janela.fase(), Fase::Saindo);
    assert_eq!(motor.prazo_da_animacao(), None, "parado ao esconder");
    let painel = motor.painel(Some(&janela), 70);
    assert!(!painel.visivel && !painel.estresse && painel.d.is_none());
    // Mostrar antes de a janela morrer cancela a saída e força o quadro,
    // mesmo com um em voo (decisão 0018).
    janela.em_voo = true;
    motor.definir_visivel(true);
    motor.aplicar_visibilidade(&mut janela, 80);
    let fim: Vec<&str> = janela
        .pedidos
        .iter()
        .rev()
        .take(2)
        .map(String::as_str)
        .collect();
    assert_eq!(fim, vec!["quadro 2", "cancelar"]);
    assert!(motor.painel(Some(&janela), 90).visivel);
}

#[test]
fn esconder_sem_nada_desenhado_destroi_ja() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.definir_skin(Some(skin_teste()));
    let mut janela = Falsa::default();
    motor.conectou(0);
    motor.aplicar_visibilidade(&mut janela, 0);
    motor.definir_visivel(false);
    motor.aplicar_visibilidade(&mut janela, 10);
    assert_eq!(janela.pedidos, vec!["criar", "destruir"]);
    assert_eq!(janela.fase(), Fase::Ausente);
}

#[test]
fn revogar_esconde_com_a_skin_velha_e_larga_o_pet() {
    let (mut motor, mut janela) = ligado();
    motor.trocar_skin(&mut janela, None, 100);
    assert_eq!(janela.pedidos.last().unwrap(), "apagar com _teste");
    assert!(!motor.quer_mostrar() && motor.skin().is_none());
    assert_eq!(
        motor.tocar_comando(Some(&mut janela), "nod", 110),
        Tocou::SemPersonagem
    );
}

#[test]
fn skin_nova_esquece_a_cena_refaz_o_palco_e_redesenha_tudo() {
    let (mut motor, mut janela) = ligado();
    let mut outra = (*skin_teste()).clone();
    outra.id = "outra".into();
    janela.em_voo = true;
    motor.trocar_skin(&mut janela, Some(Rc::new(outra)), 200);
    let fim: Vec<&str> = janela
        .pedidos
        .iter()
        .rev()
        .take(2)
        .map(String::as_str)
        .collect();
    assert_eq!(
        fim,
        vec!["quadro 2", "esquecer"],
        "forçado, com o quadro em voo"
    );
    assert_eq!(motor.skin().map(|s| s.id.as_str()), Some("outra"));
    assert_eq!(motor.painel(Some(&janela), 210).d, Some(5));
}

#[test]
fn skin_nova_sem_janela_pronta_espera_o_palco() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    let mut janela = Falsa::default();
    motor.conectou(0);
    motor.aplicar_visibilidade(&mut janela, 0);
    assert!(janela.pedidos.is_empty(), "sem personagem, nada de janela");
    motor.trocar_skin(&mut janela, Some(skin_teste()), 10);
    assert_eq!(janela.pedidos, vec!["esquecer", "criar"]);
    assert!(motor.painel(Some(&janela), 20).d.is_none(), "palco espera");
    janela.pronta = Some(edp());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta(edp()), 30);
    assert_eq!(janela.quadros(), 1);
}

#[test]
fn tocar_responde_o_que_fez() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    assert_eq!(motor.tocar_comando(None, "nod", 0), Tocou::SemPersonagem);
    motor.definir_skin(Some(skin_teste()));
    assert_eq!(
        motor.tocar_comando(None, "nada_disso", 0),
        Tocou::Desconhecida {
            skin: "_teste".into()
        }
    );
    // Sem compositor: a skin sabe tocar, mas não há onde.
    assert_eq!(
        motor.tocar_comando(None, "nod", 0),
        Tocou::ForaDaTela {
            tag: "wave".into(),
            motivo: "sem compositor"
        },
        "na _teste o aceno cai no wave (decisão 0030)"
    );
    let (mut motor, mut janela) = ligado();
    assert_eq!(
        motor.tocar_comando(Some(&mut janela), "done_small", 10),
        Tocou::NaTela {
            tag: "done_small".into()
        }
    );
    motor.definir_visivel(false);
    assert!(matches!(
        motor.tocar_comando(Some(&mut janela), "nod", 20),
        Tocou::ForaDaTela {
            motivo: "o pet está escondido (bin/pet mostrar)",
            ..
        }
    ));
}

#[test]
fn cerebro_acena_no_prazo_e_o_pet_toca() {
    let (mut motor, mut janela) = ligado();
    let agora = |mono_ms: u64| Agora {
        parede_ms: PAREDE + mono_ms,
        mono_ms,
    };
    let evento = |e: &str| Evento {
        e: e.into(),
        sid: Some("0123456789abcdef".into()),
        turno: Some("p1".into()),
        ent: Some("cli".into()),
        ts: None,
        ..Evento::default()
    };
    assert!(
        motor
            .evento(&evento("UserPromptSubmit"), PAREDE + 1_000, agora(1_000))
            .is_empty()
    );
    assert!(
        motor
            .evento(&evento("Stop"), PAREDE + 2_000, agora(2_000))
            .is_empty()
    );
    let animacao = motor.prazo_da_animacao().unwrap();
    assert_eq!(motor.prazo_do_cerebro(), Some(2_800), "acomodação de 0,8 s");
    assert_eq!(motor.proximo_prazo(), Some(animacao.min(2_800)));
    let reacoes = motor.tique(agora(2_800));
    assert_eq!(reacoes.len(), 1);
    assert_eq!(
        (reacoes[0].nome, reacoes[0].nivel),
        ("nod", Some(Nivel::T0))
    );
    janela.mostrou();
    assert!(motor.tocar(Some(&mut janela), reacoes[0].nome, 2_800));
    assert_eq!(
        motor.painel(Some(&janela), 2_810).reacao.as_deref(),
        Some("nod")
    );
    assert_eq!(motor.resumo().turnos.len(), 1);
}

#[test]
fn janela_fechada_perde_o_palco_e_recriar_so_se_o_pet_deve_aparecer() {
    let (mut motor, mut janela) = ligado();
    janela.fase = None;
    motor.evento_overlay(&mut janela, EventoOverlay::Sumiu, 100);
    assert_eq!(motor.prazo_da_animacao(), None);
    assert!(motor.painel(Some(&janela), 110).d.is_none());
    motor.definir_visivel(false);
    motor.evento_overlay(&mut janela, EventoOverlay::Recriar, 350);
    assert_eq!(
        janela.pedidos.last().unwrap(),
        "quadro 1",
        "escondido: não recria"
    );
    motor.definir_visivel(true);
    motor.evento_overlay(&mut janela, EventoOverlay::Recriar, 360);
    assert_eq!(janela.pedidos.last().unwrap(), "criar");
}

#[test]
fn painel_sem_janela_e_vazio_e_desconectar_zera() {
    let (mut motor, janela) = ligado();
    assert_eq!(motor.painel(None, 10), Painel::default());
    assert_eq!(motor.painel(Some(&janela), 10).commits_total, 1);
    motor.desconectou();
    assert!(!motor.quer_mostrar());
    assert_eq!(motor.proximo_prazo(), None);
    let mut nova = Falsa::default();
    motor.conectou(500);
    assert_eq!(
        motor.painel(Some(&nova), 510).commits_total,
        0,
        "sessão nova"
    );
    motor.aplicar_visibilidade(&mut nova, 510);
    assert_eq!(nova.pedidos, vec!["criar"]);
}

#[test]
fn quadro_esperado_e_o_ultimo_sprite_na_tela() {
    let (motor, janela) = ligado();
    let q = motor.quadro_esperado(&janela).expect("pet na tela");
    assert_eq!(q.monitor, "eDP-1");
    assert_eq!(q.area, Ret::novo(1706, 951, 214, 240));
    assert_eq!((q.d, q.seq), (5, 1));
    assert_eq!(q.grade.0 + 38 * 5, 1896, "canto da célula");
    assert_eq!(q.rgba.len(), (q.area.w * q.area.h * 4) as usize);
    assert!(motor.quadro_esperado(&Falsa::default()).is_none());
}

#[test]
fn estresse_poe_confete_na_cena_e_acaba_pelo_relogio() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    motor.estresse(&mut janela, 20, 1, 1_000);
    assert!(janela.cena.as_ref().unwrap().len() > 1, "pet e confetes");
    assert_eq!(
        motor.prazo_da_animacao(),
        Some(1_050),
        "passo de 50 ms a 20 fps"
    );
    assert!(motor.painel(Some(&janela), 1_500).estresse);
    assert!(
        !motor.painel(Some(&janela), 2_000).estresse,
        "acabou no relógio"
    );
    janela.mostrou();
    motor.vencer_animacao(&mut janela, 2_000);
    assert_eq!(janela.cena.as_ref().unwrap().len(), 1, "só o pet de novo");
}

#[test]
fn estresse_sem_palco_avisa_e_nao_comeca() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    let mut janela = Falsa::default();
    motor.estresse(&mut janela, 30, 1, 0);
    assert!(!motor.painel(Some(&janela), 10).estresse);
}

#[test]
fn encerrar_esconde_e_espera_a_confirmacao() {
    let (mut motor, mut janela) = ligado();
    motor.encerrar(&mut janela, 100);
    assert_eq!(
        janela.pedidos[janela.pedidos.len() - 2..],
        ["apagar com _teste".to_owned(), "encerrar true".to_owned()]
    );
    let mut vazia = Falsa::default();
    motor.encerrar(&mut vazia, 200);
    assert_eq!(vazia.pedidos, vec!["encerrar false"]);
}

#[test]
fn tamanho_pequeno_e_grande_mudam_o_d_sem_mudar_a_skin() {
    let (mut motor, mut janela) = ligado();
    assert_eq!(motor.painel(Some(&janela), 10).d, Some(5), "normal");
    janela.em_voo = true;
    motor.definir_tamanho(Tamanho::Pequeno, Some(&mut janela), 20);
    let fim: Vec<&str> = janela
        .pedidos
        .iter()
        .rev()
        .take(2)
        .map(String::as_str)
        .collect();
    assert_eq!(fim, vec!["quadro 2", "esquecer"], "redesenha tudo, já");
    let painel = motor.painel(Some(&janela), 30);
    assert_eq!(painel.d, Some(4), "_teste no eDP-1: 3,75 → 4");
    // Célula de 48 × 4 com o corpo (38 × 4) e os pés (45 × 4) a 16 lógicos
    // (24 pixels) das bordas, recortada na direita: (1744, 996, 176, 192).
    assert_eq!(painel.sprite_disp, Some(Ret::novo(1744, 996, 176, 192)));
    motor.definir_tamanho(Tamanho::Grande, Some(&mut janela), 40);
    assert_eq!(motor.painel(Some(&janela), 50).d, Some(6));
    assert_eq!(motor.skin().map(|s| s.id.as_str()), Some("_teste"));
    // O mesmo tamanho não mexe na tela; sem janela, vale no próximo palco.
    let antes = janela.pedidos.len();
    motor.definir_tamanho(Tamanho::Grande, Some(&mut janela), 60);
    assert_eq!(janela.pedidos.len(), antes);
    let mut novo = Motor::novo(ConfigCerebro::default());
    novo.definir_skin(Some(skin_teste()));
    novo.definir_tamanho(Tamanho::Pequeno, None, 0);
    let mut outra = Falsa::default();
    novo.conectou(0);
    novo.aplicar_visibilidade(&mut outra, 0);
    outra.pronta = Some(edp());
    novo.evento_overlay(&mut outra, EventoOverlay::Pronta(edp()), 0);
    assert_eq!(novo.painel(Some(&outra), 10).d, Some(4));
}

#[test]
fn regiao_que_muda_sem_pixels_novos_conta_como_commit() {
    let (mut motor, mut janela) = ligado();
    // A janela esqueceu a região (como depois de uma troca de escala): o
    // próximo quadro igual só manda o estado.
    janela.regiao = None;
    janela.mostrou();
    motor.desenhar(&mut janela, 50, false);
    assert_eq!(janela.quadros(), 1, "nenhum pixel novo");
    assert_eq!(motor.painel(Some(&janela), 60).commits_total, 2);
}
