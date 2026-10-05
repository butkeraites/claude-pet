//! O Motor com relógio falso e uma janela falsa: nada de compositor.

use std::path::PathBuf;
use std::rc::Rc;

use super::*;
use crate::cerebro::Nivel;
use crate::plataforma::falsa::JanelaFalsa as Falsa;

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
        ..Monitor::default()
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
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 0);
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
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 30);
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
    let sem = motor.painel(None, 10);
    assert_eq!(
        Painel {
            desktop: Painel::default().desktop,
            ..sem.clone()
        },
        Painel::default(),
        "vazio, fora o desktop"
    );
    assert_eq!(
        sem.desktop.monitor_em_foco.as_deref(),
        Some("eDP-1"),
        "o desktop fica: a camada contou o monitor em foco (decisão 0059)"
    );
    assert_eq!(motor.painel(Some(&janela), 10).commits_total, 1);
    motor.desconectou();
    assert!(!motor.quer_mostrar());
    assert_eq!(
        motor.proximo_prazo(),
        Some(tela::BOCEJO_MS),
        "só o relógio do sono (decisão 0076)"
    );
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
    novo.evento_overlay(&mut outra, EventoOverlay::Pronta, 0);
    assert_eq!(novo.painel(Some(&outra), 10).d, Some(4));
}

#[test]
fn regiao_que_muda_sem_pixels_novos_conta_como_commit() {
    let (mut motor, mut janela) = ligado();
    // A janela esqueceu a região (como depois de uma troca de escala): o
    // próximo quadro igual só manda o estado.
    janela.toque = None;
    janela.mostrou();
    motor.desenhar(&mut janela, 50, false);
    assert_eq!(janela.quadros(), 1, "nenhum pixel novo");
    assert_eq!(motor.painel(Some(&janela), 60).commits_total, 2);
}

#[test]
fn duas_prontas_na_mesma_leva_desenham_so_o_palco_de_agora() {
    // A escala muda e o `configure` chega junto: a janela já está no monitor
    // novo quando o Motor vê o primeiro evento. Os dois desenham com o palco
    // de agora (D da escala 2), nunca com o da escala velha.
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    janela.pronta = Some(Monitor {
        escala: 2.0,
        ..edp()
    });
    janela.cena = None; // o buffer novo: redesenha tudo
    for _ in 0..2 {
        motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 100);
    }
    assert_eq!(janela.quadros(), 2, "um quadro só para as duas");
    assert_eq!(motor.painel(Some(&janela), 110).d, Some(6), "_teste a 2,0");
    let Some(Elemento::Sprite { d, .. }) = janela.cena.as_ref().and_then(|c| c.first().copied())
    else {
        panic!("sem sprite");
    };
    assert_eq!(d, 6, "o quadro na tela é o do palco novo");
}

#[test]
fn janela_pequena_usa_a_area_util_e_o_acerto_e_no_palco() {
    // Uma janela pequena (Win32, AppKit, X11) num monitor com barra de
    // tarefas: o pet nasce acima da barra, e um clique no corpo dele, já em
    // coordenadas do palco, acerta; ao lado, atravessa.
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.definir_skin(Some(skin_teste()));
    let mut janela = Falsa::pequena();
    motor.conectou(0);
    motor.aplicar_visibilidade(&mut janela, 0);
    janela.pronta = Some(Monitor {
        origem: Some((1280, 0)),
        area_util: Some(Ret::novo(0, 0, 1920, 1128)),
        ..edp()
    });
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 0);
    let painel = motor.painel(Some(&janela), 10);
    let sprite = painel.sprite_disp.unwrap();
    // Os pés (linha 45 da célula, D = 5) a 24 pixels da barra.
    assert_eq!(sprite.y + 45 * 5, 1128 - 24, "acima da barra de tarefas");
    let toque = janela.toque.expect("toque pedido no palco");
    assert_eq!(toque, Ret::novo(1756, 944, 140, 160));
    assert!(motor.acerta_o_pet(toque.x, toque.y));
    assert!(motor.acerta_o_pet(toque.direita() - 1, toque.baixo() - 1));
    assert!(!motor.acerta_o_pet(toque.x - 1, toque.y));
    assert!(!motor.acerta_o_pet(toque.x, toque.baixo()));
    // Sem palco (janela fechada), nada acerta.
    motor.evento_overlay(&mut janela, EventoOverlay::Sumiu, 20);
    assert!(!motor.acerta_o_pet(toque.x, toque.y));
}

#[test]
fn estresse_numa_janela_pequena_avisa_e_nao_comeca() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.definir_skin(Some(skin_teste()));
    let mut janela = Falsa::pequena();
    motor.conectou(0);
    motor.aplicar_visibilidade(&mut janela, 0);
    janela.pronta = Some(edp());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 0);
    janela.mostrou();
    motor.estresse(&mut janela, 30, 5, 10);
    assert!(!motor.painel(Some(&janela), 20).estresse);
    assert_eq!(janela.cena.as_ref().unwrap().len(), 1, "só o pet");
}

#[test]
fn desktop_conta_o_monitor_e_a_janela_ativa_no_painel_com_ou_sem_conexao() {
    use crate::plataforma::{Alca, EventoDesktop};
    let (mut motor, mut janela) = ligado();
    let agora = Agora {
        parede_ms: PAREDE,
        mono_ms: 0,
    };
    assert_eq!(motor.painel(Some(&janela), 0).desktop.eventos, "sem");
    assert!(motor.evento_desktop(Some(&mut janela), &EventoDesktop::Ligado(true), agora));
    assert!(motor.evento_desktop(
        Some(&mut janela),
        &EventoDesktop::MonitorEmFoco("HDMI-A-1".into()),
        agora
    ));
    assert!(motor.evento_desktop(
        None,
        &EventoDesktop::JanelaAtiva {
            janela: Some(Alca("5bbf4e6128f0".into())),
            parede_ms: PAREDE,
        },
        agora
    ));
    janela.desktop.janelas = vec![Alca("5bbf4e6128f0".into())];
    let p = motor.painel(Some(&janela), 0).desktop;
    assert_eq!(p.eventos, "ligado");
    assert_eq!(p.monitor_em_foco.as_deref(), Some("HDMI-A-1"));
    assert_eq!(p.janela_ativa.as_deref(), Some("5bbf4e6128f0"));
    assert!(p.foca_janelas, "o desktop da conexão foca");
    assert_eq!((p.protocolos, p.janelas), (vec!["falso".to_owned()], 1));
    // Sem conexão (o compositor caiu), o que os eventos contaram continua.
    let sem = motor.painel(None, 0).desktop;
    assert_eq!(sem.janela_ativa.as_deref(), Some("5bbf4e6128f0"));
    assert!(!sem.foca_janelas && sem.janelas == 0);
}

#[test]
fn a_janela_falsa_e_um_punho_com_o_desktop_junto() {
    use crate::plataforma::{Alca, Cursor, Desktop, ErroFoco, Punho};
    let mut janela = Falsa::default();
    janela.desktop.janelas = vec![Alca("a1".into())];
    let punho: &mut dyn Punho = &mut janela;
    assert_eq!(punho.desktop().focar(&Alca("a1".into())), Ok(()));
    assert_eq!(
        punho.desktop().focar(&Alca("a2".into())),
        Err(ErroFoco::JanelaSumiu)
    );
    punho.janela().cursor(Cursor::Agarrar);
    assert_eq!(janela.cursor, Some(Cursor::Agarrar));
    assert_eq!(janela.desktop.focos, vec![Alca("a1".into())]);
    janela.desktop.capacidades.foca_janela = false;
    assert_eq!(
        janela.desktop.focar(&Alca("a1".into())),
        Err(ErroFoco::NaoSuportado)
    );
}

// --- arrastar e clicar (decisão 0048) ---------------------------------------

fn ponteiro(motor: &mut Motor, janela: &mut Falsa, ev: crate::plataforma::EventoPonteiro, t: u64) {
    motor.evento_overlay(janela, EventoOverlay::Ponteiro(ev), t);
}

fn apertou(x: i32, y: i32) -> crate::plataforma::EventoPonteiro {
    crate::plataforma::EventoPonteiro::Apertou {
        botao: crate::plataforma::Botao::Esquerdo,
        x,
        y,
    }
}

fn soltou(x: i32, y: i32) -> crate::plataforma::EventoPonteiro {
    crate::plataforma::EventoPonteiro::Soltou {
        botao: crate::plataforma::Botao::Esquerdo,
        x,
        y,
    }
}

fn moveu(x: i32, y: i32) -> crate::plataforma::EventoPonteiro {
    crate::plataforma::EventoPonteiro::Moveu { x, y }
}

/// O meio do corpo do pet, no palco.
fn meio_do_corpo(janela: &Falsa) -> (i32, i32) {
    let t = janela.toque.expect("toque no corpo");
    (t.x + t.w / 2, t.y + t.h / 2)
}

#[test]
fn arrastar_pendura_o_pet_cresce_o_toque_e_solta_com_pouso() {
    use crate::plataforma::Cursor;
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let corpo = janela.toque.unwrap();
    let (x, y) = meio_do_corpo(&janela);
    let antes = motor.painel(Some(&janela), 0).sprite_disp.unwrap();
    ponteiro(&mut motor, &mut janela, apertou(x, y), 1_000);
    assert_eq!(janela.cursor, Some(Cursor::Agarrar));
    assert_eq!(janela.toque, Some(corpo), "apertar não cresce o toque");
    // 5 pixels (menos de 4 lógicos a 1,5 = 6): ainda não arrasta.
    ponteiro(&mut motor, &mut janela, moveu(x - 5, y), 1_010);
    assert!(!motor.arrastando());
    // 100 pixels para a esquerda e 52 para cima: arrasta, em múltiplos de D
    // (5 no _teste): −100 → −100, −52 → −50.
    ponteiro(&mut motor, &mut janela, moveu(x - 100, y - 52), 1_020);
    assert!(motor.arrastando());
    assert_eq!(
        janela.toque,
        Some(Ret::novo(0, 0, 1920, 1200)),
        "o palco inteiro enquanto arrasta"
    );
    let p = motor.painel(Some(&janela), 1_020);
    assert!(p.arrastando);
    assert_eq!(p.reacao.as_deref(), Some(ARRASTADO));
    let agora = p.sprite_disp.unwrap();
    assert_eq!((agora.x, agora.y), (antes.x - 100, antes.y - 50));
    // Bem além da borda direita: o corpo fica inteiro no palco.
    ponteiro(&mut motor, &mut janela, moveu(x + 5_000, y), 1_030);
    janela.mostrou();
    motor.evento_overlay(&mut janela, EventoOverlay::Redesenhar, 1_031);
    let toque = motor_toque(&motor);
    assert_eq!(toque.direita(), 1920, "preso na borda");
    // Solta: o cursor volta, o toque volta ao corpo, pousa.
    ponteiro(&mut motor, &mut janela, soltou(x - 300, y - 200), 1_100);
    assert!(!motor.arrastando());
    assert_eq!(janela.cursor, Some(Cursor::Pegar));
    let p = motor.painel(Some(&janela), 1_100);
    assert_eq!(p.reacao.as_deref(), Some(SOLTO), "o pouso");
    let sprite = p.sprite_disp.unwrap();
    assert_eq!((sprite.x, sprite.y), (antes.x - 300, antes.y - 200));
    let toque = janela.toque.unwrap();
    assert_eq!(
        (toque.w, toque.h),
        (corpo.w, corpo.h),
        "toque de novo só no corpo"
    );
}

/// A área de toque do pet como o Motor a calcula agora (no palco).
fn motor_toque(motor: &Motor) -> Ret {
    let (Some(pet), Some(palco)) = (motor.pet.as_ref(), motor.palco.as_ref()) else {
        panic!("sem pet");
    };
    pet.toque_no_palco(palco).unwrap()
}

#[test]
fn segurar_250_ms_arrasta_no_lugar_e_o_fail_safe_solta() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 2_000);
    assert_eq!(motor.proximo_prazo().map(|p| p.min(2_250)), Some(2_250));
    motor.vencer(&mut janela, 2_250);
    assert!(motor.arrastando(), "segurou: arrasta no lugar");
    assert_eq!(janela.toque, Some(Ret::novo(0, 0, 1920, 1200)));
    // 5 s sem evento do ponteiro desde o aperto (perdeu a pegada): solta
    // onde está.
    janela.mostrou();
    motor.vencer(&mut janela, 2_000 + 4_999);
    assert!(motor.arrastando());
    motor.vencer(&mut janela, 2_000 + 5_000);
    assert!(!motor.arrastando());
    assert_ne!(
        janela.toque,
        Some(Ret::novo(0, 0, 1920, 1200)),
        "toque de volta ao corpo"
    );
}

#[test]
fn clique_esquerdo_da_risadinha_e_o_do_lado_atravessa() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 3_000);
    ponteiro(&mut motor, &mut janela, soltou(x + 2, y + 1), 3_080);
    assert_eq!(
        motor.painel(Some(&janela), 3_080).reacao.as_deref(),
        Some(RISADINHA)
    );
    // Um aperto fora do corpo (a região chegaria só no corpo; aqui o Motor
    // confere de novo) não é do pet.
    let corpo = janela.toque.unwrap();
    ponteiro(
        &mut motor,
        &mut janela,
        apertou(corpo.x - 1, corpo.y),
        4_000,
    );
    ponteiro(
        &mut motor,
        &mut janela,
        moveu(corpo.x - 200, corpo.y),
        4_010,
    );
    assert!(!motor.arrastando());
}

#[test]
fn esconder_e_a_janela_fechada_largam_o_arraste() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 0);
    ponteiro(&mut motor, &mut janela, moveu(x - 50, y), 10);
    assert!(motor.arrastando());
    motor.definir_visivel(false);
    motor.aplicar_visibilidade(&mut janela, 20);
    assert!(!motor.arrastando(), "esconder larga");
    assert_eq!(motor.painel(Some(&janela), 20).reacao, None);
    motor.definir_visivel(true);
    motor.aplicar_visibilidade(&mut janela, 30);
    janela.mostrou();
    ponteiro(&mut motor, &mut janela, apertou(x - 50, y), 40);
    ponteiro(&mut motor, &mut janela, moveu(x - 150, y), 50);
    assert!(motor.arrastando());
    janela.fase = None;
    motor.evento_overlay(&mut janela, EventoOverlay::Sumiu, 60);
    assert!(!motor.arrastando(), "janela fechada larga");
    // Sem palco, o ponteiro não faz nada.
    ponteiro(&mut motor, &mut janela, apertou(x, y), 70);
    assert!(motor.proximo_prazo().is_none_or(|p| p > 70 + 1_000));
}

#[test]
fn arrastar_com_quadro_em_voo_adia_e_o_redesenhar_leva_a_posicao_de_agora() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 0);
    ponteiro(&mut motor, &mut janela, moveu(x - 50, y), 10);
    let quadros = janela.quadros();
    // Três movimentos com o quadro em voo: nenhum quadro novo.
    for dx in [60, 70, 80] {
        ponteiro(&mut motor, &mut janela, moveu(x - dx, y), 20);
    }
    assert_eq!(janela.quadros(), quadros, "um quadro em voo por vez");
    janela.mostrou();
    motor.evento_overlay(&mut janela, EventoOverlay::Redesenhar, 30);
    assert_eq!(janela.quadros(), quadros + 1);
    let Some(Elemento::Sprite { x: sx, .. }) =
        janela.cena.as_ref().and_then(|c| c.first().copied())
    else {
        panic!("sem sprite");
    };
    let antes = 1706; // a célula padrão do _teste no eDP-1
    assert_eq!(sx, antes - 80, "a posição do último movimento");
}

// --- seguir o monitor ativo (decisão 0051) ----------------------------------

fn hdmi() -> Monitor {
    Monitor {
        nome: Some("HDMI-A-1".into()),
        descricao: Some("Dell Inc. DELL U2720Q 7LN4 (HDMI-A-1)".into()),
        logico: (2560, 1440),
        escala: 1.5,
        origem: Some((0, -1440)),
        area_util: None,
    }
}

fn em(ms: u64) -> Agora {
    Agora {
        parede_ms: PAREDE + ms,
        mono_ms: ms,
    }
}

fn foco(motor: &mut Motor, janela: &mut Falsa, nome: &str, ms: u64) {
    motor.evento_desktop(
        Some(janela),
        &crate::plataforma::EventoDesktop::MonitorEmFoco(nome.into()),
        em(ms),
    );
}

fn tem_sprite(janela: &Falsa) -> bool {
    janela
        .cena
        .as_ref()
        .is_some_and(|c| c.iter().any(|e| matches!(e, Elemento::Sprite { .. })))
}

/// A janela de mentira termina de sair (o quadro transparente foi mostrado).
fn saiu(motor: &mut Motor, janela: &mut Falsa, ms: u64) {
    janela.fase = None;
    motor.evento_overlay(janela, EventoOverlay::Saiu, ms);
}

#[test]
fn segue_o_monitor_em_foco_com_debounce_e_poof_na_saida_e_na_chegada() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    foco(&mut motor, &mut janela, "HDMI-A-1", 1_000);
    motor.vencer(&mut janela, 1_299);
    assert_eq!(motor.painel(Some(&janela), 1_299).viagem, None, "debounce");
    motor.vencer(&mut janela, 1_300);
    assert_eq!(motor.painel(Some(&janela), 1_300).viagem, Some("poof"));
    assert!(tem_sprite(&janela), "no primeiro passo o pet ainda está");
    assert!(janela.cena.as_ref().unwrap().len() > 1, "e a nuvem");
    janela.mostrou();
    motor.vencer(&mut janela, 1_360);
    assert!(!tem_sprite(&janela), "o pet some no segundo passo");
    janela.mostrou();
    motor.vencer(&mut janela, 1_540);
    assert_eq!(janela.pedidos.last().unwrap(), "apagar com _teste");
    assert_eq!(motor.painel(Some(&janela), 1_540).viagem, Some("saindo"));
    saiu(&mut motor, &mut janela, 1_600);
    assert_eq!(
        janela.pedidos.last().unwrap(),
        "criar",
        "nasce no monitor em foco"
    );
    assert_eq!(motor.painel(Some(&janela), 1_600).viagem, Some("chegando"));
    janela.pronta = Some(hdmi());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 1_700);
    let p = motor.painel(Some(&janela), 1_700);
    assert_eq!(p.viagem, Some("entrando"));
    assert_eq!(p.monitor.as_deref(), Some("HDMI-A-1"));
    assert_eq!(p.d, Some(8), "o D do monitor novo");
    assert!(!tem_sprite(&janela), "a nuvem antes do pet");
    janela.mostrou();
    motor.vencer(&mut janela, 1_760);
    assert!(tem_sprite(&janela), "o pet aparece no segundo passo");
    janela.mostrou();
    motor.vencer(&mut janela, 1_700 + 240);
    assert_eq!(motor.painel(Some(&janela), 1_940).viagem, None);
    assert_eq!(janela.cena.as_ref().unwrap().len(), 1, "só o pet");
    // Já está no monitor em foco: conferir de novo não viaja.
    motor.vencer(&mut janela, 5_000);
    assert_eq!(motor.painel(Some(&janela), 5_000).viagem, None);
}

#[test]
fn rajada_de_foco_que_volta_ao_mesmo_monitor_nao_viaja() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    // A proteção de tela do Omarchy foca cada monitor em sequência.
    foco(&mut motor, &mut janela, "HDMI-A-1", 0);
    foco(&mut motor, &mut janela, "eDP-1", 120);
    motor.vencer(&mut janela, 300);
    assert_eq!(
        motor.painel(Some(&janela), 300).viagem,
        None,
        "ainda no debounce"
    );
    motor.vencer(&mut janela, 420);
    assert_eq!(
        motor.painel(Some(&janela), 420).viagem,
        None,
        "voltou ao mesmo"
    );
    assert!(!janela.pedidos.iter().any(|p| p.starts_with("apagar")));
}

#[test]
fn arrastando_congela_e_solto_dentro_confere_o_foco() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 0);
    ponteiro(&mut motor, &mut janela, moveu(x - 60, y), 10);
    foco(&mut motor, &mut janela, "HDMI-A-1", 20);
    motor.vencer(&mut janela, 400);
    assert_eq!(
        motor.painel(Some(&janela), 400).viagem,
        None,
        "congelado no arraste"
    );
    janela.mostrou();
    ponteiro(&mut motor, &mut janela, soltou(x - 60, y), 500);
    motor.vencer(&mut janela, 500);
    assert_eq!(
        motor.painel(Some(&janela), 500).viagem,
        Some("poof"),
        "solto, segue o foco"
    );
}

#[test]
fn solto_fora_do_monitor_pousa_no_monitor_debaixo_do_ponteiro() {
    let (mut motor, mut janela) = ligado();
    janela.pronta = Some(Monitor {
        origem: Some((640, 0)),
        ..edp()
    });
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 0);
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 10);
    ponteiro(&mut motor, &mut janela, moveu(x - 30, y - 600), 20);
    // Solto acima do eDP-1 (y negativo no palco): o HDMI fica em cima.
    let (sx, sy) = (900, -300);
    janela.mostrou();
    ponteiro(&mut motor, &mut janela, soltou(sx, sy), 30);
    assert!(!motor.arrastando());
    assert_eq!(motor.painel(Some(&janela), 30).viagem, Some("poof"));
    janela.mostrou();
    motor.vencer(&mut janela, 30 + 240);
    saiu(&mut motor, &mut janela, 300);
    janela.pronta = Some(hdmi());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 400);
    let p = motor.painel(Some(&janela), 400);
    assert_eq!(p.reacao.as_deref(), Some(SOLTO), "pousa lá");
    // O ponto solto no desktop: (640 + 900/1,5, 0 − 300/1,5) = (1240, −200)
    // lógicos; no HDMI (origem (0, −1440)): (1240, 1240) lógicos = (1860,
    // 1860) no palco. A célula fica com a pegada debaixo dele.
    let palco = motor.palco.unwrap();
    let pegada = (x - 1706, y - 951);
    assert_eq!((palco.x + pegada.0, palco.y + pegada.1), (1860, 1860));
    // E a posição ficou guardada para o monitor novo.
    assert!(
        motor
            .posicoes()
            .de("descricao:Dell Inc. DELL U2720Q 7LN4")
            .is_some()
    );
}

#[test]
fn mais_de_3_viagens_em_20_s_viram_rapidas_e_escondido_nao_viaja() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let mut t = 0;
    let monitores = [hdmi(), edp(), hdmi(), edp()];
    for (i, monitor) in monitores.iter().enumerate() {
        foco(&mut motor, &mut janela, monitor.nome.as_deref().unwrap(), t);
        t += 2_000;
        janela.mostrou();
        motor.vencer(&mut janela, t);
        let viagem = motor.painel(Some(&janela), t).viagem;
        if i < 3 {
            assert_eq!(viagem, Some("poof"), "viagem {i}");
            janela.mostrou();
            t += 240;
            motor.vencer(&mut janela, t);
        } else {
            assert_eq!(viagem, Some("saindo"), "a quarta em 20 s: sem poof");
        }
        saiu(&mut motor, &mut janela, t);
        janela.pronta = Some(monitor.clone());
        motor.evento_overlay(&mut janela, EventoOverlay::Pronta, t);
        janela.mostrou();
        t += 240;
        motor.vencer(&mut janela, t);
        assert_eq!(motor.painel(Some(&janela), t).viagem, None);
    }
    // Escondido, o foco muda e nada acontece (ele nasce no monitor em foco
    // quando voltar).
    motor.definir_visivel(false);
    motor.aplicar_visibilidade(&mut janela, t);
    foco(&mut motor, &mut janela, "HDMI-A-1", t);
    motor.vencer(&mut janela, t + 300);
    assert_eq!(motor.painel(Some(&janela), t + 300).viagem, None);
}

#[test]
fn esconder_no_meio_da_viagem_desiste_e_a_janela_fechada_espera_a_nova() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    foco(&mut motor, &mut janela, "HDMI-A-1", 0);
    motor.vencer(&mut janela, 300);
    janela.mostrou();
    motor.vencer(&mut janela, 540);
    assert_eq!(motor.painel(Some(&janela), 540).viagem, Some("saindo"));
    motor.definir_visivel(false);
    motor.aplicar_visibilidade(&mut janela, 560);
    saiu(&mut motor, &mut janela, 600);
    assert_eq!(motor.painel(Some(&janela), 600).viagem, None, "desistiu");
    assert_ne!(
        janela.pedidos.last().unwrap(),
        "criar",
        "escondido: não recria"
    );
    // A janela fecha no meio de uma viagem (o monitor saiu): espera a nova.
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    foco(&mut motor, &mut janela, "HDMI-A-1", 0);
    motor.vencer(&mut janela, 300);
    janela.fase = None;
    motor.evento_overlay(&mut janela, EventoOverlay::Sumiu, 320);
    assert_eq!(motor.painel(Some(&janela), 320).viagem, Some("chegando"));
    motor.evento_overlay(&mut janela, EventoOverlay::Recriar, 570);
    assert_eq!(janela.pedidos.last().unwrap(), "criar");
    janela.pronta = Some(hdmi());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 600);
    assert_eq!(motor.painel(Some(&janela), 600).viagem, Some("entrando"));
}

// --- o balão (decisão 0052) -------------------------------------------------

fn prompt_em(motor: &mut Motor, sid: &str, proj: &str, ms: u64) {
    let ev = Evento {
        e: "UserPromptSubmit".into(),
        sid: Some(sid.into()),
        turno: Some(format!("{sid}-p")),
        ent: Some("cli".into()),
        proj: Some(proj.into()),
        ts: Some(PAREDE + ms),
        ..Evento::default()
    };
    motor.evento(&ev, PAREDE + ms, em(ms));
}

/// Há letras na cena (o texto do balão, o "zZ"): os selos ao lado do corpo
/// só têm o "+", algarismos e o "…" (decisão 0083).
fn tem_glifos(janela: &Falsa) -> bool {
    janela.cena.as_ref().is_some_and(|c| {
        c.iter()
            .any(|e| matches!(e, Elemento::Glifo { c, .. } if c.is_alphabetic()))
    })
}

#[test]
fn clique_sem_pendencia_mostra_as_sessoes_num_balao_que_some_sozinho() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    // Sem sessão nenhuma.
    ponteiro(&mut motor, &mut janela, apertou(x, y), 100);
    ponteiro(&mut motor, &mut janela, soltou(x, y), 150);
    let p = motor.painel(Some(&janela), 150);
    assert_eq!(
        p.balao,
        Some(vec!["nenhuma sessão do Claude aberta".to_owned()])
    );
    assert_eq!(p.reacao.as_deref(), Some(RISADINHA));
    // Duas sessões: a mais recente primeiro, com o estado e há quanto tempo.
    prompt_em(&mut motor, "aaaa1111", "api", 1_000);
    prompt_em(&mut motor, "bbbb2222", "claude-pet", 31_000);
    janela.mostrou();
    ponteiro(&mut motor, &mut janela, apertou(x, y), 91_000);
    ponteiro(&mut motor, &mut janela, soltou(x, y), 91_050);
    let linhas = motor.painel(Some(&janela), 91_050).balao.unwrap();
    assert_eq!(
        linhas,
        vec![
            "claude-pet: pensando (1 min)".to_owned(),
            "api: pensando (1 min)".to_owned()
        ]
    );
    janela.mostrou();
    motor.evento_overlay(&mut janela, EventoOverlay::Redesenhar, 91_060);
    assert!(tem_glifos(&janela), "o texto na cena");
    // Some sozinho no prazo (base + uma linha por segundo).
    let ate = 91_050 + balao::BASE_MS + 2 * balao::POR_LINHA_MS;
    assert!(motor.proximo_prazo().unwrap() <= ate);
    janela.mostrou();
    motor.vencer(&mut janela, ate);
    assert_eq!(motor.painel(Some(&janela), ate).balao, None);
    assert!(!tem_glifos(&janela), "o balão saiu da cena");
}

#[test]
fn arrastar_ou_viajar_tira_o_balao() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    motor.mostrar_balao(Some(&mut janela), vec!["oi".into()], 0);
    assert!(motor.balao(10).is_some());
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 20);
    ponteiro(&mut motor, &mut janela, moveu(x - 60, y), 30);
    assert!(motor.balao(30).is_none(), "o arraste tira o balão");
    ponteiro(&mut motor, &mut janela, soltou(x - 60, y), 40);
    motor.mostrar_balao(Some(&mut janela), vec!["oi".into()], 50);
    foco(&mut motor, &mut janela, "HDMI-A-1", 60);
    janela.mostrou();
    motor.vencer(&mut janela, 360);
    assert!(motor.balao(360).is_none(), "a viagem tira o balão");
}

// --- proteção de tela e soneca (decisão 0053) --------------------------------

#[test]
fn a_protecao_de_tela_esconde_o_pet_e_ele_volta_quando_ela_fecha() {
    use crate::plataforma::{Alca, EventoDesktop};
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let abriu = EventoDesktop::JanelaAbriu {
        janela: Alca("5c0ff".into()),
        protetor: true,
    };
    motor.evento_desktop(Some(&mut janela), &abriu, em(1_000));
    assert_eq!(janela.pedidos.last().unwrap(), "apagar com _teste");
    assert!(!motor.quer_mostrar());
    assert!(motor.painel(Some(&janela), 1_000).desktop.protetor_de_tela);
    saiu(&mut motor, &mut janela, 1_050);
    // Um Recriar da janela não volta com a proteção na tela.
    motor.evento_overlay(&mut janela, EventoOverlay::Recriar, 1_100);
    assert_ne!(janela.pedidos.last().unwrap(), "criar");
    let fechou = EventoDesktop::JanelaFechou(Alca("5c0ff".into()));
    motor.evento_desktop(Some(&mut janela), &fechou, em(9_000));
    assert_eq!(janela.pedidos.last().unwrap(), "criar", "volta");
    assert!(motor.quer_mostrar());
}

fn clique_direito(motor: &mut Motor, janela: &mut Falsa, ms: u64) {
    let (x, y) = meio_do_corpo(janela);
    ponteiro(
        motor,
        janela,
        crate::plataforma::EventoPonteiro::Apertou {
            botao: crate::plataforma::Botao::Direito,
            x,
            y,
        },
        ms,
    );
    ponteiro(
        motor,
        janela,
        crate::plataforma::EventoPonteiro::Soltou {
            botao: crate::plataforma::Botao::Direito,
            x,
            y,
        },
        ms + 40,
    );
}

#[test]
fn botao_direito_cochila_30_min_com_o_zz_e_so_reacoes_pequenas() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    clique_direito(&mut motor, &mut janela, 1_000);
    let p = motor.painel(Some(&janela), 1_040);
    assert_eq!(p.soneca_restante_s, Some(1_800));
    assert_eq!(p.reacao.as_deref(), Some(BOCEJO));
    assert!(tem_glifos(&janela), "o selo zZ");
    // O cérebro pede o pulinho: na soneca, vira o aceno.
    assert!(motor.reagir(Some(&mut janela), crate::cerebro::PULINHO, 5_000));
    assert_eq!(
        motor.painel(Some(&janela), 5_000).reacao.as_deref(),
        Some(crate::cerebro::ACENO)
    );
    // O tchau continua tchau (na _teste ele nem anima).
    assert!(!motor.reagir(Some(&mut janela), crate::cerebro::TCHAU, 6_000));
    // De novo o direito: acorda.
    janela.mostrou();
    clique_direito(&mut motor, &mut janela, 10_000);
    let p = motor.painel(Some(&janela), 10_040);
    assert_eq!(p.soneca_restante_s, None);
    // A _teste não tem o despertar (o Zeca tem, nativo): o pet fica na pose.
    assert_eq!(p.reacao, None);
    janela.mostrou();
    motor.vencer(&mut janela, 20_000);
    assert!(!tem_glifos(&janela), "sem selo");
    // A soneca acaba sozinha em 30 min.
    janela.mostrou();
    clique_direito(&mut motor, &mut janela, 30_000);
    let fim = 30_040 + SONECA_MS;
    assert!(motor.proximo_prazo().unwrap() <= fim);
    janela.mostrou();
    motor.vencer(&mut janela, fim);
    assert_eq!(motor.painel(Some(&janela), fim).soneca_restante_s, None);
    assert!(!tem_glifos(&janela));
    assert!(motor.reagir(Some(&mut janela), crate::cerebro::PULINHO, fim + 1));
    assert_eq!(
        motor.painel(Some(&janela), fim + 1).reacao.as_deref(),
        Some(crate::cerebro::PULINHO),
        "acordado, o pulinho de volta"
    );
}

// --- a janela de cada sessão (decisão 0055) ----------------------------------

fn ativou(motor: &mut Motor, janela: Option<&str>, ms: u64) {
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::JanelaAtiva {
            janela: janela.map(|j| crate::plataforma::Alca(j.into())),
            parede_ms: PAREDE + ms,
        },
        em(ms),
    );
}

fn janela_da(motor: &Motor, sid: &str) -> Option<janelas::ResumoJanela> {
    motor
        .resumo()
        .sessoes
        .into_iter()
        .find(|s| s.chave.1 == sid)
        .and_then(|s| s.janela)
}

#[test]
fn o_prompt_casa_a_sessao_com_a_janela_ativa_na_hora_do_hook() {
    use janelas::Certeza;
    let mut motor = Motor::novo(ConfigCerebro::default());
    // Gravado de um socket2: o Renan no foot1, depois no foot2.
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 5_000);
    let a = janela_da(&motor, "sessao-a").unwrap();
    assert_eq!(
        (a.endereco.as_deref(), a.certeza),
        (Some("f00d01"), Certeza::Certa)
    );
    ativou(&mut motor, Some("f00d02"), 10_000);
    prompt_em(&mut motor, "sessao-b", "web", 15_000);
    let b = janela_da(&motor, "sessao-b").unwrap();
    assert_eq!(b.endereco.as_deref(), Some("f00d02"));
    // Trocou para o foot1 e, meio segundo depois, um prompt de B: dúvida,
    // mas a janela certa de antes fica.
    ativou(&mut motor, Some("f00d01"), 20_000);
    prompt_em(&mut motor, "sessao-b", "web", 20_500);
    let b = janela_da(&motor, "sessao-b").unwrap();
    assert_eq!(
        (b.endereco.as_deref(), b.certeza),
        (Some("f00d02"), Certeza::Certa)
    );
    // Uma sessão nova nesse meio segundo: só a dúvida.
    prompt_em(&mut motor, "sessao-c", "x", 20_600);
    let c = janela_da(&motor, "sessao-c").unwrap();
    assert_eq!((c.endereco, c.certeza), (None, Certeza::Duvida));
    // Numa área de trabalho vazia, nenhuma janela.
    ativou(&mut motor, None, 30_000);
    prompt_em(&mut motor, "sessao-d", "y", 35_000);
    assert_eq!(
        janela_da(&motor, "sessao-d").unwrap().certeza,
        Certeza::SemJanela
    );
    // A janela do B fechou.
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::JanelaFechou(crate::plataforma::Alca("f00d02".into())),
        em(40_000),
    );
    let b = janela_da(&motor, "sessao-b").unwrap();
    assert_eq!((b.endereco, b.certeza), (None, Certeza::Fechou));
    // A sessão A acabou: a janela dela some junto.
    let fim = Evento {
        e: "SessionEnd".into(),
        sid: Some("sessao-a".into()),
        ent: Some("cli".into()),
        reason: Some("clear".into()),
        ..Evento::default()
    };
    motor.evento(&fim, PAREDE + 50_000, em(50_000));
    assert!(janela_da(&motor, "sessao-a").is_none());
    assert!(motor.identidades.de(&(false, "sessao-a".into())).is_none());
}

#[test]
fn sem_anel_nao_ha_janela_e_o_hook_traz_os_ids_de_terminal() {
    use janelas::Certeza;
    let mut motor = Motor::novo(ConfigCerebro::default());
    let ev = Evento {
        e: "SessionStart".into(),
        sid: Some("sessao-t".into()),
        ent: Some("cli".into()),
        ts: Some(PAREDE + 100),
        term: Some(crate::evento::Terminal {
            tmux: Some("%4".into()),
            ..Default::default()
        }),
        ..Evento::default()
    };
    motor.evento(&ev, PAREDE + 100, em(100));
    let t = janela_da(&motor, "sessao-t").unwrap();
    assert_eq!(t.certeza, Certeza::SemAnel, "o pet não viu janela nenhuma");
    assert_eq!(t.terminal.unwrap().tmux.as_deref(), Some("%4"));
    // Uma origem que o cérebro não acompanha (claude -p) não ganha janela.
    let sdk = Evento {
        ent: Some("sdk-cli".into()),
        sid: Some("sessao-sdk".into()),
        ..ev.clone()
    };
    motor.evento(&sdk, PAREDE + 200, em(200));
    assert!(
        motor
            .identidades
            .de(&(false, "sessao-sdk".into()))
            .is_none()
    );
}

// --- avisos e o clique (decisão 0057) ----------------------------------------

fn ligar_desktop(motor: &mut Motor, ms: u64) {
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::Ligado(true),
        em(ms),
    );
}

/// Um hook da sessão `sid` (no turno `{sid}-p`, o do [`prompt_em`]).
fn hook_em(motor: &mut Motor, sid: &str, proj: &str, e: &str, ms: u64) {
    let ev = Evento {
        e: e.into(),
        sid: Some(sid.into()),
        turno: Some(format!("{sid}-p")),
        ent: Some("cli".into()),
        proj: Some(proj.into()),
        tool: (e == "PermissionRequest").then(|| "Bash".into()),
        ts: Some(PAREDE + ms),
        ..Evento::default()
    };
    motor.evento(&ev, PAREDE + ms, em(ms));
}

/// A sessão `sid` terminou o turno em `ms`: o pronto depois da acomodação.
fn pronto_em(motor: &mut Motor, sid: &str, proj: &str, ms: u64) {
    hook_em(motor, sid, proj, "Stop", ms);
    motor.tique(em(ms + crate::cerebro::ACOMODACAO_MS));
}

fn pendentes(motor: &Motor) -> Vec<(String, TipoAviso)> {
    motor
        .cerebro
        .pendencias()
        .into_iter()
        .map(|p| (p.chave.1, p.aviso.tipo))
        .collect()
}

/// O vermelho do coração da risadinha na cena (os selos usam outro).
fn tem_coracao(janela: &Falsa) -> bool {
    const VERMELHO_DO_CORACAO: [u8; 4] = [0x4F, 0x3B, 0xE2, 0xFF];
    janela.cena.as_ref().is_some_and(|c| {
        c.iter()
            .any(|e| matches!(e, Elemento::Bloco { cor, .. } if *cor == VERMELHO_DO_CORACAO))
    })
}

fn alcas(nomes: &[&str]) -> Vec<crate::plataforma::Alca> {
    nomes
        .iter()
        .map(|n| crate::plataforma::Alca((*n).into()))
        .collect()
}

#[test]
fn clique_leva_ao_terminal_do_aviso_mais_urgente_e_o_seguinte_ao_proximo() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    // A no foot1 (vai ficar pronto), B no foot2 (vai pedir permissão).
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    ativou(&mut motor, Some("f00d02"), 3_000);
    prompt_em(&mut motor, "sessao-b", "web", 4_000);
    pronto_em(&mut motor, "sessao-a", "api", 5_000);
    hook_em(&mut motor, "sessao-b", "web", "PermissionRequest", 6_000);
    assert_eq!(
        pendentes(&motor),
        vec![
            ("sessao-b".to_owned(), TipoAviso::Esperando),
            ("sessao-a".to_owned(), TipoAviso::Pronto)
        ]
    );
    janela.desktop.janelas = alcas(&["f00d01", "f00d02"]);
    // O Renan está numa terceira janela. Primeiro clique: B, a mais urgente.
    ativou(&mut motor, Some("f00d03"), 7_000);
    janela.mostrou();
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 8_000);
    assert_eq!(
        clicou,
        Clicou::Focou {
            sid8: "sessao-b".into(),
            aviso: TipoAviso::Esperando,
            janela: "f00d02".into(),
            confirmado: false
        }
    );
    assert_eq!(janela.desktop.focos, alcas(&["f00d02"]));
    let p = motor.painel(Some(&janela), 8_000);
    assert_eq!(p.reacao.as_deref(), Some(RISADINHA));
    assert_eq!(p.focando.as_deref(), Some("f00d02"));
    assert!(tem_coracao(&janela), "o coração na cena");
    assert_eq!(pendentes(&motor).len(), 2, "o aviso espera a confirmação");
    // O socket2 conta que o foot2 ficou ativo: o aviso de B sai.
    ativou(&mut motor, Some("f00d02"), 8_100);
    assert!(motor.tirar_mudanca_do_cerebro(), "o núcleo publica");
    assert_eq!(motor.painel(Some(&janela), 8_100).focando, None);
    assert_eq!(
        pendentes(&motor),
        vec![("sessao-a".to_owned(), TipoAviso::Pronto)]
    );
    // O coração sai no prazo dele.
    janela.mostrou();
    motor.vencer(&mut janela, 8_000 + CORACAO_MS);
    assert!(!tem_coracao(&janela), "sem coração");
    // O clique seguinte vai a A.
    janela.mostrou();
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 10_000);
    assert!(
        matches!(&clicou, Clicou::Focou { sid8, janela, aviso: TipoAviso::Pronto, .. }
            if sid8 == "sessao-a" && janela == "f00d01"),
        "{clicou:?}"
    );
    ativou(&mut motor, Some("f00d01"), 10_050);
    assert!(pendentes(&motor).is_empty());
    // Sem aviso: o balão com as sessões.
    janela.mostrou();
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 12_000);
    assert_eq!(clicou, Clicou::Lista { sessoes: 2 });
    let linhas = motor.painel(Some(&janela), 12_000).balao.unwrap();
    assert_eq!(
        linhas,
        vec![
            "web: esperando você (6 s)".to_owned(),
            "api: parado (7 s)".to_owned()
        ]
    );
}

#[test]
fn sem_como_focar_o_balao_diz_o_porque_e_o_ciclo_da_a_volta() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    // C perguntou antes de o pet ver janela nenhuma: sem identidade.
    prompt_em(&mut motor, "sessao-c", "api", 1_000);
    pronto_em(&mut motor, "sessao-c", "api", 2_000);
    // D no foot4, que o desktop não conhece mais (fechou sem o socket2
    // contar).
    ativou(&mut motor, Some("f00d04"), 3_000);
    prompt_em(&mut motor, "sessao-d", "web", 4_000);
    hook_em(&mut motor, "sessao-d", "web", "PermissionRequest", 5_000);
    janela.mostrou();
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 6_000);
    assert_eq!(
        clicou,
        Clicou::NaoFocou {
            sid8: "sessao-d".into(),
            aviso: TipoAviso::Esperando,
            motivo: "a janela dela sumiu"
        }
    );
    let linhas = motor.painel(Some(&janela), 6_000).balao.unwrap();
    assert_eq!(
        linhas,
        vec![
            "web: esperando você".to_owned(),
            "a janela dela sumiu".to_owned(),
            "web: esperando você (1 s)".to_owned(),
            "api: pronto (4 s)".to_owned()
        ]
    );
    assert_eq!(
        motor.painel(Some(&janela), 6_000).reacao,
        None,
        "sem risadinha"
    );
    // O seguinte vai a C, que não tem janela.
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 7_000);
    assert_eq!(
        clicou,
        Clicou::NaoFocou {
            sid8: "sessao-c".into(),
            aviso: TipoAviso::Pronto,
            motivo: "não vi a janela dela"
        }
    );
    // Visitou todas: volta a D. E sem os protocolos, o balão diz.
    janela.desktop.janelas = alcas(&["f00d04"]);
    janela.desktop.capacidades.foca_janela = false;
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 8_000);
    assert_eq!(
        clicou,
        Clicou::NaoFocou {
            sid8: "sessao-d".into(),
            aviso: TipoAviso::Esperando,
            motivo: "aqui eu não sei focar janelas"
        }
    );
    assert!(janela.desktop.focos.is_empty());
    assert_eq!(pendentes(&motor).len(), 2, "nada foi visto");
    // O botão direito: a soneca.
    assert_eq!(
        motor.clicar(&mut janela, Botao::Direito, 9_000),
        Clicou::Soneca { cochilando: true }
    );
    assert!(matches!(
        motor.clicar(&mut janela, Botao::Meio, 9_100),
        Clicou::Nada { .. }
    ));
}

#[test]
fn foco_sem_confirmacao_deixa_o_aviso_e_o_balao_diz() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    pronto_em(&mut motor, "sessao-a", "api", 3_000);
    ativou(&mut motor, Some("f00d03"), 4_000);
    janela.desktop.janelas = alcas(&["f00d01"]);
    janela.mostrou();
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 5_000);
    assert!(matches!(
        clicou,
        Clicou::Focou {
            confirmado: false,
            ..
        }
    ));
    assert!(motor.proximo_prazo().unwrap() <= 5_000 + CONFIRMAR_FOCO_MS);
    // Outra janela ficou ativa (não a do clique): continua esperando.
    ativou(&mut motor, Some("f00d05"), 5_500);
    janela.mostrou();
    motor.vencer(&mut janela, 5_000 + CONFIRMAR_FOCO_MS - 1);
    assert!(motor.painel(Some(&janela), 6_000).focando.is_some());
    // O prazo vence sem a janela ficar ativa (a sessão bloqueada): o aviso
    // fica, e o balão diz.
    janela.mostrou();
    motor.vencer(&mut janela, 5_000 + CONFIRMAR_FOCO_MS);
    let p = motor.painel(Some(&janela), 6_500);
    assert_eq!(p.focando, None);
    let linhas = p.balao.unwrap();
    assert_eq!(
        &linhas[..2],
        &[
            "api: pronto".to_owned(),
            "não consegui focar a janela dela".to_owned()
        ]
    );
    assert_eq!(
        pendentes(&motor),
        vec![("sessao-a".to_owned(), TipoAviso::Pronto)]
    );
    assert!(!motor.tirar_mudanca_do_cerebro());
}

#[test]
fn com_a_janela_ja_ativa_o_clique_ve_na_hora() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    pronto_em(&mut motor, "sessao-a", "api", 3_000);
    janela.desktop.janelas = alcas(&["f00d01"]);
    janela.mostrou();
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 4_000);
    assert!(matches!(
        clicou,
        Clicou::Focou {
            confirmado: true,
            ..
        }
    ));
    assert!(pendentes(&motor).is_empty());
    assert!(motor.tirar_mudanca_do_cerebro());
    // Sem fonte de trocas e sem o foreign-toplevel contar ativações, o
    // aviso também sai na hora (não há como confirmar).
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    pronto_em(&mut motor, "sessao-a", "api", 3_000);
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::Ligado(false),
        em(3_500),
    );
    janela.desktop.janelas = alcas(&["f00d01"]);
    janela.desktop.capacidades.janela_ativa = false;
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 4_000);
    assert!(matches!(
        clicou,
        Clicou::Focou {
            confirmado: true,
            ..
        }
    ));
    assert!(pendentes(&motor).is_empty());
}

/// O desktop conta que o Renan está longe do teclado e do mouse (`true`) ou
/// mexendo (`false`).
fn ocioso(motor: &mut Motor, longe: bool, ms: u64) {
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::Ocioso(longe),
        em(ms),
    );
}

/// O título da janela em foco é (`true`) ou não é o de um terminal do Claude.
fn olhando(motor: &mut Motor, olhando: bool, ms: u64) {
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::OlhandoClaude(olhando),
        em(ms),
    );
}

#[test]
fn o_pronto_sai_com_10_s_do_terminal_em_foco_e_o_esperando_fica() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    // Os foot são terminais do Claude: a escalada do aviso de B fica na L1
    // e não tem prazo (decisão 0075).
    olhando(&mut motor, true, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    ativou(&mut motor, Some("f00d02"), 3_000);
    prompt_em(&mut motor, "sessao-b", "web", 4_000);
    hook_em(&mut motor, "sessao-b", "web", "PermissionRequest", 5_000);
    // A fica pronta com o foot2 em foco: nada de prazo pelo foco de A; o
    // diálogo de B, no terminal dela, conta como visto em 5 s (decisão 0090).
    pronto_em(&mut motor, "sessao-a", "api", 6_000);
    assert_eq!(pendentes(&motor).len(), 2);
    assert_eq!(motor.prazo_do_cerebro(), Some(5_000 + ESPERA_VISTA_MS));
    // O Renan vai ao foot1 por 9 s e sai: o pronto fica.
    ativou(&mut motor, Some("f00d01"), 10_000);
    assert_eq!(motor.prazo_do_cerebro(), Some(10_000 + VISTO_PELO_FOCO_MS));
    motor.tique(em(19_000));
    ativou(&mut motor, Some("f00d03"), 19_000);
    motor.tique(em(20_000));
    assert_eq!(pendentes(&motor).len(), 2);
    // Volta e fica 10 s: o pronto sai sem clique.
    ativou(&mut motor, Some("f00d01"), 30_000);
    motor.tique(em(39_999));
    assert_eq!(pendentes(&motor).len(), 2);
    motor.tique(em(40_000));
    assert_eq!(
        pendentes(&motor),
        vec![("sessao-b".to_owned(), TipoAviso::Esperando)]
    );
    assert!(motor.tirar_mudanca_do_cerebro());
    // O "esperando você" não sai pelo foco, nem com 1 min no foot2.
    ativou(&mut motor, Some("f00d02"), 50_000);
    motor.tique(em(110_000));
    assert_eq!(pendentes(&motor).len(), 1);
    // Com a fonte das trocas fora, a janela "ativa" não conta.
    hook_em(&mut motor, "sessao-b", "web", "PostToolUse", 120_000);
    pronto_em(&mut motor, "sessao-b", "web", 121_000);
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::Ligado(false),
        em(122_000),
    );
    motor.tique(em(140_000));
    assert_eq!(
        pendentes(&motor),
        vec![("sessao-b".to_owned(), TipoAviso::Pronto)]
    );
}

// --- prazos sem conexão e o alvo velho (decisão 0059) ------------------------

/// Vence os prazos do Motor até `ate` sem conexão, como o laço do daemon
/// faz: cada prazo vencido tem de sair (ou ir para depois); um que ficasse
/// armado faria o laço acordar de novo na hora, sem fim.
fn vencer_sem_conexao_ate(motor: &mut Motor, ate: u64) {
    for _ in 0..1_000 {
        let Some(prazo) = motor.proximo_prazo().filter(|&p| p <= ate) else {
            return;
        };
        if motor.prazo_do_cerebro().is_some_and(|p| p <= prazo) {
            motor.tique(em(prazo));
        }
        motor.vencer_sem_conexao(prazo);
        assert!(
            motor.proximo_prazo().is_none_or(|p| p > prazo),
            "o prazo {prazo} ficou armado sem conexão"
        );
    }
    panic!("mil prazos sem conexão até {ate}: o laço giraria");
}

#[test]
fn sem_conexao_nenhum_prazo_vencido_fica_armado() {
    use crate::plataforma::EventoDesktop;
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    motor.mostrar_balao(Some(&mut janela), vec!["oi".into()], 100);
    janela.mostrou();
    clique_direito(&mut motor, &mut janela, 200);
    assert!(motor.soneca(300).is_some());
    // O compositor cai; o socket2 continua contando o foco.
    motor.desconectou();
    assert!(motor.balao(300).is_none(), "o balão era da janela");
    motor.evento_desktop(
        None,
        &EventoDesktop::MonitorEmFoco("HDMI-A-1".into()),
        em(1_000),
    );
    assert_eq!(motor.proximo_prazo(), Some(1_000 + viagem::DEBOUNCE_MS));
    vencer_sem_conexao_ate(&mut motor, 1_000 + SONECA_MS);
    assert_eq!(motor.soneca(1_000 + SONECA_MS), None, "a soneca acabou");
    assert_eq!(motor.proximo_prazo(), None);
    // Com a conexão de volta, a camada nova nasce no monitor em foco.
    let mut nova = Falsa::default();
    motor.conectou(SONECA_MS + 2_000);
    motor.aplicar_visibilidade(&mut nova, SONECA_MS + 2_000);
    assert_eq!(nova.pedidos, vec!["criar"]);
}

#[test]
fn alvo_velho_nao_faz_o_pet_viajar_sem_fim() {
    // O foco contado é o HDMI, mas a camada nova cai no eDP-1 (o HDMI saiu,
    // ou o Hyprland reiniciou com o eDP-1 em foco) e nenhum foco novo chega:
    // o eDP-1 é o monitor em foco, e o pet fica nele.
    let (mut motor, mut janela) = ligado();
    assert_eq!(
        motor
            .painel(Some(&janela), 0)
            .desktop
            .monitor_em_foco
            .as_deref(),
        Some("eDP-1"),
        "a primeira camada já diz o monitor em foco"
    );
    janela.mostrou();
    foco(&mut motor, &mut janela, "HDMI-A-1", 0);
    motor.vencer(&mut janela, 300);
    janela.mostrou();
    motor.vencer(&mut janela, 540);
    saiu(&mut motor, &mut janela, 600);
    assert_eq!(janela.pedidos.last().unwrap(), "criar");
    // O compositor pôs a camada nova no eDP-1 (a janela de mentira continua
    // pronta nele).
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 700);
    let pedidos = janela.pedidos.len();
    for t in [700 + 240, 2_500, 4_000, 10_000, 30_000] {
        janela.mostrou();
        motor.vencer(&mut janela, t);
    }
    let novos = &janela.pedidos[pedidos..];
    assert!(
        !novos
            .iter()
            .any(|p| p == "criar" || p.starts_with("apagar") || p == "destruir"),
        "viajou de novo: {novos:?}"
    );
    let p = motor.painel(Some(&janela), 30_000);
    assert_eq!(p.viagem, None);
    assert_eq!(p.desktop.monitor_em_foco.as_deref(), Some("eDP-1"));
}

#[test]
fn foco_que_chega_depois_da_camada_pedida_ainda_leva_o_pet() {
    // A camada nova cai no HDMI (o foco quando ela foi pedida), mas o Renan
    // volta ao eDP-1 antes de ela ficar pronta: o foco novo vale.
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    foco(&mut motor, &mut janela, "HDMI-A-1", 0);
    motor.vencer(&mut janela, 300);
    janela.mostrou();
    motor.vencer(&mut janela, 540);
    saiu(&mut motor, &mut janela, 600);
    foco(&mut motor, &mut janela, "eDP-1", 650);
    janela.pronta = Some(hdmi());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 700);
    janela.mostrou();
    motor.vencer(&mut janela, 940);
    assert_eq!(motor.painel(Some(&janela), 940).viagem, None, "o intervalo");
    janela.mostrou();
    motor.vencer(&mut janela, 300 + viagem::INTERVALO_MS);
    assert_eq!(
        motor.painel(Some(&janela), 1_800).viagem,
        Some("poof"),
        "volta ao eDP-1"
    );
}

// --- a janela da sessão: o começo, a compactação e o hook atrasado (decisão
// 0060) ------------------------------------------------------------------------

fn evento_de(e: &str, sid: &str, src: Option<&str>, ts_ms: u64) -> Evento {
    Evento {
        e: e.into(),
        sid: Some(sid.into()),
        turno: Some(format!("{sid}-{ts_ms}")),
        ent: Some("cli".into()),
        proj: Some("api".into()),
        src: src.map(str::to_owned),
        ts: Some(PAREDE + ts_ms),
        ..Evento::default()
    }
}

#[test]
fn a_compactacao_no_meio_do_turno_nao_troca_a_janela_da_sessao() {
    // A sessão casou com o foot1 no prompt. Num turno longo o Claude Code
    // compacta o contexto (`SessionStart` com `compact`) com o Renan no
    // navegador há 30 s: a janela da sessão continua o foot1, e 10 s com o
    // navegador em foco não dão o pronto como visto.
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    ativou(&mut motor, Some("f00d03"), 3_000);
    let compacta = evento_de("SessionStart", "sessao-a", Some("compact"), 33_000);
    motor.evento(&compacta, PAREDE + 33_000, em(33_000));
    let a = janela_da(&motor, "sessao-a").unwrap();
    assert_eq!(
        (a.endereco.as_deref(), a.certeza),
        (Some("f00d01"), janelas::Certeza::Certa)
    );
    pronto_em(&mut motor, "sessao-a", "api", 40_000);
    motor.tique(em(60_000));
    assert_eq!(
        pendentes(&motor),
        vec![("sessao-a".to_owned(), TipoAviso::Pronto)],
        "o navegador em foco não é o terminal da sessão"
    );
    // Um prompt que não veio do teclado (`source` de sistema) também não
    // casa; o do teclado, sim.
    let sistema = evento_de("UserPromptSubmit", "sessao-a", Some("system"), 70_000);
    motor.evento(&sistema, PAREDE + 70_000, em(70_000));
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d01")
    );
    prompt_em(&mut motor, "sessao-a", "api", 80_000);
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d03"),
        "o --resume noutro terminal"
    );
}

#[test]
fn o_hook_atrasado_e_o_comeco_nao_desfazem_a_janela_certa() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    ativou(&mut motor, Some("f00d02"), 15_000);
    // O prompt de 20 s (no foot2) chega antes do de 10 s (no foot1): os hooks
    // são assíncronos.
    prompt_em(&mut motor, "sessao-a", "api", 20_000);
    let atrasado = evento_de("UserPromptSubmit", "sessao-a", None, 10_000);
    motor.evento(&atrasado, PAREDE + 20_100, em(20_100));
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d02")
    );
    // Um `resume` com o Renan noutra janela não troca a janela certa.
    ativou(&mut motor, Some("f00d04"), 25_000);
    let volta = evento_de("SessionStart", "sessao-a", Some("resume"), 30_000);
    motor.evento(&volta, PAREDE + 30_000, em(30_000));
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d02")
    );
    // Uma sessão nova sem janela: o começo preenche.
    let nova = evento_de("SessionStart", "sessao-b", Some("startup"), 31_000);
    motor.evento(&nova, PAREDE + 31_000, em(31_000));
    assert_eq!(
        janela_da(&motor, "sessao-b").unwrap().endereco.as_deref(),
        Some("f00d04")
    );
}

// --- o Renan longe, a volta do ciclo e os cliques seguidos (decisão 0062) ----

#[test]
fn o_pronto_nao_sai_pelo_foco_com_o_renan_longe() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    // O Renan sai com o terminal em foco (bloqueado, o Hyprland continua
    // contando o terminal como ativo): longe 5 s depois da última tecla.
    ocioso(&mut motor, true, 3_000 + OCIOSO_MS);
    pronto_em(&mut motor, "sessao-a", "api", 60_000);
    motor.tique(em(600_000));
    assert_eq!(
        pendentes(&motor),
        vec![("sessao-a".to_owned(), TipoAviso::Pronto)],
        "longe, o terminal em foco não é visto"
    );
    // Ele volta e mexe: conta 10 s daqui. No meio, longe de novo: pausa.
    ocioso(&mut motor, false, 700_000);
    assert_eq!(motor.prazo_do_cerebro(), Some(700_000 + VISTO_PELO_FOCO_MS));
    ocioso(&mut motor, true, 705_000);
    motor.tique(em(710_000));
    assert_eq!(pendentes(&motor).len(), 1);
    ocioso(&mut motor, false, 720_000);
    motor.tique(em(729_999));
    assert_eq!(pendentes(&motor).len(), 1);
    motor.tique(em(730_000));
    assert!(pendentes(&motor).is_empty(), "10 s com ele ali: viu");
}

#[test]
fn sem_saber_se_o_renan_esta_o_pronto_so_sai_pelo_clique() {
    // Um desktop que não conta se o Renan está (sem o protocolo, ou a
    // conexão caiu): o terminal em foco nunca dá o pronto como visto.
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    motor.desconectou();
    pronto_em(&mut motor, "sessao-a", "api", 3_000);
    vencer_sem_conexao_ate(&mut motor, 60_000);
    assert_eq!(pendentes(&motor).len(), 1);
    assert_eq!(motor.painel(None, 60_000).desktop.ocioso, None);
    // A conexão volta, sem contar: ainda não sai pelo foco.
    motor.conectou(61_000);
    motor.aplicar_visibilidade(&mut janela, 61_000);
    motor.tique(em(120_000));
    assert_eq!(pendentes(&motor).len(), 1);
}

#[test]
fn com_o_renan_longe_o_clique_de_script_nao_ve_a_janela_ja_ativa() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    pronto_em(&mut motor, "sessao-a", "api", 3_000);
    // Bloqueado: o terminal continua "ativo", o Renan longe. O clique do
    // `/v1/comando` (um script) não conta como visto: espera o desktop.
    ocioso(&mut motor, true, 9_000);
    janela.desktop.janelas = alcas(&["f00d01"]);
    janela.mostrou();
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 20_000);
    assert!(
        matches!(
            clicou,
            Clicou::Focou {
                confirmado: false,
                ..
            }
        ),
        "{clicou:?}"
    );
    janela.mostrou();
    motor.vencer(&mut janela, 20_000 + CONFIRMAR_FOCO_MS);
    assert_eq!(pendentes(&motor).len(), 1, "o aviso fica");
    let linhas = motor.painel(Some(&janela), 21_600).balao.unwrap();
    assert_eq!(linhas[1], "não consegui focar a janela dela");
    // O clique de verdade, com o mouse: o aperto no pet é o Renan ali, e a
    // janela já ativa conta na hora.
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 30_000);
    ponteiro(&mut motor, &mut janela, soltou(x, y), 30_050);
    assert!(pendentes(&motor).is_empty());
    assert_eq!(
        motor.painel(Some(&janela), 30_050).desktop.ocioso,
        Some(false)
    );
}

#[test]
fn um_clique_muito_depois_recomeca_a_volta_do_mais_urgente() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    // C e E sem janela; D numa janela que o desktop não conhece mais.
    prompt_em(&mut motor, "sessao-c", "api", 1_000);
    pronto_em(&mut motor, "sessao-c", "api", 2_000);
    prompt_em(&mut motor, "sessao-e", "doc", 2_500);
    pronto_em(&mut motor, "sessao-e", "doc", 2_600);
    ativou(&mut motor, Some("f00d04"), 3_000);
    prompt_em(&mut motor, "sessao-d", "web", 4_000);
    hook_em(&mut motor, "sessao-d", "web", "PermissionRequest", 5_000);
    janela.mostrou();
    let sid = |c: Clicou| match c {
        Clicou::NaoFocou { sid8, .. } => sid8,
        outro => panic!("{outro:?}"),
    };
    assert_eq!(
        sid(motor.clicar(&mut janela, Botao::Esquerdo, 6_000)),
        "sessao-d"
    );
    // Logo depois, a volta continua: C.
    assert_eq!(
        sid(motor.clicar(&mut janela, Botao::Esquerdo, 7_000)),
        "sessao-c"
    );
    // Um minuto depois, outra volta: o mais urgente de novo (e não o E, o
    // próximo da volta velha).
    assert_eq!(
        sid(motor.clicar(&mut janela, Botao::Esquerdo, 67_000)),
        "sessao-d"
    );
}

#[test]
fn cliques_seguidos_esperam_cada_um_a_sua_confirmacao() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    prompt_em(&mut motor, "sessao-a", "api", 2_000);
    ativou(&mut motor, Some("f00d02"), 3_000);
    prompt_em(&mut motor, "sessao-b", "web", 4_000);
    pronto_em(&mut motor, "sessao-a", "api", 5_000);
    hook_em(&mut motor, "sessao-b", "web", "PermissionRequest", 6_000);
    ativou(&mut motor, Some("f00d03"), 7_000);
    janela.desktop.janelas = alcas(&["f00d01", "f00d02"]);
    janela.mostrou();
    // Dois cliques antes de o socket2 contar a primeira ativação.
    let b = motor.clicar(&mut janela, Botao::Esquerdo, 8_000);
    let a = motor.clicar(&mut janela, Botao::Esquerdo, 8_200);
    assert!(
        matches!(&b, Clicou::Focou { janela, .. } if janela == "f00d02"),
        "{b:?}"
    );
    assert!(
        matches!(&a, Clicou::Focou { janela, .. } if janela == "f00d01"),
        "{a:?}"
    );
    assert_eq!(
        motor.painel(Some(&janela), 8_200).focando.as_deref(),
        Some("f00d01"),
        "o mais novo"
    );
    // O Hyprland foca as duas, em ordem: os dois avisos saem.
    ativou(&mut motor, Some("f00d02"), 8_300);
    ativou(&mut motor, Some("f00d01"), 8_310);
    assert!(pendentes(&motor).is_empty(), "{:?}", pendentes(&motor));
    assert_eq!(motor.painel(Some(&janela), 8_400).focando, None);
}

// --- a pegada perdida e o cursor (decisão 0063) ------------------------------

#[test]
fn a_pegada_perdida_solta_o_pet_e_ele_nao_anda_sem_aperto() {
    // Numa área de trabalho vazia (sem superfície com o teclado), o Hyprland
    // não segura o ponteiro: o `leave` vem no meio do arraste, e o soltar vai
    // para o outro monitor.
    use crate::plataforma::EventoPonteiro;
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let corpo = janela.toque.unwrap();
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), 0);
    ponteiro(&mut motor, &mut janela, moveu(x - 60, y), 10);
    assert!(motor.arrastando());
    let arrastado = motor.painel(Some(&janela), 10).sprite_disp.unwrap();
    janela.mostrou();
    ponteiro(&mut motor, &mut janela, EventoPonteiro::Saiu, 20);
    assert!(!motor.arrastando());
    let p = motor.painel(Some(&janela), 20);
    assert_eq!(
        p.reacao.as_deref(),
        Some(SOLTO),
        "pousou, sem o laço do voo"
    );
    let toque = janela.toque.unwrap();
    assert_eq!(
        (toque.w, toque.h),
        (corpo.w, corpo.h),
        "toque de novo no corpo"
    );
    // O ponteiro volta sem botão nenhum: o pet fica onde pousou.
    janela.mostrou();
    ponteiro(
        &mut motor,
        &mut janela,
        EventoPonteiro::Entrou { x: x - 300, y },
        30,
    );
    ponteiro(&mut motor, &mut janela, moveu(x - 500, y - 100), 40);
    assert_eq!(motor.painel(Some(&janela), 40).sprite_disp, Some(arrastado));
    assert!(
        !motor.arraste.segurando() && motor.arraste.prazo().is_none(),
        "sem arraste nem fail-safe armado"
    );
}

#[test]
fn o_cursor_volta_a_pegar_quando_o_aperto_desiste() {
    use crate::plataforma::{Botao, Cursor, EventoPonteiro};
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    let (x, y) = meio_do_corpo(&janela);
    let direito = |apertou: bool, x: i32| {
        if apertou {
            EventoPonteiro::Apertou {
                botao: Botao::Direito,
                x,
                y,
            }
        } else {
            EventoPonteiro::Soltou {
                botao: Botao::Direito,
                x,
                y,
            }
        }
    };
    // O direito anda e solta: nem clique nem arraste, e o cursor volta.
    ponteiro(&mut motor, &mut janela, direito(true, x), 0);
    assert_eq!(janela.cursor, Some(Cursor::Agarrar));
    ponteiro(&mut motor, &mut janela, moveu(x + 40, y), 10);
    ponteiro(&mut motor, &mut janela, direito(false, x + 40), 20);
    assert_eq!(janela.cursor, Some(Cursor::Pegar));
    assert_eq!(motor.soneca(30), None, "não foi clique");
    // O direito esquecido apertado: o fail-safe larga e o cursor volta.
    ponteiro(&mut motor, &mut janela, direito(true, x), 100);
    assert_eq!(janela.cursor, Some(Cursor::Agarrar));
    motor.vencer(&mut janela, 100 + arraste::SEM_PONTEIRO_MS);
    assert_eq!(janela.cursor, Some(Cursor::Pegar));
}

// --- o orçamento de commits das peças do M4 (decisões 0005 e 0064) -----------

/// Simula o laço com o compositor mostrando cada quadro na hora (o melhor
/// caso para gastar commits): vence os prazos do Motor de `de` até `ate` e
/// devolve a hora de cada quadro enviado.
fn quadros_entre(motor: &mut Motor, janela: &mut Falsa, de: u64, ate: u64) -> Vec<u64> {
    let mut horas = Vec::new();
    let mut t = de;
    for _ in 0..1_000_000 {
        let Some(prazo) = motor.proximo_prazo().filter(|&p| p <= ate) else {
            return horas;
        };
        t = prazo.max(t);
        janela.mostrou();
        let antes = janela.quadros();
        if motor.prazo_do_cerebro().is_some_and(|p| p <= t) {
            motor.tique(em(t));
        }
        motor.vencer(janela, t);
        horas.extend(std::iter::repeat_n(t, janela.quadros() - antes));
    }
    panic!("um milhão de prazos até {ate}: o laço giraria");
}

/// Como [`quadros_entre`], tocando as reações dos prazos do cérebro como o
/// laço do daemon (a festa, a rajada, o bocejo): o pior caso de verdade.
fn quadros_como_o_laco(motor: &mut Motor, janela: &mut Falsa, de: u64, ate: u64) -> Vec<u64> {
    let mut horas = Vec::new();
    let mut t = de;
    for _ in 0..1_000_000 {
        let Some(prazo) = motor.proximo_prazo().filter(|&p| p <= ate) else {
            return horas;
        };
        t = prazo.max(t);
        janela.mostrou();
        let antes = janela.quadros();
        if motor.prazo_do_cerebro().is_some_and(|p| p <= t) {
            for r in motor.tique(em(t)) {
                janela.mostrou();
                motor.reagir(Some(&mut *janela), r.nome, t);
            }
        }
        janela.mostrou();
        motor.vencer(janela, t);
        horas.extend(std::iter::repeat_n(t, janela.quadros() - antes));
    }
    panic!("um milhão de prazos até {ate}: o laço giraria");
}

/// Dois Motores ligados e assentados até 1 s, com o compositor mostrando
/// tudo: um para a peça, outro de controle.
fn dois_ligados() -> ((Motor, Falsa), (Motor, Falsa)) {
    let (mut a, mut ja) = ligado();
    let (mut b, mut jb) = ligado();
    ja.mostrou();
    jb.mostrou();
    quadros_entre(&mut a, &mut ja, 0, 1_000);
    quadros_entre(&mut b, &mut jb, 0, 1_000);
    ja.mostrou();
    jb.mostrou();
    ((a, ja), (b, jb))
}

#[test]
fn o_balao_custa_dois_quadros() {
    let ((mut a, mut ja), (mut b, mut jb)) = dois_ligados();
    let antes = ja.quadros();
    a.mostrar_balao(Some(&mut ja), vec!["oi".into()], 1_000);
    let com = ja.quadros() - antes + quadros_entre(&mut a, &mut ja, 1_000, 60_000).len();
    let sem = quadros_entre(&mut b, &mut jb, 1_000, 60_000).len();
    assert_eq!(com, sem + 2, "aparecer e sumir (decisão 0052)");
}

#[test]
fn o_selo_zz_parado_nao_custa_nada_na_soneca_e_um_quadro_no_fim() {
    // A soneca contra só o bocejo, no mesmo instante: o selo aparece no
    // primeiro quadro do bocejo, fica parado 30 min e some num quadro. Um
    // erro pendente nos dois os mantém acordados (o sono anima a base,
    // decisão 0082, e o sono profundo cairia no mesmo instante do fim da
    // soneca).
    use crate::plataforma::EventoPonteiro;
    let ((mut a, mut ja), (mut b, mut jb)) = dois_ligados();
    for m in [&mut a, &mut b] {
        hook_em(m, "s1", "api", "UserPromptSubmit", 1_000);
        hook_em(m, "s1", "api", "StopFailure", 1_000);
    }
    let (x, y) = meio_do_corpo(&ja);
    let antes_a = ja.quadros();
    for evento in [
        EventoPonteiro::Apertou {
            botao: Botao::Direito,
            x,
            y,
        },
        EventoPonteiro::Soltou {
            botao: Botao::Direito,
            x,
            y,
        },
    ] {
        ponteiro(&mut a, &mut ja, evento, 1_000);
    }
    assert!(a.soneca(1_000).is_some());
    let antes_b = jb.quadros();
    b.tocar(Some(&mut jb), BOCEJO, 1_000);
    let fim = 1_000 + SONECA_MS + 60_000;
    let com = ja.quadros() - antes_a + quadros_entre(&mut a, &mut ja, 1_000, fim).len();
    let sem = jb.quadros() - antes_b + quadros_entre(&mut b, &mut jb, 1_000, fim).len();
    assert_eq!(com, sem + 1, "só o quadro de sumir (decisão 0053)");
}

#[test]
fn o_coracao_custa_no_maximo_dois_quadros() {
    // O clique que foca o terminal contra só a risadinha, no mesmo instante.
    let ((mut a, mut ja), (mut b, mut jb)) = dois_ligados();
    a.acertar_relogio(em(0));
    ligar_desktop(&mut a, 0);
    ocioso(&mut a, false, 0);
    ativou(&mut a, Some("f00d01"), 100);
    prompt_em(&mut a, "sessao-a", "api", 200);
    pronto_em(&mut a, "sessao-a", "api", 300);
    ativou(&mut a, Some("f00d03"), 900);
    ja.desktop.janelas = alcas(&["f00d01"]);
    let antes_a = ja.quadros();
    assert!(matches!(
        a.clicar(&mut ja, Botao::Esquerdo, 1_000),
        Clicou::Focou { .. }
    ));
    ativou(&mut a, Some("f00d01"), 1_010);
    let antes_b = jb.quadros();
    b.tocar(Some(&mut jb), RISADINHA, 1_000);
    let com = ja.quadros() - antes_a + quadros_entre(&mut a, &mut ja, 1_000, 60_000).len();
    let sem = jb.quadros() - antes_b + quadros_entre(&mut b, &mut jb, 1_000, 60_000).len();
    assert!(
        (sem + 1..=sem + 2).contains(&com),
        "o coração: {com} contra {sem} (decisão 0057)"
    );
}

#[test]
fn o_poof_fica_abaixo_de_30_quadros_por_segundo() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    quadros_entre(&mut motor, &mut janela, 0, 1_000);
    foco(&mut motor, &mut janela, "HDMI-A-1", 1_000);
    let saida = quadros_entre(&mut motor, &mut janela, 1_000, 1_540);
    assert_eq!(saida.len(), poof::PASSOS as usize, "{saida:?}");
    saiu(&mut motor, &mut janela, 1_600);
    janela.pronta = Some(hdmi());
    janela.mostrou();
    let antes = janela.quadros();
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 1_700);
    let mut chegada = vec![1_700; janela.quadros() - antes];
    chegada.extend(quadros_entre(&mut motor, &mut janela, 1_700, 2_000));
    for horas in [&saida, &chegada] {
        for par in horas.windows(2) {
            assert!(
                par[1] - par[0] >= animador::DURACAO_MIN_MS,
                "mais de 30 quadros por segundo: {horas:?}"
            );
        }
    }
}

#[test]
fn o_arraste_faz_commits_no_ritmo_do_ponteiro_e_para_ao_soltar() {
    let (mut motor, mut janela) = ligado();
    janela.mostrou();
    quadros_entre(&mut motor, &mut janela, 0, 1_000);
    let (x, y) = meio_do_corpo(&janela);
    let antes = janela.quadros();
    ponteiro(&mut motor, &mut janela, apertou(x, y), 1_000);
    // 50 movimentos a ~60 Hz, cada quadro mostrado antes do próximo.
    for i in 1..=50 {
        janela.mostrou();
        ponteiro(
            &mut motor,
            &mut janela,
            moveu(x - 10 * i, y),
            1_000 + 16 * i as u64,
        );
    }
    janela.mostrou();
    ponteiro(&mut motor, &mut janela, soltou(x - 500, y), 1_900);
    assert!(
        janela.quadros() - antes <= 51 + 1,
        "no máximo um quadro por movimento: {}",
        janela.quadros() - antes
    );
    // Depois do pouso, só o repouso: até 2 commits por segundo em média.
    let depois = quadros_entre(&mut motor, &mut janela, 1_900, 61_900);
    let pouso = depois.iter().filter(|&&t| t < 3_900).count();
    let repouso = depois.len() - pouso;
    assert!(pouso <= 5, "o pouso: {pouso} quadros");
    assert!(
        repouso as u64 <= 58 * animador::COMMITS_POR_S_PARADO,
        "{repouso} quadros em 58 s"
    );
    assert!(motor.arraste.prazo().is_none(), "nada do arraste armado");
}

// --- a origem do prompt e a janela da sessão (decisão 0073) ---------------

fn prompt_de(sid: &str, orig: &str, ms: u64) -> Evento {
    Evento {
        orig: Some(orig.into()),
        ..evento_de("UserPromptSubmit", sid, None, ms)
    }
}

#[test]
fn so_o_prompt_digitado_casa_a_janela_da_sessao() {
    // O 2.1.288 não manda o `source`: a notificação de uma tarefa e um
    // prompt que chega com o Renan longe do teclado não casam a sessão com a
    // janela em foco (o navegador); o prompt digitado, com ele presente, sim.
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    let p = prompt_de("sessao-a", "comum", 2_000);
    motor.evento(&p, PAREDE + 2_000, em(2_000));
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d01")
    );
    ativou(&mut motor, Some("f00d03"), 3_000);
    let aviso = prompt_de("sessao-a", "notificacao", 40_000);
    motor.evento(&aviso, PAREDE + 40_000, em(40_000));
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d01"),
        "a notificação não casa"
    );
    ocioso(&mut motor, true, 50_000);
    let longe = prompt_de("sessao-a", "comum", 60_000);
    motor.evento(&longe, PAREDE + 60_000, em(60_000));
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d01"),
        "longe do teclado ninguém digitou"
    );
    ocioso(&mut motor, false, 70_000);
    let digitado = prompt_de("sessao-a", "comum", 80_000);
    motor.evento(&digitado, PAREDE + 80_000, em(80_000));
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d03"),
        "o prompt digitado troca (o --resume noutro terminal)"
    );
}

#[test]
fn o_tique_do_laco_pela_janela_certa_fora_de_foco() {
    // Com agendamento pendente (o Stop com crn = 1), um prompt comum com a
    // janela certa da sessão fora de foco é um tique: discreto, e a janela
    // da sessão fica.
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    ativou(&mut motor, Some("f00d01"), 1_000);
    let p = prompt_de("sessao-a", "comum", 2_000);
    motor.evento(&p, PAREDE + 2_000, em(2_000));
    let parar = Evento {
        crn: Some(1),
        bg: Some(0),
        ..evento_de("Stop", "sessao-a", None, 2_000)
    };
    motor.evento(&parar, PAREDE + 3_000, em(3_000));
    assert!(!motor.tique(em(3_800))[0].discreta);
    ativou(&mut motor, Some("f00d03"), 10_000);
    let tique = prompt_de("sessao-a", "comum", 60_000);
    motor.evento(&tique, PAREDE + 60_000, em(60_000));
    let parar = Evento {
        crn: Some(1),
        bg: Some(0),
        tool: None,
        ..evento_de("Stop", "sessao-a", None, 60_000)
    };
    let editou = Evento {
        tool: Some("Edit".into()),
        arq: Some("aaaaaaaaaaa1".into()),
        dur: Some(20),
        ..evento_de("PostToolUse", "sessao-a", None, 60_000)
    };
    motor.evento(&editou, PAREDE + 60_500, em(60_500));
    motor.evento(&parar, PAREDE + 61_000, em(61_000));
    let reacoes = motor.tique(em(61_800));
    assert_eq!(reacoes.len(), 1);
    assert!(reacoes[0].discreta, "o tique é de máquina");
    assert_eq!(
        janela_da(&motor, "sessao-a").unwrap().endereco.as_deref(),
        Some("f00d01")
    );
    let resumo = motor.resumo();
    assert_eq!(resumo.turnos[0].origem, cerebro::OrigemTurno::Tique);
}

// --- avisos de espera e a escalada (decisão 0075) ---------------------------

/// As intenções desde `t` (inclusive), em JSON, sem as do turno, da festa
/// e da tela (a base e os selos).
fn chamadas_desde(motor: &Motor, t: u64) -> Vec<String> {
    motor
        .intencoes()
        .filter(|i| i.t_ms >= t)
        .filter(|i| {
            !matches!(
                i.tipo,
                intencoes::Tipo::Turno { .. }
                    | intencoes::Tipo::Base { .. }
                    | intencoes::Tipo::Selos(_)
                    | intencoes::Tipo::Festa { .. }
            )
        })
        .map(|i| {
            // O "t" é sempre o primeiro campo: fora ele, na ordem da linha.
            let linha = serde_json::to_string(i).unwrap();
            let (_, resto) = linha.split_once(',').unwrap();
            format!("{{{resto}")
        })
        .collect()
}

fn nomes(reacoes: &[Reacao]) -> Vec<&'static str> {
    reacoes.iter().map(|r| r.nome).collect()
}

#[test]
fn a_chamada_toca_na_l1_e_a_escalada_segue_o_aviso_mais_velho() {
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    ocioso(&mut motor, false, 0);
    prompt_em(&mut motor, "sessao-a", "api", 1_000);
    hook_em(&mut motor, "sessao-a", "api", "PermissionRequest", 2_000);
    assert_eq!(
        chamadas_desde(&motor, 2_000),
        vec![
            r#"{"i":"reacao","nome":"alert","motivo":"aviso","sid8":"sessao-a"}"#,
            r#"{"i":"balao","linhas":["Ô, meu camarada!","api precisa de você"],"motivo":"aviso"}"#,
            r#"{"i":"escalada","sid8":"sessao-a","nivel":1,"espera":"permissao","motivo":"aviso"}"#,
        ]
    );
    assert_eq!(
        motor.balao(2_000).map(|b| b.linhas[1].as_str()),
        Some("api precisa de você"),
        "o balão vai para a tela"
    );
    // Um segundo aviso chama na hora dele, mas não escala: vira o "+N".
    prompt_em(&mut motor, "sessao-b", "web", 3_000);
    let ev = Evento {
        e: "PermissionRequest".into(),
        sid: Some("sessao-b".into()),
        turno: Some("sessao-b-p".into()),
        ent: Some("cli".into()),
        proj: Some("web".into()),
        tool: Some("Bash".into()),
        ts: Some(PAREDE + 10_000),
        ..Evento::default()
    };
    let reacoes = motor.evento(&ev, PAREDE + 10_000, em(10_000));
    assert_eq!(nomes(&reacoes), vec![CHAMADA]);
    assert_eq!(
        chamadas_desde(&motor, 10_000).len(),
        2,
        "a chamada e o balão"
    );
    // A L2 do mais velho: 30 s depois dele, uma rajada a cada 6 s.
    assert_eq!(motor.prazo_do_cerebro(), Some(32_000));
    assert_eq!(nomes(&motor.tique(em(32_000))), vec![CHAMADA]);
    assert_eq!(motor.nivel_da_escalada(), 2);
    assert_eq!(motor.prazo_do_cerebro(), Some(38_000));
    // A respondida (a ferramenta rodou): a escalada dela acaba, e a de B
    // começa do relógio dela (aberta em 10 s: já na L2, na hora).
    let rodou = Evento {
        e: "PostToolUse".into(),
        sid: Some("sessao-a".into()),
        turno: Some("sessao-a-p".into()),
        ent: Some("cli".into()),
        proj: Some("api".into()),
        tool: Some("Bash".into()),
        dur: Some(50),
        ts: Some(PAREDE + 40_000),
        ..Evento::default()
    };
    assert!(motor.evento(&rodou, PAREDE + 40_000, em(40_000)).is_empty());
    assert_eq!(
        chamadas_desde(&motor, 40_000),
        vec![
            r#"{"i":"escalada","sid8":"sessao-a","nivel":0,"motivo":"andou"}"#,
            r#"{"i":"escalada","sid8":"sessao-b","nivel":1,"espera":"permissao","motivo":"vez"}"#,
        ]
    );
    assert_eq!(motor.prazo_do_cerebro(), Some(40_000));
    assert_eq!(nomes(&motor.tique(em(40_000))), vec![CHAMADA]);
    assert_eq!(motor.nivel_da_escalada(), 2);
    // O fim da sessão B leva a escalada junto.
    let fim = Evento {
        e: "SessionEnd".into(),
        sid: Some("sessao-b".into()),
        ent: Some("cli".into()),
        ts: Some(PAREDE + 41_000),
        ..Evento::default()
    };
    motor.evento(&fim, PAREDE + 41_000, em(41_000));
    assert!(chamadas_desde(&motor, 41_000).contains(
        &r#"{"i":"escalada","sid8":"sessao-b","nivel":0,"motivo":"sessao_saiu"}"#.to_owned()
    ));
    assert_eq!(motor.nivel_da_escalada(), 0);
    assert_eq!(motor.prazo_da_escalada(), None);
}

#[test]
fn soneca_e_pet_escondido_seguram_a_escalada() {
    // Na soneca: a chamada vira o aceno e nada passa da L1; acordado, a
    // escalada segue da fase em que está.
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ocioso(&mut motor, false, 0);
    motor.alternar_soneca(&mut janela, 500);
    prompt_em(&mut motor, "sessao-a", "api", 1_000);
    let ev = Evento {
        e: "PermissionRequest".into(),
        sid: Some("sessao-a".into()),
        turno: Some("sessao-a-p".into()),
        ent: Some("cli".into()),
        proj: Some("api".into()),
        tool: Some("Bash".into()),
        ts: Some(PAREDE + 2_000),
        ..Evento::default()
    };
    assert_eq!(
        nomes(&motor.evento(&ev, PAREDE + 2_000, em(2_000))),
        vec![cerebro::ACENO]
    );
    assert_eq!(motor.prazo_da_escalada(), None, "teto L1: nada a vencer");
    motor.tique(em(200_000));
    assert_eq!(motor.nivel_da_escalada(), 1);
    motor.alternar_soneca(&mut janela, 200_000);
    assert_eq!(motor.prazo_da_escalada(), Some(200_000), "acordou: já");
    motor.tique(em(200_000));
    assert_eq!(
        chamadas_desde(&motor, 200_000)
            .iter()
            .filter(|l| l.contains("escalada") || l.contains("voo"))
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            r#"{"i":"escalada","sid8":"sessao-a","nivel":3,"motivo":"tempo"}"#,
            r#"{"i":"voo","destino":"alto_centro","motivo":"escalada","sid8":"sessao-a"}"#,
        ]
    );
    // Escondido: a L1 não toca (nem balão), e o relógio anda.
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    motor.definir_visivel(false);
    prompt_em(&mut motor, "sessao-a", "api", 1_000);
    assert!(motor.evento(&ev, PAREDE + 2_000, em(2_000)).is_empty());
    assert_eq!(
        chamadas_desde(&motor, 2_000),
        vec![
            r#"{"i":"escalada","sid8":"sessao-a","nivel":1,"espera":"permissao","motivo":"aviso"}"#
        ]
    );
    assert!(motor.balao(2_000).is_none());
    assert_eq!(motor.prazo_da_escalada(), None);
    motor.definir_visivel(true);
    motor.tique(em(100_000));
    assert_eq!(
        motor.nivel_da_escalada(),
        3,
        "voltou a aparecer na L3: o voo"
    );
    assert!(
        chamadas_desde(&motor, 100_000)
            .iter()
            .any(|l| l.contains("\"voo\""))
    );
}

#[test]
fn o_clique_que_ve_o_aviso_acaba_a_escalada() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    ativou(&mut motor, Some("f00d01"), 500);
    prompt_em(&mut motor, "sessao-a", "api", 1_000);
    hook_em(&mut motor, "sessao-a", "api", "PermissionRequest", 2_000);
    assert_eq!(motor.nivel_da_escalada(), 1);
    janela.desktop.janelas = alcas(&["f00d01"]);
    janela.mostrou();
    let clicou = motor.clicar(&mut janela, Botao::Esquerdo, 5_000);
    assert!(matches!(
        clicou,
        Clicou::Focou {
            confirmado: true,
            ..
        }
    ));
    assert_eq!(
        chamadas_desde(&motor, 5_000)
            .into_iter()
            .filter(|l| l.contains("escalada"))
            .collect::<Vec<_>>(),
        vec![r#"{"i":"escalada","sid8":"sessao-a","nivel":0,"motivo":"visto"}"#]
    );
    assert_eq!(motor.prazo_da_escalada(), None);
}

#[test]
fn o_clique_que_ve_a_espera_solta_a_pose_e_o_pet_dorme() {
    // Um Esc numa pergunta não manda evento nenhum (conferido no 2.1.288): a
    // sessão fica "esperando". O clique que leva ao terminal dela vê o
    // aviso, e a pose de espera sai junto (decisão 0090): o pet boceja e
    // dorme no prazo de sempre, com nada pendente.
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    ocioso(&mut motor, false, 0);
    ativou(&mut motor, Some("f00d01"), 500);
    prompt_em(&mut motor, "sessao-a", "api", 1_000);
    hook_em(&mut motor, "sessao-a", "api", "PermissionRequest", 2_000);
    assert_eq!(motor.painel(None, 2_000).fotografia.base, "waiting");
    janela.desktop.janelas = alcas(&["f00d01"]);
    janela.mostrou();
    motor.clicar(&mut janela, Botao::Esquerdo, 3_000);
    assert!(pendentes(&motor).is_empty());
    assert_eq!(
        motor.resumo().sessoes[0].estado,
        EstadoSessao::Esperando,
        "o cérebro ainda acha que espera"
    );
    assert_eq!(motor.painel(None, 3_000).fotografia.base, "idle");
    let reacoes = andar(&mut motor, 3_000 + tela::SONO_MS + 1_000);
    assert_eq!(
        reacoes,
        vec![(3_000 + tela::BOCEJO_MS, BOCEJO)],
        "boceja e dorme"
    );
    assert_eq!(
        motor
            .painel(None, 3_000 + tela::SONO_MS + 1_000)
            .fotografia
            .base,
        "sleep"
    );
}

#[test]
fn a_volta_que_ninguem_viu_com_a_sessao_bloqueada_sai_quando_os_quadros_voltam() {
    // Bloqueada, a sessão do Hyprland não mostra a camada: o primeiro quadro
    // do voo fica preso e o voo acaba pelo relógio. A volta do Renan (a
    // senha digitada) não pode se perder ali: quando o compositor mostra o
    // quadro preso (desbloqueou), o voo sai de novo (decisão 0090).
    let l3 = 2_000 + escalada::L3_APOS_MS;
    let (mut motor, mut janela) = esperando_ate(l3 - 1);
    ocioso(&mut motor, true, 10_000);
    quadros_entre(&mut motor, &mut janela, 10_000, l3 + 200_000);
    // Os voos da L3 saíram (a tela ainda mostrava); agora a sessão bloqueia.
    janela.em_voo = true;
    let volta = l3 + 300_000;
    ocioso(&mut motor, false, volta);
    let primeiro = *motor.voo().expect("o voo da volta");
    assert_eq!(primeiro.motivo, "voltou");
    // Nenhum quadro passa do preso: o desenho espera o compositor, e o voo
    // fica "no ar" só pelo relógio.
    for t in (volta..primeiro.fim_ms() + 2_000).step_by(34) {
        motor.vencer(&mut janela, t);
    }
    assert_eq!(motor.voo_quadros, 0);
    // Desbloqueou: o quadro preso é mostrado; o voo que ninguém viu acaba
    // pelo relógio, e o da volta sai de novo, agora na tela.
    let desbloqueio = primeiro.fim_ms() + 5_000;
    janela.mostrou();
    motor.evento_overlay(&mut janela, EventoOverlay::Redesenhar, desbloqueio);
    let segundo = *motor.voo().expect("o voo da volta, agora na tela");
    assert_eq!((segundo.motivo, segundo.inicio_ms), ("voltou", desbloqueio));
    assert!(motor.volta_por_mostrar.is_none());
    // Com os quadros andando, ele acaba visto, e não sai uma terceira vez.
    let fim = segundo.fim_ms();
    quadros_entre(&mut motor, &mut janela, desbloqueio, fim + 1_000);
    assert!(motor.voo().is_none() && motor.volta_por_mostrar.is_none());
    janela.mostrou();
    motor.evento_overlay(&mut janela, EventoOverlay::Redesenhar, fim + 2_000);
    assert!(motor.voo().is_none());
    // Só uma intenção de voo da volta: o desenho não muda as intenções.
    assert_eq!(
        motor
            .intencoes()
            .filter(|i| matches!(
                i.tipo,
                intencoes::Tipo::Voo {
                    motivo: "voltou",
                    ..
                }
            ))
            .count(),
        1
    );
}

#[test]
fn a_janela_que_fecha_no_meio_do_voo_nao_deixa_o_pet_batendo_as_asas() {
    // O monitor sai (o HDMI desplugado) no meio do voo da L3: a janela some,
    // um evento chega antes da nova (o desenho sem palco) e a nova fica
    // pronta. O pet não pode ficar segurando o voo da skin em laço (uns 12
    // commits por segundo no Zeca original).
    let l3 = 2_000 + escalada::L3_APOS_MS;
    let (mut motor, mut janela) = esperando_ate(l3 + 500);
    assert!(motor.voo().is_some(), "voando");
    motor.evento_overlay(&mut janela, EventoOverlay::Sumiu, l3 + 600);
    assert!(motor.voo().is_none());
    assert!(!motor.pet.as_ref().unwrap().segurado(), "largou o voo");
    hook_em(&mut motor, "s9", "lab", "SessionStart", l3 + 700);
    motor.vencer(&mut janela, l3 + 800);
    janela.pronta = Some(edp());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, l3 + 900);
    assert!(!motor.pet.as_ref().unwrap().segurado());
    // E o voo que acaba sem palco também larga.
    let (mut motor, mut janela) = esperando_ate(l3 + 500);
    motor.palco = None;
    motor.desenhar(&mut janela, l3 + 600, false);
    assert!(motor.voo().is_none());
    assert!(!motor.pet.as_ref().unwrap().segurado());
}

#[test]
fn a_volta_so_voa_depois_de_60_s_longe_e_fora_do_terminal_do_claude() {
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    ocioso(&mut motor, false, 0);
    olhando(&mut motor, true, 0);
    prompt_em(&mut motor, "sessao-a", "api", 1_000);
    hook_em(&mut motor, "sessao-a", "api", "PermissionRequest", 2_000);
    assert_eq!(
        motor.prazo_da_escalada(),
        None,
        "olhando o Claude e mexendo"
    );
    // Parou de mexer (o desktop conta 5 s depois): chama aos 60 s parado.
    ocioso(&mut motor, true, 10_000);
    assert_eq!(motor.prazo_da_escalada(), Some(65_000));
    // Voltou com 25 s parado: nem voo, nem prazo.
    ocioso(&mut motor, false, 30_000);
    assert_eq!(motor.prazo_da_escalada(), None);
    assert!(
        !chamadas_desde(&motor, 30_000)
            .iter()
            .any(|l| l.contains("voo"))
    );
    // Parado de novo, até chamar: a fase já é a L3 (o voo).
    ocioso(&mut motor, true, 40_000);
    assert_eq!(motor.prazo_da_escalada(), Some(95_000));
    motor.tique(em(95_000));
    assert_eq!(motor.nivel_da_escalada(), 3);
    // Volta ao terminal do Claude depois de 85 s parado: ele vê o aviso lá,
    // nada de voo.
    ocioso(&mut motor, false, 120_000);
    assert!(
        !chamadas_desde(&motor, 96_000)
            .iter()
            .any(|l| l.contains("voltou"))
    );
    // Parado de novo por 60 s e de volta noutra janela: o voo, na hora.
    ocioso(&mut motor, true, 130_000);
    olhando(&mut motor, false, 131_000);
    motor.tique(em(155_000));
    ocioso(&mut motor, false, 200_000);
    assert!(chamadas_desde(&motor, 200_000).contains(
        &r#"{"i":"voo","destino":"alto_centro","motivo":"voltou","sid8":"sessao-a"}"#.to_owned()
    ));
}

#[test]
fn o_tipo_refinado_troca_o_balao_sem_chamar_de_novo() {
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    prompt_em(&mut motor, "sessao-a", "api", 1_000);
    let notificacao = Evento {
        e: "Notification".into(),
        sid: Some("sessao-a".into()),
        turno: Some("sessao-a-p".into()),
        ent: Some("cli".into()),
        proj: Some("api".into()),
        nt: Some("permission_prompt".into()),
        ts: Some(PAREDE + 2_000),
        ..Evento::default()
    };
    assert_eq!(
        nomes(&motor.evento(&notificacao, PAREDE + 2_000, em(2_000))),
        vec![CHAMADA]
    );
    let pergunta = Evento {
        e: "PreToolUse".into(),
        tool: Some("AskUserQuestion".into()),
        ts: Some(PAREDE + 2_030),
        nt: None,
        ..notificacao
    };
    assert!(
        motor
            .evento(&pergunta, PAREDE + 2_030, em(2_030))
            .is_empty()
    );
    assert_eq!(
        chamadas_desde(&motor, 2_030),
        vec![r#"{"i":"balao","linhas":["api: pergunta pra você"],"motivo":"aviso_refinado"}"#]
    );
    assert_eq!(motor.nivel_da_escalada(), 1);
}

// --- a festa e a tela (decisão 0076) ----------------------------------------

fn hook_de(sid: &str, proj: &str, e: &str, ms: u64) -> Evento {
    Evento {
        e: e.into(),
        sid: Some(sid.into()),
        turno: Some(format!("{sid}-p")),
        ent: Some("cli".into()),
        proj: Some(proj.into()),
        ts: Some(PAREDE + ms),
        ..Evento::default()
    }
}

fn mandar(motor: &mut Motor, ev: Evento, ms: u64) -> Vec<&'static str> {
    nomes(&motor.evento(&ev, PAREDE + ms, em(ms)))
}

/// Vence os prazos do cérebro até `ate`, como o laço; as reações com a hora.
fn andar(motor: &mut Motor, ate: u64) -> Vec<(u64, &'static str)> {
    let mut saida = Vec::new();
    let mut voltas = 0;
    while let Some(p) = motor.prazo_do_cerebro().filter(|p| *p <= ate) {
        voltas += 1;
        assert!(voltas < 1_000, "prazo que não anda em {p}");
        for r in motor.tique(em(p)) {
            saida.push((p, r.nome));
        }
    }
    motor.acertar_relogio(em(ate));
    saida
}

/// Um turno com uma edição: o T1 na acomodação.
fn turno_pequeno(motor: &mut Motor, sid: &str, proj: &str, ms: u64) {
    mandar(motor, hook_de(sid, proj, "UserPromptSubmit", ms), ms);
    let edit = Evento {
        tool: Some("Edit".into()),
        arq: Some(format!("{sid:0>12}")),
        dur: Some(30),
        ..hook_de(sid, proj, "PostToolUse", ms + 1_000)
    };
    mandar(motor, edit, ms + 1_000);
    mandar(motor, hook_de(sid, proj, "Stop", ms + 2_000), ms + 2_000);
}

fn bases(motor: &Motor) -> Vec<(u64, &'static str, bool)> {
    motor
        .intencoes()
        .filter_map(|i| match i.tipo {
            intencoes::Tipo::Base {
                estado, profundo, ..
            } => Some((i.t_ms, estado, profundo)),
            _ => None,
        })
        .collect()
}

#[test]
fn parado_boceja_dorme_e_um_evento_acorda() {
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    ocioso(&mut motor, false, 0);
    assert_eq!(
        andar(&mut motor, 40 * 60_000),
        vec![(tela::BOCEJO_MS, BOCEJO)],
        "o bocejo aos 3 min; o sono é a base, sem reação"
    );
    assert_eq!(
        bases(&motor),
        vec![
            (tela::SONO_MS, "sleep", false),
            (tela::SONO_PROFUNDO_MS, "sleep", true)
        ]
    );
    assert_eq!(motor.prazo_do_cerebro(), None, "no sono profundo, nada");
    // Um evento acorda: o despertar antes de tudo, e o relógio recomeça.
    let t = 40 * 60_000;
    assert_eq!(
        mandar(&mut motor, hook_de("s1", "api", "UserPromptSubmit", t), t),
        vec![DESPERTAR]
    );
    assert_eq!(bases(&motor).last(), Some(&(t, "thinking", false)));
    // Longe do teclado, dorme aos 3 min, logo depois do bocejo.
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    ocioso(&mut motor, true, 0);
    assert_eq!(
        andar(&mut motor, 4 * 60_000),
        vec![(tela::BOCEJO_MS, BOCEJO)]
    );
    assert_eq!(bases(&motor), vec![(tela::SONO_LONGE_MS, "sleep", false)]);
    // O clique acorda sem o despertar (a risadinha toca por cima).
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    andar(&mut motor, 10 * 60_000);
    janela.mostrou();
    motor.clicar(&mut janela, Botao::Esquerdo, 10 * 60_000);
    assert_eq!(bases(&motor).last(), Some(&(10 * 60_000, "idle", false)));
    assert!(
        !motor
            .intencoes()
            .any(|i| matches!(&i.tipo, intencoes::Tipo::Reacao { nome, .. } if nome == DESPERTAR))
    );
    // Um aviso pendente (o erro, a espera) não deixa dormir; o pronto deixa.
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    turno_pequeno(&mut motor, "s1", "api", 1_000);
    andar(&mut motor, 60 * 60_000);
    assert!(
        bases(&motor).iter().any(|(_, e, _)| *e == "sleep"),
        "com o pronto"
    );
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    mandar(
        &mut motor,
        hook_de("s1", "api", "UserPromptSubmit", 1_000),
        1_000,
    );
    mandar(
        &mut motor,
        hook_de("s1", "api", "StopFailure", 2_000),
        2_000,
    );
    andar(&mut motor, 60 * 60_000);
    assert!(
        !bases(&motor).iter().any(|(_, e, _)| *e == "sleep"),
        "com o erro"
    );
}

#[test]
fn a_base_e_da_sessao_mais_alta_e_as_outras_viram_selos() {
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    olhando(&mut motor, true, 0);
    ocioso(&mut motor, false, 0);
    // C termina (pronto), A trabalha, B espera.
    turno_pequeno(&mut motor, "sessao-c", "lab", 0);
    andar(&mut motor, 3_000);
    mandar(
        &mut motor,
        hook_de("sessao-a", "api", "UserPromptSubmit", 4_000),
        4_000,
    );
    let edit = Evento {
        tool: Some("Bash".into()),
        dur: Some(10),
        ..hook_de("sessao-a", "api", "PostToolUse", 5_000)
    };
    mandar(&mut motor, edit, 5_000);
    mandar(
        &mut motor,
        hook_de("sessao-b", "web", "UserPromptSubmit", 6_000),
        6_000,
    );
    hook_em(&mut motor, "sessao-b", "web", "PermissionRequest", 7_000);
    let foto = motor.painel(None, 7_000).fotografia;
    assert_eq!(
        (foto.base, foto.sid8.as_deref()),
        ("waiting", Some("sessao-b"))
    );
    assert_eq!(
        foto.selos,
        Selos {
            mais: 1,
            bandeiras: vec![tela::cor("lab")],
            corrente: false
        }
    );
    assert_eq!(foto.escalada.as_ref().map(|e| e.nivel), Some(1));
    // B respondida: o pronto de C ainda segura a base (até 2 min).
    let rodou = Evento {
        tool: Some("Bash".into()),
        dur: Some(10),
        ..hook_de("sessao-b", "web", "PostToolUse", 8_000)
    };
    mandar(&mut motor, rodou, 8_000);
    let foto = motor.painel(None, 8_000).fotografia;
    assert_eq!(
        (foto.base, foto.sid8.as_deref()),
        ("ready", Some("sessao-c"))
    );
    assert_eq!(foto.selos.mais, 2, "A e B trabalhando");
    assert!(
        foto.selos.bandeiras.is_empty(),
        "o pronto de C está na base"
    );
    // Passados 2 min, o pronto vira a bandeirinha e a base é o trabalho.
    andar(&mut motor, 3_000 + tela::PRONTO_NA_BASE_MS);
    let foto = motor
        .painel(None, 3_000 + tela::PRONTO_NA_BASE_MS)
        .fotografia;
    assert_eq!(foto.base, "working");
    assert_eq!(foto.selos.bandeiras, vec![tela::cor("lab")]);
    assert!(foto.escalada.is_none());
}

#[test]
fn a_festa_com_o_nao_perturbe_a_soneca_e_o_modo_discreto() {
    let grande = |motor: &mut Motor, ms: u64, dnd: bool| {
        let com = |e: Evento| Evento { dnd, ..e };
        mandar(motor, com(hook_de("s1", "api", "UserPromptSubmit", ms)), ms);
        let sub = Evento {
            agente: false,
            aid: Some("x1".into()),
            ..com(hook_de("s1", "api", "SubagentStart", ms + 100))
        };
        mandar(motor, sub, ms + 100);
        for i in 0..20 {
            let edit = Evento {
                tool: Some("Edit".into()),
                arq: Some(format!("{i:0>12}")),
                dur: Some(60_000),
                ..com(hook_de("s1", "api", "PostToolUse", ms + 1_000 + i))
            };
            mandar(motor, edit, ms + 1_000 + i);
        }
        mandar(
            motor,
            com(hook_de("s1", "api", "Stop", ms + 2_000)),
            ms + 2_000,
        );
        andar(motor, ms + 3_000)
    };
    let festa = |motor: &Motor| {
        motor
            .intencoes()
            .filter_map(|i| match &i.tipo {
                intencoes::Tipo::Festa {
                    reacao,
                    voo,
                    confete,
                    faixa,
                    ..
                } => Some((*reacao, *voo, *confete, *faixa)),
                _ => None,
            })
            .last()
    };
    // Com o "não perturbe": o T3 sem o voo pela tela.
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    assert_eq!(
        grande(&mut motor, 0, true),
        vec![(2_800, cerebro::VOO_GRANDE)]
    );
    assert_eq!(festa(&motor), Some((Some("done_big"), None, 40, true)));
    // Na soneca: o aceno, sem confete, voo nem faixa.
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    motor.alternar_soneca(&mut janela, 0);
    assert_eq!(grande(&mut motor, 0, false), vec![(2_800, cerebro::ACENO)]);
    assert_eq!(festa(&motor), Some((Some("nod"), None, 0, false)));
    // No modo discreto: o pulinho sem balão, e o segundo fim dentro de uma
    // festa só fica no registro.
    let config = ConfigCerebro {
        modo: crate::config::ModoCelebracao::Discreta,
        ..ConfigCerebro::default()
    };
    let mut motor = Motor::novo(config);
    motor.acertar_relogio(em(0));
    turno_pequeno(&mut motor, "s1", "api", 0);
    assert_eq!(andar(&mut motor, 3_000), vec![(2_800, cerebro::PULINHO)]);
    assert!(motor.balao(3_000).is_none(), "sem balão");
    assert!(festa(&motor).is_none(), "não é festa");
}

#[test]
fn um_fim_t0_dentro_da_festa_so_muda_o_balao() {
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    turno_pequeno(&mut motor, "s1", "api", 0);
    mandar(
        &mut motor,
        hook_de("s2", "web", "UserPromptSubmit", 500),
        500,
    );
    assert_eq!(andar(&mut motor, 2_800), vec![(2_800, cerebro::PULINHO)]);
    // s2 responde sem ferramenta (T0) 1 s depois: entra na festa, sem tocar.
    mandar(&mut motor, hook_de("s2", "web", "Stop", 3_000), 3_000);
    assert!(andar(&mut motor, 3_800).is_empty());
    assert_eq!(
        motor.balao(3_800).map(|b| b.linhas.clone()),
        Some(vec!["2 prontos: api, web".to_owned()])
    );
    let foto = motor.painel(None, 3_800).fotografia;
    assert_eq!(foto.festa.map(|f| (f.sessoes, f.ha_ms)), Some((2, 1_000)));
}

// --- o compartilhamento de tela no 0.56.2 (decisão 0081) -------------------

fn compartilhar(motor: &mut Motor, sinal: bool, ms: u64) {
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::Compartilhando(sinal),
        em(ms),
    );
}

fn discricoes(motor: &Motor) -> Vec<(u64, bool)> {
    motor
        .intencoes()
        .filter_map(|i| match i.tipo {
            intencoes::Tipo::Discricao { ligada, .. } => Some((i.t_ms, ligada)),
            _ => None,
        })
        .collect()
}

#[test]
fn uma_captura_de_tela_nao_liga_a_discricao() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    // O grim: o sinal aceso por 400 ms. Passado um minuto, o episódio acaba.
    compartilhar(&mut motor, true, 1_000);
    compartilhar(&mut motor, false, 1_400);
    andar(&mut motor, 120_000);
    assert!(discricoes(&motor).is_empty());
    assert!(!motor.painel(None, 120_000).fotografia.discricao);
    // Outra captura, mais de um minuto depois, não soma com a primeira.
    for inicio in [130_000, 135_000, 140_000, 145_000] {
        compartilhar(&mut motor, true, inicio);
        compartilhar(&mut motor, false, inicio + 400);
    }
    andar(&mut motor, 250_000);
    assert!(discricoes(&motor).is_empty(), "1,6 s somados");
    // Cinco seguidas, a menos de um minuto uma da outra, somam 2 s: na
    // dúvida, discreto (os nomes voltam 5 min depois do último sinal).
    for inicio in [260_000, 270_000, 280_000, 290_000, 300_000] {
        compartilhar(&mut motor, true, inicio);
        compartilhar(&mut motor, false, inicio + 400);
    }
    andar(&mut motor, 700_000);
    assert_eq!(
        discricoes(&motor),
        vec![(300_400, true), (300_400 + tela::SEGURA_MS, false)]
    );
}

#[test]
fn a_tela_parada_que_pisca_liga_e_segura_a_discricao() {
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    // Uma chamada com a tela parada: o Hyprland manda `0` meio segundo
    // depois do último quadro copiado e `1` no próximo desenho (600 ms de
    // sinal a cada 5 s). A discrição liga quando o sinal soma 2 s.
    for n in 0..11 {
        let inicio = 10_000 + n * 5_000;
        compartilhar(&mut motor, true, inicio);
        andar(&mut motor, inicio + 600);
        compartilhar(&mut motor, false, inicio + 600);
        andar(&mut motor, inicio + 5_000);
    }
    assert_eq!(
        discricoes(&motor),
        vec![(25_200, true)],
        "nunca desliga no `0`"
    );
    // Uma festa no meio: sem o nome do projeto.
    turno_pequeno(&mut motor, "s1", "agenda-secreta", 70_000);
    andar(&mut motor, 73_000);
    assert_eq!(
        motor.balao(73_000).map(|b| b.linhas.clone()),
        Some(vec!["Prontinho!".to_owned()])
    );
    // O último sinal foi aos 60,6 s: os nomes voltam 5 min depois.
    let fim = 60_600 + tela::SEGURA_MS;
    andar(&mut motor, fim + 1_000);
    assert_eq!(discricoes(&motor), vec![(25_200, true), (fim, false)]);
    turno_pequeno(&mut motor, "s2", "agenda-secreta", fim + 10_000);
    andar(&mut motor, fim + 13_000);
    assert_eq!(
        motor.balao(fim + 13_000).map(|b| b.linhas.clone()),
        Some(vec!["Prontinho! agenda-secreta".to_owned()])
    );
}

#[test]
fn a_fonte_que_cai_desliga_o_sinal_e_a_discricao_segura() {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    ligar_desktop(&mut motor, 0);
    compartilhar(&mut motor, true, 1_000);
    andar(&mut motor, 5_000);
    assert_eq!(discricoes(&motor), vec![(3_000, true)]);
    // O socket2 cai: o fim do compartilhamento pode se perder no meio.
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::Ligado(false),
        em(10_000),
    );
    assert!(!motor.desktop().compartilhando);
    ligar_desktop(&mut motor, 11_000);
    andar(&mut motor, 200_000);
    assert_eq!(discricoes(&motor).len(), 1, "segura");
    // Voltou compartilhando: segue discreto. Parou: 5 min depois do fim.
    compartilhar(&mut motor, true, 200_000);
    compartilhar(&mut motor, false, 250_000);
    andar(&mut motor, 250_000 + tela::SEGURA_MS + 1);
    assert_eq!(
        discricoes(&motor),
        vec![(3_000, true), (250_000 + tela::SEGURA_MS, false)]
    );
    // Sem o sinal voltar, a discrição acaba sozinha (nunca fica para sempre).
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.acertar_relogio(em(0));
    compartilhar(&mut motor, true, 0);
    andar(&mut motor, 3_000);
    motor.evento_desktop(
        None,
        &crate::plataforma::EventoDesktop::Ligado(false),
        em(4_000),
    );
    andar(&mut motor, 4_000 + tela::SEGURA_MS);
    assert_eq!(
        discricoes(&motor),
        vec![(2_000, true), (4_000 + tela::SEGURA_MS, false)]
    );
    assert_eq!(motor.prazo_da_discricao(5_000 + tela::SEGURA_MS), None);
}

// --- a base segurada (decisão 0082) -----------------------------------------

fn base_do_pet(motor: &Motor) -> (String, crate::animador::Ritmo) {
    let base = motor.pet.as_ref().expect("o pet").base();
    (base.estado.clone(), base.ritmo)
}

#[test]
fn o_animador_segura_a_base_da_tela_no_ritmo_dela() {
    use crate::animador::Ritmo;
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    ocioso(&mut motor, false, 0);
    janela.mostrou();
    quadros_entre(&mut motor, &mut janela, 0, 1_000);
    assert_eq!(base_do_pet(&motor), ("idle".into(), Ritmo::Repouso));
    mandar(
        &mut motor,
        hook_de("s1", "api", "UserPromptSubmit", 1_000),
        1_000,
    );
    assert_eq!(base_do_pet(&motor), ("thinking".into(), Ritmo::Quieto));
    // A pose nova vai para a tela já, no próximo quadro.
    assert_eq!(motor.prazo_da_animacao(), Some(1_000));
    let pose = |motor: &Motor, estado: &str| {
        let skin = motor.skin.as_ref().unwrap();
        let tag = skin.tags_do_estado(estado)[0];
        skin.canonico[skin.tags[tag].de]
    };
    let quadro = |janela: &Falsa| match janela.cena.as_ref().unwrap()[0] {
        Elemento::Sprite { quadro, .. } => quadro,
        _ => panic!("o sprite primeiro"),
    };
    quadros_entre(&mut motor, &mut janela, 1_000, 2_000);
    assert_eq!(quadro(&janela), pose(&motor, "thinking"));
    let bash = Evento {
        tool: Some("Bash".into()),
        dur: Some(10),
        ..hook_de("s1", "api", "PostToolUse", 2_000)
    };
    mandar(&mut motor, bash, 2_000);
    assert_eq!(base_do_pet(&motor), ("working".into(), Ritmo::Quieto));
    quadros_entre(&mut motor, &mut janela, 2_000, 3_000);
    assert_eq!(quadro(&janela), pose(&motor, "working"));
    mandar(&mut motor, hook_de("s1", "api", "Stop", 3_000), 3_000);
    quadros_entre(&mut motor, &mut janela, 3_000, 4_000);
    assert_eq!(base_do_pet(&motor), ("ready".into(), Ritmo::Repouso));
    // O pronto vira bandeirinha em 2 min, o pet dorme 8 min depois (o laço
    // do sono) e entra no sono profundo aos 30 min: só a pose, sem commit.
    quadros_entre(&mut motor, &mut janela, 4_000, 700_000);
    assert_eq!(base_do_pet(&motor), ("sleep".into(), Ritmo::Laco));
    quadros_entre(&mut motor, &mut janela, 700_000, 1_930_000);
    assert_eq!(base_do_pet(&motor), ("sleep".into(), Ritmo::Parado));
    let profundo = quadros_entre(&mut motor, &mut janela, 1_930_000, 1_930_000 + 30 * 60_000);
    assert!(
        profundo.is_empty(),
        "{} commits no sono profundo",
        profundo.len()
    );
    assert_eq!(motor.prazo_da_animacao(), None);
    // Um evento acorda: a base volta a andar.
    mandar(
        &mut motor,
        hook_de("s2", "web", "UserPromptSubmit", 3_800_000),
        3_800_000,
    );
    assert_eq!(base_do_pet(&motor), ("thinking".into(), Ritmo::Quieto));
}

#[test]
fn no_teto_da_escalada_a_espera_fica_so_na_pose() {
    use crate::animador::Ritmo;
    let (mut motor, _janela) = ligado();
    motor.acertar_relogio(em(0));
    olhando(&mut motor, false, 0);
    ocioso(&mut motor, false, 0);
    mandar(
        &mut motor,
        hook_de("s1", "api", "UserPromptSubmit", 1_000),
        1_000,
    );
    hook_em(&mut motor, "s1", "api", "PermissionRequest", 2_000);
    assert_eq!(base_do_pet(&motor), ("waiting".into(), Ritmo::Repouso));
    andar(&mut motor, 2_000 + escalada::L4_APOS_MS);
    assert_eq!(motor.nivel_da_escalada(), 4);
    assert_eq!(base_do_pet(&motor), ("waiting".into(), Ritmo::Parado));
    // Respondida, a espera sai e a base anda de novo.
    let rodou = Evento {
        tool: Some("Bash".into()),
        dur: Some(10),
        ..hook_de("s1", "api", "PostToolUse", 400_000)
    };
    mandar(&mut motor, rodou, 400_000);
    assert_eq!(base_do_pet(&motor), ("working".into(), Ritmo::Quieto));
}

// --- os selos e o selo do aviso (decisão 0083) --------------------------------

/// As cores dos blocos da cena.
fn cores_na_cena(janela: &Falsa) -> Vec<[u8; 4]> {
    janela
        .cena
        .as_ref()
        .map(|c| {
            c.iter()
                .filter_map(|e| match e {
                    Elemento::Bloco { cor, .. } => Some(*cor),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// O texto dos glifos da cena, sem a sombra repetida.
fn caracteres_na_cena(janela: &Falsa) -> String {
    janela
        .cena
        .as_ref()
        .map(|c| {
            c.iter()
                .filter_map(|e| match e {
                    Elemento::Glifo { c, cor, .. } if *cor != [0x2A, 0x1B, 0x1D, 0xFF] => Some(*c),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn os_selos_das_outras_sessoes_vao_para_a_tela_ao_lado_do_corpo() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    olhando(&mut motor, true, 0);
    ocioso(&mut motor, false, 0);
    // C termina (pronto), A trabalha, B pensa.
    turno_pequeno(&mut motor, "sessao-c", "lab", 0);
    andar(&mut motor, 3_000);
    mandar(
        &mut motor,
        hook_de("sessao-a", "api", "UserPromptSubmit", 4_000),
        4_000,
    );
    let bash = Evento {
        tool: Some("Bash".into()),
        dur: Some(10),
        ..hook_de("sessao-a", "api", "PostToolUse", 5_000)
    };
    mandar(&mut motor, bash, 5_000);
    mandar(
        &mut motor,
        hook_de("sessao-b", "web", "UserPromptSubmit", 6_000),
        6_000,
    );
    // Depois dos 2 min do pronto: a base é o trabalho de A, o "+1" é B e a
    // bandeirinha é o pronto de C.
    janela.mostrou();
    quadros_entre(
        &mut motor,
        &mut janela,
        6_000,
        3_000 + tela::PRONTO_NA_BASE_MS + 10,
    );
    let foto = motor
        .painel(None, 3_000 + tela::PRONTO_NA_BASE_MS + 10)
        .fotografia;
    assert_eq!(foto.selos.mais, 1);
    assert_eq!(foto.selos.bandeiras, vec![tela::cor("lab")]);
    assert!(cores_na_cena(&janela).contains(&selos::PALETA[usize::from(tela::cor("lab"))]));
    assert_eq!(caracteres_na_cena(&janela), "+1");
    // Nada em cima da área de toque, tudo dentro do monitor.
    let toque = janela.toque.expect("o toque no corpo");
    for e in janela.cena.as_ref().unwrap().iter().skip(1) {
        if let Elemento::Bloco { ret, .. } = e {
            assert!(
                ret.intersecao(&toque).is_none(),
                "{ret:?} sobre o corpo {toque:?}"
            );
            assert!(ret.x >= 0 && ret.y >= 0 && ret.direita() <= 1920 && ret.baixo() <= 1200);
        }
    }
}

#[test]
fn o_selo_do_aviso_aparece_na_chamada_e_pulsa_uma_troca_por_segundo_na_l4() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    olhando(&mut motor, false, 0);
    ocioso(&mut motor, false, 0);
    mandar(
        &mut motor,
        hook_de("s1", "api", "UserPromptSubmit", 1_000),
        1_000,
    );
    hook_em(&mut motor, "s1", "api", "PermissionRequest", 2_000);
    janela.mostrou();
    quadros_entre(&mut motor, &mut janela, 2_000, 3_000);
    let amarelo = [0x3F, 0xD2, 0xFF, 0xFF];
    let vermelho = [0x4B, 0x39, 0xE5, 0xFF];
    assert!(cores_na_cena(&janela).contains(&amarelo), "o «!» na L1");
    // Na L4 (5 min), a pose parada e o selo trocando de cor a cada segundo:
    // um commit por segundo.
    let l4 = 2_000 + escalada::L4_APOS_MS;
    quadros_entre(&mut motor, &mut janela, 3_000, l4 + 500);
    assert!(
        motor
            .painel(None, l4 + 500)
            .fotografia
            .escalada
            .unwrap()
            .pulso
    );
    let quadros = quadros_entre(&mut motor, &mut janela, l4 + 500, l4 + 10_500);
    assert_eq!(quadros.len(), 10, "{quadros:?}");
    assert!(quadros.windows(2).all(|j| j[1] - j[0] == PULSO_MS));
    let mut cores = Vec::new();
    for t in [l4 + 10_600, l4 + 11_600] {
        quadros_entre(&mut motor, &mut janela, t - 100, t);
        let na_cena = cores_na_cena(&janela);
        cores.push((na_cena.contains(&amarelo), na_cena.contains(&vermelho)));
    }
    assert_eq!(
        cores,
        vec![(true, false), (false, true)],
        "alterna: amarelo nos segundos pares desde o começo do pulso, vermelho nos ímpares"
    );
    // Respondida, o selo sai.
    let rodou = Evento {
        tool: Some("Bash".into()),
        dur: Some(10),
        ..hook_de("s1", "api", "PostToolUse", l4 + 20_000)
    };
    mandar(&mut motor, rodou, l4 + 20_000);
    quadros_entre(&mut motor, &mut janela, l4 + 20_000, l4 + 21_000);
    let na_cena = cores_na_cena(&janela);
    assert!(!na_cena.contains(&amarelo) && !na_cena.contains(&vermelho));
}

// --- o voo da escalada (decisão 0084) ---------------------------------------

/// Um Motor ligado com uma permissão pedida em 2 s e o Renan longe de um
/// terminal do Claude (a escalada sobe), desenhado até `ate`.
fn esperando_ate(ate: u64) -> (Motor, Falsa) {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    olhando(&mut motor, false, 0);
    ocioso(&mut motor, false, 0);
    mandar(
        &mut motor,
        hook_de("s1", "api", "UserPromptSubmit", 1_000),
        1_000,
    );
    hook_em(&mut motor, "s1", "api", "PermissionRequest", 2_000);
    janela.mostrou();
    quadros_entre(&mut motor, &mut janela, 0, ate);
    (motor, janela)
}

fn celula(motor: &Motor) -> (i32, i32) {
    let palco = motor.palco.expect("o palco");
    (palco.x, palco.y)
}

#[test]
fn o_voo_da_escalada_vai_ao_alto_centro_e_volta_sem_mexer_na_posicao_salva() {
    let l3 = 2_000 + escalada::L3_APOS_MS;
    let (mut motor, mut janela) = esperando_ate(l3 - 1);
    let casa = celula(&motor);
    let toque_em_casa = janela.toque.expect("o toque");
    assert!(motor.voo().is_none());
    let quadros = quadros_entre(&mut motor, &mut janela, l3 - 1, l3 + voo::SUBIDA_MS + 100);
    let v = *motor.voo().expect("o voo da L3");
    assert_eq!((v.motivo, v.casa), ("escalada", casa));
    // Lá em cima, no meio: o corpo no centro do monitor, perto do alto.
    let la = celula(&motor);
    assert_eq!(la, v.alvo);
    assert_eq!(
        ((la.0 - casa.0) % 5, (la.1 - casa.1) % 5),
        (0, 0),
        "múltiplos de D"
    );
    let toque = janela.toque.expect("o toque");
    assert!(
        (toque.x + toque.w / 2 - 960).abs() <= 5,
        "no centro: {toque:?}"
    );
    assert!(toque.y < 200, "no alto: {toque:?}");
    assert_ne!(toque, toque_em_casa, "a área de toque segue o pet");
    // O "!!" aceso em blocos de D, em cima da cabeça, dentro do monitor.
    let amarelo = [0x3F, 0xD2, 0xFF, 0xFF];
    let exclamacoes: Vec<Ret> = janela
        .cena
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|e| match e {
            Elemento::Bloco { ret, cor } if *cor == amarelo => Some(*ret),
            _ => None,
        })
        .collect();
    assert!(!exclamacoes.is_empty(), "o «!!» na cena");
    assert!(
        exclamacoes
            .iter()
            .all(|r| r.w % 5 == 0 && r.h == 5 && r.y >= 0 && r.baixo() <= toque.y)
    );
    // Os quadros do voo: nunca dois a menos de 34 ms.
    let voando: Vec<u64> = quadros.iter().copied().filter(|&t| t >= l3).collect();
    assert!(voando.len() > 20, "{voando:?}");
    assert!(
        voando
            .windows(2)
            .all(|j| j[1] - j[0] >= animador::DURACAO_MIN_MS),
        "{voando:?}"
    );
    // Paira, volta e pousa na casa; nada gravado.
    let fim = v.fim_ms();
    let resto = quadros_entre(
        &mut motor,
        &mut janela,
        l3 + voo::SUBIDA_MS + 100,
        fim + 2_000,
    );
    assert!(
        resto
            .windows(2)
            .all(|j| j[1] - j[0] >= animador::DURACAO_MIN_MS)
    );
    assert!(motor.voo().is_none());
    assert_eq!(celula(&motor), casa);
    assert_eq!(janela.toque, Some(toque_em_casa));
    assert_eq!(
        motor.posicoes_para_gravar(),
        None,
        "a posição salva intacta"
    );
    assert!(
        resto.iter().filter(|&&t| t <= fim).count() < 140,
        "uma rajada curta: {} quadros",
        resto.len()
    );
}

#[test]
fn a_resposta_no_meio_do_voo_manda_o_pet_de_volta_e_o_arraste_pega_ele_no_ar() {
    let l3 = 2_000 + escalada::L3_APOS_MS;
    let (mut motor, mut janela) = esperando_ate(l3 + 500);
    let casa = motor.voo().expect("voando").casa;
    // A permissão foi respondida: a escalada acaba e o pet desce já.
    let rodou = Evento {
        tool: Some("Bash".into()),
        dur: Some(10),
        ..hook_de("s1", "api", "PostToolUse", l3 + 600)
    };
    mandar(&mut motor, rodou, l3 + 600);
    let voltando = *motor.voo().expect("descendo");
    assert_eq!(voltando.fase(l3 + 600), Some(voo::Fase::Descendo));
    quadros_entre(
        &mut motor,
        &mut janela,
        l3 + 600,
        l3 + 600 + voo::DESCIDA_MS + 100,
    );
    assert!(motor.voo().is_none());
    assert_eq!(celula(&motor), casa);
    // Outro voo (a volta do Renan, depois de 60 s longe), pego no ar.
    let (mut motor, mut janela) = esperando_ate(l3 + 500);
    let antes = celula(&motor);
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), l3 + 520);
    janela.mostrou();
    ponteiro(&mut motor, &mut janela, moveu(x + 40, y + 40), l3 + 560);
    assert!(motor.voo().is_none(), "o arraste acaba o voo");
    assert!(motor.arrastando());
    assert_ne!(
        celula(&motor),
        casa_do_canto(&motor),
        "segue de onde estava"
    );
    let _ = antes;
}

/// A célula do canto padrão do monitor (a casa sem posição salva).
fn casa_do_canto(motor: &Motor) -> (i32, i32) {
    let pet = motor.pet.as_ref().unwrap();
    let palco = pet.palco(&edp(), motor.tamanho);
    (palco.x, palco.y)
}

#[test]
fn nao_voa_arrastando_e_a_soneca_manda_de_volta() {
    let l3 = 2_000 + escalada::L3_APOS_MS;
    // Segurando o pet quando a L3 chega: a intenção fica, o voo não sai.
    let (mut motor, mut janela) = esperando_ate(l3 - 300);
    let (x, y) = meio_do_corpo(&janela);
    ponteiro(&mut motor, &mut janela, apertou(x, y), l3 - 200);
    janela.mostrou();
    ponteiro(&mut motor, &mut janela, moveu(x - 30, y), l3 - 100);
    assert!(motor.arrastando());
    motor.tique(em(l3));
    assert!(
        motor
            .intencoes()
            .any(|i| matches!(i.tipo, intencoes::Tipo::Voo { .. })),
        "a escalada pediu"
    );
    assert!(motor.voo().is_none(), "arrastando, não voa");
    // No ar, o botão direito (a soneca) manda de volta.
    let (mut motor, mut janela) = esperando_ate(l3 + 300);
    assert!(motor.voo().is_some());
    janela.mostrou();
    motor.clicar(&mut janela, Botao::Direito, l3 + 400);
    assert!(motor.soneca(l3 + 400).is_some());
    assert_eq!(
        motor.voo().unwrap().fase(l3 + 400),
        Some(voo::Fase::Descendo)
    );
}

// --- o confete da festa (decisão 0085) ---------------------------------------

/// Um turno com `bash_ms` de Bash e uma edição, com o Stop em `ms + 1 s`: com
/// 4 min de Bash dá 4,8 pontos (T2); com 12 min, 12,8 (T3).
fn turno_de(motor: &mut Motor, sid: &str, proj: &str, ms: u64, bash_ms: u64) {
    mandar(motor, hook_de(sid, proj, "UserPromptSubmit", ms), ms);
    let bash = Evento {
        tool: Some("Bash".into()),
        dur: Some(bash_ms),
        ..hook_de(sid, proj, "PostToolUse", ms + 500)
    };
    mandar(motor, bash, ms + 500);
    let edit = Evento {
        tool: Some("Edit".into()),
        arq: Some(format!("{:0>12}", sid.len())),
        dur: Some(30),
        ..hook_de(sid, proj, "PostToolUse", ms + 600)
    };
    mandar(motor, edit, ms + 600);
    mandar(motor, hook_de(sid, proj, "Stop", ms + 1_000), ms + 1_000);
}

/// Os blocos de confete da cena (as cores do confete).
fn pedacos_na_cena(janela: &Falsa) -> Vec<Ret> {
    janela
        .cena
        .as_ref()
        .map(|c| {
            c.iter()
                .filter_map(|e| match e {
                    Elemento::Bloco { ret, cor } if crate::confete::CORES.contains(cor) => {
                        Some(*ret)
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_festa_t2_solta_a_fonte_e_a_t3_a_chuva_a_ate_30_quadros_por_segundo() {
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    janela.mostrou();
    quadros_entre(&mut motor, &mut janela, 0, 1_000);
    turno_de(&mut motor, "s1", "api", 1_000, 240_000);
    let festa = 2_000 + crate::cerebro::ACOMODACAO_MS;
    quadros_como_o_laco(&mut motor, &mut janela, 1_000, festa + 100);
    let nivel = motor.intencoes().find_map(|i| match i.tipo {
        intencoes::Tipo::Festa { nivel, confete, .. } => Some((nivel, confete)),
        _ => None,
    });
    assert_eq!(nivel, Some((Nivel::T2, tela::CONFETES_T2)));
    let palco = motor.palco.unwrap();
    let pedacos = pedacos_na_cena(&janela);
    assert!(!pedacos.is_empty() && pedacos.len() <= 12, "{pedacos:?}");
    for r in &pedacos {
        assert_eq!(
            ((r.x - palco.x) % 5, (r.y - palco.y) % 5),
            (0, 0),
            "na grade de D"
        );
        assert_eq!((r.w, r.h), (2 * 5, 2 * 5));
    }
    // Anda de 100 em 100 ms: os quadros nunca a menos de 34 ms, e o confete
    // acaba em menos de 4,5 s.
    let mut horas = Vec::new();
    let mut acabou = None;
    let mut t = festa + 100;
    while t < festa + 6_000 {
        horas.extend(quadros_como_o_laco(&mut motor, &mut janela, t, t + 100));
        t += 100;
        if acabou.is_none() && motor.confete_na_tela(t) == 0 {
            acabou = Some(t);
        }
    }
    assert!(
        horas
            .windows(2)
            .all(|j| j[1] - j[0] >= animador::DURACAO_MIN_MS)
    );
    let acabou = acabou.expect("o confete acaba");
    assert!(
        acabou <= festa + CONFETE_MAX_MS,
        "curto: {}",
        acabou - festa
    );
    assert!(pedacos_na_cena(&janela).is_empty());
    // O T3 de outra sessão: a chuva pela tela inteira.
    turno_de(&mut motor, "s2", "web", 20_000, 720_000);
    let festa = 21_000 + crate::cerebro::ACOMODACAO_MS;
    let mut mais = 0;
    let mut horas = Vec::new();
    let mut t = festa;
    while t < festa + 6_000 {
        horas.extend(quadros_como_o_laco(&mut motor, &mut janela, t, t + 100));
        mais = mais.max(pedacos_na_cena(&janela).len());
        t += 100;
    }
    assert!(mais >= 20 && mais <= tela::CONFETES_T3 as usize, "{mais}");
    assert!(
        horas
            .windows(2)
            .all(|j| j[1] - j[0] >= animador::DURACAO_MIN_MS)
    );
    assert!(
        horas
            .iter()
            .filter(|&&t| t <= festa + CONFETE_MAX_MS)
            .count()
            <= 135,
        "até 30 por segundo por 4,5 s"
    );
    assert_eq!(motor.confete_na_tela(festa + 6_000), 0);
}

#[test]
fn sem_confete_na_soneca_e_numa_janela_pequena_e_o_nivel_que_sobe_troca_a_fonte_pela_chuva() {
    // Na soneca, a festa é o aceno, sem confete.
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    janela.mostrou();
    motor.alternar_soneca(&mut janela, 0);
    turno_de(&mut motor, "s1", "api", 1_000, 240_000);
    quadros_entre(&mut motor, &mut janela, 0, 3_000);
    assert_eq!(motor.confete_na_tela(3_000), 0);
    // Numa janela pequena (o palco transitório é do M8), nada pela tela.
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.definir_skin(Some(skin_teste()));
    let mut janela = Falsa::pequena();
    motor.conectou(0);
    motor.aplicar_visibilidade(&mut janela, 0);
    janela.pronta = Some(edp());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 0);
    motor.acertar_relogio(em(0));
    turno_de(&mut motor, "s1", "api", 1_000, 240_000);
    quadros_entre(&mut motor, &mut janela, 0, 3_000);
    assert_eq!(motor.confete_na_tela(3_000), 0);
    assert!(pedacos_na_cena(&janela).is_empty());
    // A festa mesclada que sobe do T2 ao T3: a chuva no lugar da fonte.
    let (mut motor, mut janela) = ligado();
    motor.acertar_relogio(em(0));
    janela.mostrou();
    turno_de(&mut motor, "s1", "api", 1_000, 240_000);
    turno_de(&mut motor, "s2", "web", 2_500, 720_000);
    quadros_entre(&mut motor, &mut janela, 0, 4_400);
    let mescladas: Vec<(Nivel, u32)> = motor
        .intencoes()
        .filter_map(|i| match i.tipo {
            intencoes::Tipo::FestaMesclada { nivel, confete, .. } => Some((nivel, confete)),
            _ => None,
        })
        .collect();
    assert_eq!(mescladas, vec![(Nivel::T3, tela::CONFETES_T3)]);
    let mut mais = 0;
    for t in (4_400..9_000).step_by(100) {
        quadros_entre(&mut motor, &mut janela, t, t + 100);
        mais = mais.max(pedacos_na_cena(&janela).len());
    }
    assert!(mais > 12, "a chuva do T3: {mais}");
}

// --- o desenho no /v1/estado (decisão 0086) ---------------------------------

#[test]
fn o_painel_mostra_o_que_a_janela_desenha() {
    let l3 = 2_000 + escalada::L3_APOS_MS;
    let (mut motor, janela) = esperando_ate(5_000);
    let d = motor.painel(Some(&janela), 5_000).desenho;
    assert_eq!(
        (d.base.as_deref(), d.ritmo),
        (Some("waiting"), Some("repouso"))
    );
    assert_eq!(
        d.selos,
        Some(PainelFileira {
            aviso: Some("normal"),
            pulso: false,
            mais: 0,
            corrente: false,
            bandeiras: 0
        })
    );
    assert_eq!((d.voo, d.confete), (None, 0));
    // No voo: a fase e o motivo; a fileira dá lugar ao "!!".
    let (mut motor2, mut janela2) = esperando_ate(l3 + 100);
    let d = motor2.painel(Some(&janela2), l3 + 100).desenho;
    assert_eq!(
        d.voo,
        Some(PainelVoo {
            fase: "subindo",
            motivo: "escalada"
        })
    );
    assert_eq!(d.selos, None);
    // Na L4: a pose parada e o pulso.
    let l4 = 2_000 + escalada::L4_APOS_MS;
    quadros_entre(&mut motor2, &mut janela2, l3 + 100, l4 + 1_500);
    let d = motor2.painel(Some(&janela2), l4 + 1_500).desenho;
    assert_eq!(d.ritmo, Some("parado"));
    assert_eq!(
        d.selos.map(|s| (s.aviso, s.pulso)),
        Some((Some("aceso"), true))
    );
    // Numa festa T2, o confete; e o JSON só com metadados.
    turno_de(&mut motor, "s2", "agenda-secreta", 6_000, 240_000);
    let mut janela = janela;
    quadros_como_o_laco(&mut motor, &mut janela, 5_000, 7_900);
    let d = motor.painel(Some(&janela), 7_900).desenho;
    assert!(d.confete > 0 && d.confete <= 12, "{d:?}");
    let json = serde_json::to_string(&d).unwrap();
    assert!(!json.contains("agenda"), "{json}");
    // Sem janela (o compositor caiu), nada desenhado.
    assert_eq!(motor.painel(None, 7_900).desenho, PainelDesenho::default());
}

// --- a prova do orçamento e da nitidez (decisão 0088) ------------------------
//
// O orçamento de commits (decisão 0005): parado e esperando, em média até 2
// por segundo; o sono profundo, nenhum; as rajadas (o voo, o confete) até 30
// por segundo e curtas. Com o compositor mostrando cada quadro na hora (o pior
// caso: na tela de verdade um quadro em voo espera o frame callback) e as
// reações tocando como no laço do daemon, com o Zeca original de produção
// (`zeca-livre-escuro`, D = 4 no `pequeno` do eDP-1) e com a skin de teste.

fn skin_de_producao() -> Rc<Skin> {
    let pasta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skins/zeca-livre-escuro");
    Rc::new(Skin::carregar(&pasta).expect("skins/zeca-livre-escuro"))
}

/// Um Motor com `skin` no tamanho `tamanho`, pronto no eDP-1 em `t = 0`, o
/// Renan no teclado (ou longe, `longe`) e fora de um terminal do Claude.
fn ligado_com(skin: Rc<Skin>, tamanho: Tamanho, longe: bool) -> (Motor, Falsa) {
    let mut motor = Motor::novo(ConfigCerebro::default());
    motor.definir_skin(Some(skin));
    motor.definir_tamanho(tamanho, None, 0);
    let mut janela = Falsa::default();
    motor.conectou(0);
    motor.aplicar_visibilidade(&mut janela, 0);
    janela.pronta = Some(edp());
    motor.evento_overlay(&mut janela, EventoOverlay::Pronta, 0);
    motor.acertar_relogio(em(0));
    olhando(&mut motor, false, 0);
    ocioso(&mut motor, longe, 0);
    janela.mostrou();
    (motor, janela)
}

/// Os commits (quadros novos e só de estado) de `de` a `ate`, com as horas
/// dos quadros novos.
fn commits_entre(motor: &mut Motor, janela: &mut Falsa, de: u64, ate: u64) -> (u64, Vec<u64>) {
    let antes = motor.commits.total;
    let horas = quadros_como_o_laco(motor, janela, de, ate);
    (motor.commits.total - antes, horas)
}

fn por_segundo(commits: u64, de: u64, ate: u64) -> f64 {
    commits as f64 * 1000.0 / (ate - de) as f64
}

fn menor_intervalo(horas: &[u64]) -> u64 {
    horas
        .windows(2)
        .map(|j| j[1] - j[0])
        .min()
        .unwrap_or(u64::MAX)
}

fn skins_do_orcamento() -> Vec<(Rc<Skin>, Tamanho)> {
    vec![
        (skin_de_producao(), Tamanho::Pequeno),
        (skin_teste(), Tamanho::Normal),
    ]
}

#[test]
fn orcamento_trabalhando_por_20_min() {
    for (skin, tamanho) in skins_do_orcamento() {
        let id = skin.id.clone();
        let (mut motor, mut janela) = ligado_com(skin, tamanho, false);
        mandar(
            &mut motor,
            hook_de("s1", "api", "UserPromptSubmit", 1_000),
            1_000,
        );
        let mut commits = 0;
        let mut horas = Vec::new();
        // Um Bash a cada 30 s: a sessão fica trabalhando os 20 min.
        let mut t = 2_000;
        while t < 2_000 + 20 * 60_000 {
            let bash = Evento {
                tool: Some("Bash".into()),
                dur: Some(10),
                ..hook_de("s1", "api", "PostToolUse", t)
            };
            mandar(&mut motor, bash, t);
            let (c, h) = commits_entre(&mut motor, &mut janela, t, t + 30_000);
            commits += c;
            horas.extend(h);
            t += 30_000;
        }
        let media = por_segundo(commits, 2_000, t);
        eprintln!(
            "orçamento {id}: trabalhando 20 min a {media:.2} commits/s, o menor intervalo {} ms",
            menor_intervalo(&horas)
        );
        assert_eq!(
            motor.painel(None, t).fotografia.base,
            "working",
            "{id}: trabalhando o tempo todo"
        );
        assert!(media <= 1.0, "{id}: {media:.2} commits/s trabalhando");
        assert!(
            menor_intervalo(&horas) >= animador::DURACAO_MIN_QUIETO_MS,
            "{id}: quase parado, até 4 fps: {} ms",
            menor_intervalo(&horas)
        );
    }
}

#[test]
fn orcamento_na_espera_por_10_min_no_teto() {
    for (skin, tamanho) in skins_do_orcamento() {
        let id = skin.id.clone();
        let (mut motor, mut janela) = ligado_com(skin, tamanho, false);
        mandar(
            &mut motor,
            hook_de("s1", "api", "UserPromptSubmit", 1_000),
            1_000,
        );
        hook_em(&mut motor, "s1", "api", "PermissionRequest", 2_000);
        let l4 = 2_000 + escalada::L4_APOS_MS;
        // Até a L4 (a chamada, as rajadas da L2, os 3 voos da L3): rajadas
        // curtas, nenhum quadro a menos de 34 ms.
        let (_, antes) = commits_entre(&mut motor, &mut janela, 2_000, l4 + 1_000);
        assert!(menor_intervalo(&antes) >= animador::DURACAO_MIN_MS, "{id}");
        assert_eq!(motor.nivel_da_escalada(), 4);
        let (commits, horas) = commits_entre(&mut motor, &mut janela, l4 + 1_000, l4 + 601_000);
        let media = por_segundo(commits, l4 + 1_000, l4 + 601_000);
        eprintln!("orçamento {id}: a espera na L4 por 10 min a {media:.2} commits/s");
        assert!(
            (0.8..=2.0).contains(&media),
            "{id}: {media:.2} commits/s na L4 (o pulso a 1 por segundo e a rajada a cada minuto)"
        );
        assert!(menor_intervalo(&horas) >= animador::DURACAO_MIN_MS, "{id}");
        // Depois dos 30 min da L4, a espera solta a base (decisão 0090): o
        // selo "!" parado e o repouso de sempre, até o sono.
        let fim = 2_000 + escalada::L4_APOS_MS + escalada::L4_DURA_MS;
        commits_entre(&mut motor, &mut janela, l4 + 601_000, fim + 1_000);
        assert_eq!(motor.painel(None, fim + 1_000).fotografia.base, "idle");
        let (depois, _) = commits_entre(&mut motor, &mut janela, fim + 1_000, fim + 601_000);
        assert!(
            por_segundo(depois, fim + 1_000, fim + 601_000) <= 2.0,
            "{id}: {depois} commits depois do teto"
        );
        // E dorme, com o selo: o sono profundo sem commit nenhum.
        commits_entre(&mut motor, &mut janela, fim + 601_000, fim + 40 * 60_000);
        let (profundo, _) = commits_entre(
            &mut motor,
            &mut janela,
            fim + 40 * 60_000,
            fim + 50 * 60_000,
        );
        assert_eq!(profundo, 0, "{id}: o sono profundo com o aviso esperando");
        let d = motor.painel(Some(&janela), fim + 50 * 60_000).desenho;
        assert_eq!(
            d.selos.and_then(|s| s.aviso),
            Some("normal"),
            "{id}: o selo do aviso fica"
        );
    }
}

#[test]
fn orcamento_parado_por_30_min_e_o_sono_profundo_sem_commit() {
    for (skin, tamanho) in skins_do_orcamento() {
        let id = skin.id.clone();
        let (mut motor, mut janela) = ligado_com(skin, tamanho, false);
        // Até o sono profundo (aos 30 min, o quadro da pose dele inclusive).
        let (commits, horas) = commits_entre(&mut motor, &mut janela, 0, 30 * 60_000 + 1_000);
        let media = por_segundo(commits, 0, 30 * 60_000 + 1_000);
        eprintln!("orçamento {id}: parado e dormindo 30 min a {media:.2} commits/s");
        assert!(media <= 2.0, "{id}: {media:.2} commits/s parado e dormindo");
        assert!(media > 0.3, "{id}: anda ({media:.2})");
        assert!(menor_intervalo(&horas) >= animador::DURACAO_MIN_MS, "{id}");
        let (profundo, _) =
            commits_entre(&mut motor, &mut janela, 30 * 60_000 + 1_000, 90 * 60_000);
        assert_eq!(profundo, 0, "{id}: o sono profundo sem commit");
        assert_eq!(
            motor.painel(None, 90 * 60_000).fotografia.sono,
            Sono::Profundo
        );
    }
}

#[test]
fn orcamento_a_rajada_do_t3_e_curta_e_vai_ate_30_quadros_por_segundo() {
    for (skin, tamanho) in skins_do_orcamento() {
        let id = skin.id.clone();
        let (mut motor, mut janela) = ligado_com(skin, tamanho, false);
        commits_entre(&mut motor, &mut janela, 0, 1_000);
        turno_de(&mut motor, "s1", "api", 1_000, 720_000);
        let festa = 2_000 + crate::cerebro::ACOMODACAO_MS;
        let (_, ate_a_festa) = commits_entre(&mut motor, &mut janela, 1_000, festa);
        let (commits, horas) = commits_entre(&mut motor, &mut janela, festa, festa + 5_000);
        let mut todas = ate_a_festa;
        todas.extend(&horas);
        assert!(
            menor_intervalo(&todas) >= animador::DURACAO_MIN_MS,
            "{id}: até 30 por segundo ({} ms)",
            menor_intervalo(&todas)
        );
        eprintln!(
            "orçamento {id}: o T3 com {commits} commits em 5 s, o menor intervalo {} ms",
            menor_intervalo(&todas)
        );
        assert!(commits <= 5 * 30, "{id}: {commits} commits em 5 s");
        assert!(commits >= 60, "{id}: a chuva anda ({commits})");
        assert_eq!(motor.confete_na_tela(festa + 5_000), 0, "{id}: curta");
        // Depois, o pronto no repouso: até 2 por segundo.
        let (depois, _) = commits_entre(&mut motor, &mut janela, festa + 5_000, festa + 65_000);
        assert!(
            por_segundo(depois, festa + 5_000, festa + 65_000) <= 2.0,
            "{id}: {depois} commits no minuto depois"
        );
    }
}

/// Toda peça da cena em blocos inteiros: o sprite com o D do palco e a
/// célula na grade de D da casa (`casa`), os blocos e os glifos em
/// múltiplos da metade do D (os selos e o balão) ou do D (o "!!" e o
/// confete), e tudo dentro do monitor.
fn conferir_nitidez(janela: &Falsa, palco: Palco, casa: (i32, i32), onde: &str) {
    let dt = balao::dt(palco.d);
    let tela = Ret::novo(0, 0, palco.tela.0, palco.tela.1);
    for e in janela.cena.as_ref().expect("uma cena") {
        match *e {
            Elemento::Sprite { x, y, d, .. } => {
                assert_eq!(d, palco.d, "{onde}: o sprite no D do palco");
                assert_eq!(
                    ((x - casa.0) % d, (y - casa.1) % d),
                    (0, 0),
                    "{onde}: a célula na grade de D da casa"
                );
            }
            Elemento::Bloco { ret, cor } => {
                assert_eq!(cor[3], 255, "{onde}: opaco");
                assert!(
                    (ret.w % dt == 0 && ret.h % dt == 0)
                        || (ret.w % palco.d == 0 && ret.h % palco.d == 0),
                    "{onde}: bloco fora da grade: {ret:?}"
                );
                assert_eq!(
                    ret.intersecao(&tela),
                    Some(ret),
                    "{onde}: fora do monitor: {ret:?}"
                );
            }
            Elemento::Glifo { d, .. } => assert_eq!(d, dt, "{onde}: o glifo na metade do D"),
        }
    }
}

#[test]
fn nitidez_dos_desenhos_novos_com_o_zeca_de_producao() {
    let (mut motor, mut janela) = ligado_com(skin_de_producao(), Tamanho::Pequeno, false);
    let palco = motor.palco.expect("o palco");
    assert_eq!(palco.d, 4, "D = 4 no pequeno do eDP-1 (decisão 0042)");
    let casa = (palco.x, palco.y);
    // Selos: duas sessões ocupadas, uma com o pronto (a bandeirinha) e uma
    // corrente; a festa T3 com a chuva.
    turno_de(&mut motor, "s1", "api", 1_000, 720_000);
    mandar(
        &mut motor,
        hook_de("s2", "web", "UserPromptSubmit", 1_500),
        1_500,
    );
    let festa = 2_000 + crate::cerebro::ACOMODACAO_MS;
    for t in (festa..festa + 4_000).step_by(250) {
        commits_entre(&mut motor, &mut janela, t, t + 250);
        conferir_nitidez(&janela, motor.palco.unwrap(), casa, "festa");
    }
    // A espera com o «!», o voo da L3 com o "!!" e o pulso da L4.
    let (mut motor, mut janela) = ligado_com(skin_de_producao(), Tamanho::Pequeno, false);
    turno_de(&mut motor, "s3", "lab", 500, 900);
    mandar(
        &mut motor,
        hook_de("s1", "api", "UserPromptSubmit", 1_000),
        1_000,
    );
    hook_em(&mut motor, "s1", "api", "PermissionRequest", 2_000);
    let l3 = 2_000 + escalada::L3_APOS_MS;
    let mut t = 2_000;
    while t < 2_000 + escalada::L4_APOS_MS + 3_000 {
        commits_entre(&mut motor, &mut janela, t, t + 100);
        let onde = if t >= l3 && motor.voo().is_some() {
            "voo"
        } else {
            "espera"
        };
        conferir_nitidez(&janela, motor.palco.unwrap(), casa, onde);
        t += if (l3..l3 + 5_000).contains(&t) {
            100
        } else {
            5_000
        };
    }
    assert_eq!(
        motor.painel(None, t).fotografia.escalada.map(|e| e.nivel),
        Some(4)
    );
}

#[test]
fn um_voo_que_acabou_sem_quadros_nao_segura_a_volta_do_renan() {
    // A sessão bloqueada: o primeiro quadro do voo fica em voo (o
    // compositor não pede outro), e o voo da L3 acaba só pelo relógio.
    let l3 = 2_000 + escalada::L3_APOS_MS;
    let (mut motor, mut janela) = esperando_ate(l3 - 1);
    janela.em_voo = true;
    motor.tique(em(l3));
    let primeiro = *motor.voo().expect("a L3 pediu o voo");
    // O `/v1/estado.desenho` segue o relógio: no meio, o voo; depois do fim,
    // a fileira com o aviso, como o próximo quadro vai desenhar.
    let pairando = primeiro.inicio_ms + voo::SUBIDA_MS + 100;
    let d = motor.painel(Some(&janela), pairando).desenho;
    assert_eq!((d.voo.map(|v| v.fase), d.selos), (Some("pairando"), None));
    let d = motor.painel(Some(&janela), primeiro.fim_ms() + 1).desenho;
    assert_eq!(d.voo, None);
    assert_eq!(d.selos.and_then(|s| s.aviso), Some("normal"));
    // Bem depois do fim dele, o Renan volta (60 s longe, fora do terminal do
    // Claude): o voo da volta sai, no lugar do que ficou parado.
    ocioso(&mut motor, true, l3 + 10_000);
    let volta = l3 + 80_000;
    ocioso(&mut motor, false, volta);
    let v = motor.voo().expect("o voo da volta");
    assert_eq!(v.motivo, "voltou");
    assert!(v.inicio_ms > primeiro.fim_ms());
}
