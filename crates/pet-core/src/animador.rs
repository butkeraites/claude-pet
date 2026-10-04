//! Animador: toca tags com as durações por quadro do Aseprite, mantém o pet
//! parado de um jeito barato (M1) e toca uma reação uma vez, voltando à
//! pose (M3).
//!
//! Cada troca de quadro vira um commit Wayland, e cada commit repinta o
//! monitor inteiro no Hyprland 0.56 (decisão 0005). Por isso o repouso não é
//! a tag `idle` em laço: é uma **pose fixa** (o primeiro quadro da primeira
//! tag do estado `idle`) e uma **rajada** de vez em quando, que toca uma das
//! tags do estado uma vez, alternando entre elas.
//!
//! O orçamento é garantido aqui, qualquer que seja a skin:
//! - nenhum quadro dura menos que [`DURACAO_MIN_MS`] (rajadas de até 30 fps);
//! - a pausa antes de cada rajada é de pelo menos [`PAUSA_MS`] e cresce o
//!   quanto for preciso para cada trecho pausa + rajada ficar em até
//!   [`COMMITS_POR_S_PARADO`] commits/s em média.
//!
//! Uma **reação** (o aceno do T0, o pulinho do T1, um `tocar`) é uma rajada
//! a mais: toca a tag uma vez, com as mesmas durações mínimas, e o repouso
//! recomeça no fim dela, com a pausa inteira antes da próxima rajada.
//!
//! Um estado **segurado** (o `dangle` enquanto o pet é arrastado, M4) toca em
//! laço até ser largado; uma reação no meio toca por cima dele e, no fim,
//! volta ao laço. O arraste é a exceção ao orçamento de commits parado: ele
//! acaba quando o botão é solto (decisão 0048).
//!
//! Tudo aqui recebe o relógio de fora (milissegundos), para os testes não
//! dependerem de tempo real.

use crate::estados;
use crate::skin::{Direcao, Skin, Tag};

/// Pausa mínima entre rajadas no repouso.
pub const PAUSA_MS: u64 = 4000;
/// Duração mínima de um quadro: nenhuma animação passa de 30 fps.
pub const DURACAO_MIN_MS: u64 = 34;
/// Teto da média de commits com o pet parado (decisão 0005).
pub const COMMITS_POR_S_PARADO: u64 = 2;

/// Quadros de uma tag na ordem em que tocam num ciclo (sem repetir as pontas
/// no ping-pong, como o Aseprite).
pub fn sequencia(tag: &Tag) -> Vec<usize> {
    let ida: Vec<usize> = (tag.de..=tag.ate).collect();
    let miolo = if ida.len() > 2 {
        &ida[1..ida.len() - 1]
    } else {
        &[][..]
    };
    match tag.direcao {
        Direcao::Frente => ida.clone(),
        Direcao::Tras => ida.iter().rev().copied().collect(),
        Direcao::PingPong => ida.iter().chain(miolo.iter().rev()).copied().collect(),
        Direcao::PingPongReverso => ida.iter().rev().chain(miolo.iter()).copied().collect(),
    }
}

/// Uma tag tocando a partir de um instante, uma vez ou em laço.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animacao {
    /// (quadro da folha, duração em ms).
    passos: Vec<(usize, u64)>,
    inicio_ms: u64,
    laco: bool,
    total_ms: u64,
}

impl Animacao {
    pub fn nova(skin: &Skin, tag: usize, inicio_ms: u64, laco: bool) -> Animacao {
        // Quadros com a mesma imagem viram o mesmo quadro: um passo que não
        // muda nada na tela não conta como troca nem gera commit.
        let passos: Vec<(usize, u64)> = sequencia(&skin.tags[tag])
            .into_iter()
            .map(|q| {
                let duracao = (skin.quadros[q].duracao_ms as u64).max(DURACAO_MIN_MS);
                (skin.canonico[q], duracao)
            })
            .collect();
        let total_ms = passos.iter().map(|(_, d)| d).sum();
        Animacao {
            passos,
            inicio_ms,
            laco,
            total_ms,
        }
    }

    pub fn duracao_ms(&self) -> u64 {
        self.total_ms
    }

    /// Quadro em `agora_ms` e o instante da próxima troca (`None`: não troca
    /// mais — acabou, ou a tag tem um quadro só).
    pub fn em(&self, agora_ms: u64) -> (usize, Option<u64>) {
        let decorrido = agora_ms.saturating_sub(self.inicio_ms);
        let ultimo = self.passos.last().map(|p| p.0).unwrap_or(0);
        if self.passos.iter().all(|p| p.0 == ultimo) {
            return (ultimo, None);
        }
        if !self.laco && decorrido >= self.total_ms {
            return (ultimo, None);
        }
        let ciclo = decorrido / self.total_ms;
        let mut t = decorrido % self.total_ms;
        let base = self.inicio_ms + ciclo * self.total_ms;
        let mut fim = base;
        for &(quadro, duracao) in &self.passos {
            fim += duracao;
            if t < duracao {
                return (quadro, Some(fim));
            }
            t -= duracao;
        }
        (ultimo, None)
    }
}

/// Um trecho do repouso: a pose parada por `pausa_ms` e depois a rajada.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Trecho {
    pausa_ms: u64,
    rajada: Animacao,
}

/// O pet parado: pose fixa e rajadas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repouso {
    pose: usize,
    /// Trechos em ordem (cada rajada começa no instante 0; o repouso desloca).
    trechos: Vec<Trecho>,
    inicio_ms: u64,
    periodo_ms: u64,
}

/// Trocas de quadro de uma rajada que sai da pose e volta para ela.
fn trocas(pose: usize, rajada: &Animacao) -> u64 {
    let mut anterior = pose;
    let mut trocas = 0;
    for quadro in rajada.passos.iter().map(|p| p.0).chain([pose]) {
        if quadro != anterior {
            trocas += 1;
            anterior = quadro;
        }
    }
    trocas
}

/// Pausa antes de uma rajada: pelo menos [`PAUSA_MS`] e o bastante para o
/// trecho inteiro ficar em até [`COMMITS_POR_S_PARADO`] commits/s.
fn pausa_para(pose: usize, rajada: &Animacao) -> u64 {
    let minimo_do_trecho = (trocas(pose, rajada) * 1000).div_ceil(COMMITS_POR_S_PARADO);
    PAUSA_MS.max(minimo_do_trecho.saturating_sub(rajada.duracao_ms()))
}

impl Repouso {
    /// Repouso do estado `idle` da skin (a skin garante que ele existe).
    pub fn novo(skin: &Skin, inicio_ms: u64) -> Repouso {
        let tags = skin.tags_do_estado("idle");
        let pose = tags
            .first()
            .map(|&t| skin.canonico[sequencia(&skin.tags[t])[0]])
            .unwrap_or(0);
        let trechos: Vec<Trecho> = tags
            .iter()
            .map(|&t| {
                let rajada = Animacao::nova(skin, t, 0, false);
                Trecho {
                    pausa_ms: pausa_para(pose, &rajada),
                    rajada,
                }
            })
            .collect();
        let periodo_ms = trechos
            .iter()
            .map(|t| t.pausa_ms + t.rajada.duracao_ms())
            .sum::<u64>();
        Repouso {
            pose,
            trechos,
            inicio_ms,
            periodo_ms: periodo_ms.max(PAUSA_MS),
        }
    }

    pub fn pose(&self) -> usize {
        self.pose
    }

    /// Quadro em `agora_ms` e o instante da próxima troca.
    pub fn em(&self, agora_ms: u64) -> (usize, u64) {
        let decorrido = agora_ms.saturating_sub(self.inicio_ms);
        let base = self.inicio_ms + decorrido / self.periodo_ms * self.periodo_ms;
        let mut t = decorrido % self.periodo_ms;
        let mut inicio_segmento = base;
        for Trecho { pausa_ms, rajada } in &self.trechos {
            if t < *pausa_ms {
                return (self.pose, inicio_segmento + pausa_ms);
            }
            t -= pausa_ms;
            inicio_segmento += pausa_ms;
            if t < rajada.duracao_ms() {
                let (quadro, proxima) = rajada.em(t);
                let fim = inicio_segmento + rajada.duracao_ms();
                let proxima = proxima.map_or(fim, |p| inicio_segmento + p);
                return (quadro, proxima.min(fim));
            }
            t -= rajada.duracao_ms();
            inicio_segmento += rajada.duracao_ms();
        }
        (self.pose, base + self.periodo_ms)
    }
}

/// Tag que toca a reação `nome` nesta skin, pelos `estados` do `skin.json`:
/// a primeira tag do estado semântico `nome`; senão a do primeiro estado de
/// reserva que a skin tem, pelas reservas do catálogo
/// ([`estados::reserva`], as mesmas que o `cargo xtask cobertura` mostra),
/// nunca a pose parada (`idle`: tocar o repouso não é reação); senão uma tag
/// com esse nome. `None`: a skin não sabe tocar a reação.
///
/// O cérebro emite `nod` (T0), `done_small` (T1) e `bye`; o `tocar` aceita
/// qualquer estado. Na skin de teste o aceno cai no `wave` e o tchau não
/// anima; no Zeca os três são nativos (o aceno é a tag composta `nod`, que
/// levanta e senta; o pulinho e o tchau são o pio), e o `cargo xtask
/// cobertura --nativos mvp` exige isso de todo personagem (decisão 0030).
pub fn tag_da_reacao(skin: &Skin, nome: &str) -> Option<usize> {
    let primeira = |estado: &str| skin.tags_do_estado(estado).first().copied();
    primeira(nome)
        .or_else(|| {
            estados::reserva(nome, &|r| r != "idle" && primeira(r).is_some()).and_then(primeira)
        })
        .or_else(|| skin.tags.iter().position(|t| t.nome == nome))
}

/// Uma reação tocando uma vez.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Tocando {
    nome: String,
    animacao: Animacao,
    fim_ms: u64,
}

/// Um estado tocando em laço até ser largado.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Segurando {
    nome: String,
    animacao: Animacao,
}

/// O que o pet mostra: o repouso (ou um estado segurado) e, por cima, uma
/// reação de cada vez.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animador {
    repouso: Repouso,
    tocando: Option<Tocando>,
    segurando: Option<Segurando>,
}

impl Animador {
    pub fn novo(skin: &Skin, agora_ms: u64) -> Animador {
        Animador {
            repouso: Repouso::novo(skin, agora_ms),
            tocando: None,
            segurando: None,
        }
    }

    /// Segura o estado `nome` em laço a partir de `agora_ms` (o `dangle`
    /// enquanto o pet é arrastado), pelas mesmas reservas das reações, até
    /// [`Animador::largar`]. Larga a reação que estiver tocando. `false`: a
    /// skin não sabe tocar o estado (o pet fica como está).
    pub fn segurar(&mut self, skin: &Skin, nome: &str, agora_ms: u64) -> bool {
        let Some(tag) = tag_da_reacao(skin, nome) else {
            return false;
        };
        self.tocando = None;
        self.segurando = Some(Segurando {
            nome: nome.to_owned(),
            animacao: Animacao::nova(skin, tag, agora_ms, true),
        });
        true
    }

    /// Larga o estado segurado: o repouso recomeça agora, com a pausa
    /// inteira antes da próxima rajada (ou no fim da reação que estiver
    /// tocando).
    pub fn largar(&mut self, skin: &Skin, agora_ms: u64) {
        if self.segurando.take().is_none() {
            return;
        }
        let fim_da_reacao = self
            .tocando
            .as_ref()
            .filter(|t| agora_ms < t.fim_ms)
            .map(|t| t.fim_ms);
        self.repouso = Repouso::novo(skin, fim_da_reacao.unwrap_or(agora_ms));
    }

    /// Há um estado segurado.
    pub fn segurado(&self) -> bool {
        self.segurando.is_some()
    }

    /// A pose parada (primeiro quadro de `idle`).
    pub fn pose(&self) -> usize {
        self.repouso.pose()
    }

    /// Toca a reação `nome` uma vez a partir de `agora_ms`, no lugar da que
    /// estiver tocando, e depois volta à pose. O repouso recomeça no fim da
    /// reação, com a pausa inteira antes da próxima rajada: a reação nunca
    /// emenda numa rajada e o orçamento de commits continua valendo.
    /// `false`: a skin não sabe tocar essa reação (nada muda).
    pub fn tocar(&mut self, skin: &Skin, nome: &str, agora_ms: u64) -> bool {
        let Some(tag) = tag_da_reacao(skin, nome) else {
            return false;
        };
        let animacao = Animacao::nova(skin, tag, agora_ms, false);
        let fim_ms = agora_ms + animacao.duracao_ms();
        self.repouso = Repouso::novo(skin, fim_ms);
        self.tocando = Some(Tocando {
            nome: nome.to_owned(),
            animacao,
            fim_ms,
        });
        true
    }

    /// Quadro em `agora_ms` e o instante da próxima troca.
    pub fn em(&self, agora_ms: u64) -> (usize, u64) {
        if let Some(t) = &self.tocando
            && agora_ms < t.fim_ms
        {
            let (quadro, proxima) = t.animacao.em(agora_ms);
            return (quadro, proxima.map_or(t.fim_ms, |p| p.min(t.fim_ms)));
        }
        if let Some(s) = &self.segurando {
            let (quadro, proxima) = s.animacao.em(agora_ms);
            // Um laço de um quadro só não troca mais: o próximo prazo fica
            // para quando ele for largado.
            return (quadro, proxima.unwrap_or(u64::MAX));
        }
        self.repouso.em(agora_ms)
    }

    /// A reação (ou o estado segurado) tocando em `agora_ms`, se houver.
    pub fn reacao(&self, agora_ms: u64) -> Option<&str> {
        self.tocando
            .as_ref()
            .filter(|t| agora_ms < t.fim_ms)
            .map(|t| t.nome.as_str())
            .or_else(|| self.segurando.as_ref().map(|s| s.nome.as_str()))
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::skin::codificar_png;
    use crate::skin::testes::{skin_com_idle, skin_minima};

    fn skin_de_teste() -> Skin {
        let pasta = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skins/_teste");
        Skin::carregar(&pasta).expect("skins/_teste (cargo xtask skin-teste)")
    }

    fn tag(de: usize, ate: usize, direcao: Direcao) -> Tag {
        Tag {
            nome: "t".into(),
            de,
            ate,
            direcao,
        }
    }

    #[test]
    fn sequencias_das_quatro_direcoes() {
        assert_eq!(sequencia(&tag(2, 5, Direcao::Frente)), vec![2, 3, 4, 5]);
        assert_eq!(sequencia(&tag(2, 5, Direcao::Tras)), vec![5, 4, 3, 2]);
        assert_eq!(
            sequencia(&tag(2, 5, Direcao::PingPong)),
            vec![2, 3, 4, 5, 4, 3]
        );
        assert_eq!(
            sequencia(&tag(2, 5, Direcao::PingPongReverso)),
            vec![5, 4, 3, 2, 3, 4]
        );
        assert_eq!(sequencia(&tag(0, 1, Direcao::PingPong)), vec![0, 1]);
        assert_eq!(sequencia(&tag(3, 3, Direcao::PingPong)), vec![3]);
    }

    #[test]
    fn animacao_em_laco_e_uma_vez() {
        let skin = skin_minima(); // idle: quadros 0 (100 ms) e 1 (300 ms)
        let laco = Animacao::nova(&skin, 0, 1000, true);
        assert_eq!(laco.duracao_ms(), 400);
        assert_eq!(laco.em(1000), (0, Some(1100)));
        assert_eq!(laco.em(1099), (0, Some(1100)));
        assert_eq!(laco.em(1100), (1, Some(1400)));
        assert_eq!(laco.em(1400), (0, Some(1500)), "volta ao começo");
        assert_eq!(laco.em(500), (0, Some(1100)), "antes do início");
        let uma = Animacao::nova(&skin, 0, 0, false);
        assert_eq!(uma.em(150), (1, Some(400)));
        assert_eq!(uma.em(400), (1, None));
        // Ping-pong de 2 quadros: 0,1,0,1…
        let pp = Animacao::nova(&skin, 1, 0, true);
        assert_eq!(pp.em(100), (1, Some(400)));
        assert_eq!(pp.em(400), (0, Some(500)));
    }

    #[test]
    fn repouso_pose_e_rajada() {
        let skin = skin_minima();
        let r = Repouso::novo(&skin, 0);
        assert_eq!(r.pose(), 0);
        assert_eq!(r.em(0), (0, PAUSA_MS));
        assert_eq!(r.em(PAUSA_MS - 1), (0, PAUSA_MS));
        assert_eq!(r.em(PAUSA_MS), (0, PAUSA_MS + 100));
        assert_eq!(r.em(PAUSA_MS + 100), (1, PAUSA_MS + 400));
        assert_eq!(
            r.em(PAUSA_MS + 400),
            (0, 2 * PAUSA_MS + 400),
            "volta à pose"
        );
    }

    /// Simula seguindo os prazos: média de trocas de quadro (= commits) por
    /// segundo e o menor intervalo entre duas trocas.
    fn simular(r: &Repouso, segundos: u64) -> (f64, u64) {
        let fim = segundos * 1000;
        let (mut atual, mut t) = r.em(0);
        let mut commits = 1; // o primeiro quadro
        let mut ultima_troca = 0;
        let mut menor = u64::MAX;
        while t < fim {
            let (q, proxima) = r.em(t);
            if q != atual {
                commits += 1;
                atual = q;
                menor = menor.min(t - ultima_troca);
                ultima_troca = t;
            }
            assert!(proxima > t, "prazo não avança em {t}");
            t = proxima;
        }
        (commits as f64 / segundos as f64, menor)
    }

    #[test]
    fn repouso_cabe_no_orcamento_de_commits() {
        let skin = skin_minima();
        let (media, _) = simular(&Repouso::novo(&skin, 0), 600);
        assert!(media <= 2.0, "{media} commits/s");
        assert!(media > 0.0);
    }

    #[test]
    fn repouso_da_skin_de_teste_cabe_no_orcamento() {
        let skin = skin_de_teste();
        assert!(skin.tags_do_estado("idle").len() > 1, "várias tags de idle");
        let (media, menor) = simular(&Repouso::novo(&skin, 0), 600);
        assert!(media <= 2.0, "{media} commits/s");
        assert!(menor >= DURACAO_MIN_MS, "trocas a {menor} ms");
    }

    #[test]
    fn idle_denso_estica_a_pausa_ate_caber() {
        // 20 quadros de 50 ms: sem esticar, 20 trocas a cada 5 s (4/s).
        let skin = skin_com_idle(&[50; 20]);
        let r = Repouso::novo(&skin, 0);
        assert_eq!(
            r.trechos[0].pausa_ms, 9000,
            "20 trocas pedem 10 s por trecho"
        );
        let (media, _) = simular(&r, 600);
        assert!(media <= 2.0, "{media} commits/s");
    }

    #[test]
    fn quadro_repetido_nao_conta_como_troca() {
        // Quadros 0 e 2 são a mesma imagem (a folha guarda uma vez só): a
        // rajada 0,1,2,0 só troca de verdade 0→1 e 1→0.
        let png = codificar_png(4, 2, &[200u8; 32]).unwrap();
        let folha = r#"{"frames":[
            {"frame":{"x":0,"y":0,"w":2,"h":2},"spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":2,"h":2},"duration":100},
            {"frame":{"x":2,"y":0,"w":2,"h":2},"spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":2,"h":2},"duration":100},
            {"frame":{"x":0,"y":0,"w":2,"h":2},"spriteSourceSize":{"x":0,"y":0,"w":2,"h":2},"sourceSize":{"w":2,"h":2},"duration":100}],
            "meta":{"size":{"w":4,"h":2},"frameTags":[{"name":"idle","from":0,"to":2}]}}"#;
        let skin = r#"{"formato":1,"id":"rep","nome":"R","autor":"t","licenca":"MIT",
            "redistribuivel":true,"folha":"sheet.png","dados":"sheet.json",
            "celula":[2,2],"pe":[1,2],"toque":[0,0,2,2],"corpo_px":2,
            "estados":{"idle":["idle"]}}"#;
        let skin = Skin::de_partes(skin, folha, &png).unwrap();
        let a = Animacao::nova(&skin, 0, 0, false);
        assert_eq!(a.em(200), (0, Some(300)), "o quadro 2 toca como o 0");
        let r = Repouso::novo(&skin, 0);
        assert_eq!(trocas(r.pose(), &r.trechos[0].rajada), 2);
    }

    #[test]
    fn reacoes_com_reserva() {
        let skin = skin_de_teste();
        let tag = |nome: &str| skin.tags.iter().position(|t| t.nome == nome);
        // A skin de teste não tem `nod`: o aceno cai no `wave`.
        assert_eq!(tag_da_reacao(&skin, "nod"), tag("wave"));
        assert_eq!(tag_da_reacao(&skin, "done_small"), tag("done_small"));
        assert_eq!(tag_da_reacao(&skin, "alert"), tag("alert"));
        assert_eq!(
            tag_da_reacao(&skin, "bye"),
            None,
            "tchau sem animação no M3"
        );
        assert_eq!(tag_da_reacao(&skin, "nada_disso"), None);
        // Sem `wave` nem `done_small`, não há o que tocar.
        let mini = skin_minima();
        assert_eq!(tag_da_reacao(&mini, "done_small"), None);
        assert_eq!(tag_da_reacao(&mini, "festa"), Some(1), "estado da skin");
        assert_eq!(tag_da_reacao(&mini, "pula"), Some(1), "tag pelo nome");
    }

    /// Skin de duas tags de um quadro (`a` e `b`) com os `estados` dados.
    fn skin_com_estados(estados: &str) -> Skin {
        let png = codificar_png(2, 1, &[10, 20, 30, 255, 40, 50, 60, 255]).unwrap();
        let folha = r#"{"frames":[
            {"frame":{"x":0,"y":0,"w":1,"h":1},"spriteSourceSize":{"x":0,"y":0,"w":1,"h":1},"sourceSize":{"w":1,"h":1},"duration":100},
            {"frame":{"x":1,"y":0,"w":1,"h":1},"spriteSourceSize":{"x":0,"y":0,"w":1,"h":1},"sourceSize":{"w":1,"h":1},"duration":100}],
            "meta":{"size":{"w":2,"h":1},"frameTags":[{"name":"a","from":0,"to":0},{"name":"b","from":1,"to":1}]}}"#;
        let skin = format!(
            r#"{{"formato":1,"id":"est","nome":"E","autor":"t","licenca":"MIT",
            "redistribuivel":true,"folha":"sheet.png","dados":"sheet.json",
            "celula":[1,1],"pe":[0,1],"toque":[0,0,1,1],"corpo_px":1,"estados":{estados}}}"#
        );
        Skin::de_partes(&skin, folha, &png).unwrap()
    }

    #[test]
    fn reacao_segue_as_reservas_do_catalogo_sem_cair_no_repouso() {
        // Só com o pio do T1: o voo grande cai nele (done_big → done_medium
        // → done_small), como a cobertura mostra; o aceno não tem `wave` e,
        // sem a pose parada como reserva, não anima.
        let pio = skin_com_estados(r#"{"idle":["a"],"done_small":["b"]}"#);
        assert_eq!(tag_da_reacao(&pio, "done_big"), Some(1));
        assert_eq!(tag_da_reacao(&pio, "nod"), None, "nunca o idle");
        assert_eq!(tag_da_reacao(&pio, "working"), None);
        // Com `wave`: aceno, pulinho, oi e risadinha caem nele; tchau não.
        let aceno = skin_com_estados(r#"{"idle":["a"],"wave":["b"]}"#);
        for reacao in ["nod", "done_small", "hello", "giggle"] {
            assert_eq!(tag_da_reacao(&aceno, reacao), Some(1), "{reacao}");
        }
        assert_eq!(tag_da_reacao(&aceno, "bye"), None);
        // O estado nativo ganha da reserva.
        let ambos = skin_com_estados(r#"{"idle":["a"],"wave":["a"],"nod":["b"]}"#);
        assert_eq!(tag_da_reacao(&ambos, "nod"), Some(1));
    }

    #[test]
    fn reacao_toca_uma_vez_e_volta_a_pose() {
        let skin = skin_minima(); // festa → pula: 0 (100 ms), 1 (300 ms)
        let mut a = Animador::novo(&skin, 0);
        assert_eq!(a.em(1000), (0, PAUSA_MS), "repouso antes");
        assert!(a.tocar(&skin, "festa", 1000));
        assert_eq!(a.reacao(1000), Some("festa"));
        assert_eq!(a.em(1000), (0, 1100));
        assert_eq!(a.em(1100), (1, 1400));
        assert_eq!(a.reacao(1399), Some("festa"));
        // Fim: pose, e a pausa inteira antes da próxima rajada.
        assert_eq!(a.reacao(1400), None);
        assert_eq!(a.em(1400), (0, 1400 + PAUSA_MS));
        assert!(!a.tocar(&skin, "nada_disso", 2000), "reação desconhecida");
        assert_eq!(a.em(2000), (0, 1400 + PAUSA_MS), "nada mudou");
    }

    #[test]
    fn reacao_nova_substitui_a_que_toca() {
        let skin = skin_de_teste();
        let mut a = Animador::novo(&skin, 0);
        assert!(a.tocar(&skin, "nod", 0)); // wave em ping-pong: 6 passos de 150 ms
        assert_eq!(a.reacao(899), Some("nod"));
        assert!(a.tocar(&skin, "done_small", 300)); // 2 quadros de 200 ms
        assert_eq!(a.reacao(300), Some("done_small"));
        assert_eq!(a.em(300).1, 500);
        assert_eq!(a.reacao(700), None);
        assert_eq!(a.em(700).0, a.pose());
    }

    #[test]
    fn reacao_respeita_34_ms_por_quadro() {
        let skin = skin_com_idle(&[5, 10, 0]);
        let mut a = Animador::novo(&skin, 0);
        assert!(a.tocar(&skin, "idle", 0));
        let (_, p1) = a.em(0);
        let (_, p2) = a.em(p1);
        assert_eq!((p1, p2), (DURACAO_MIN_MS, 2 * DURACAO_MIN_MS));
    }

    #[test]
    fn reacoes_frequentes_cabem_no_orcamento() {
        // Uma reação a cada 20 s por 10 min, com a skin de teste.
        let skin = skin_de_teste();
        let mut a = Animador::novo(&skin, 0);
        let fim = 600_000;
        let (mut atual, mut t) = a.em(0);
        let mut commits = 1u64;
        let mut ultima_troca = 0;
        let mut menor = u64::MAX;
        let mut proxima_reacao = 20_000;
        while t < fim {
            if t >= proxima_reacao {
                let nome = if (proxima_reacao / 20_000) % 2 == 0 {
                    "nod"
                } else {
                    "done_small"
                };
                assert!(a.tocar(&skin, nome, t));
                proxima_reacao += 20_000;
            }
            let (q, proxima) = a.em(t);
            if q != atual {
                commits += 1;
                atual = q;
                menor = menor.min(t - ultima_troca);
                ultima_troca = t;
            }
            assert!(proxima > t, "prazo não avança em {t}");
            t = proxima.min(proxima_reacao.max(t + 1));
        }
        let media = commits as f64 / 600.0;
        assert!(media <= 2.0, "{media} commits/s");
        assert!(menor >= DURACAO_MIN_MS, "trocas a {menor} ms");
    }

    #[test]
    fn segurar_toca_em_laco_e_uma_reacao_no_meio_volta_ao_laco() {
        let skin = skin_de_teste();
        let mut a = Animador::novo(&skin, 0);
        assert!(a.segurar(&skin, "dangle", 1_000));
        assert!(a.segurado());
        assert_eq!(a.reacao(1_000), Some("dangle"));
        let dangle = skin.tags.iter().position(|t| t.nome == "dangle").unwrap();
        let primeiro = skin.canonico[skin.tags[dangle].de];
        let (q, proxima) = a.em(1_000);
        assert_eq!(q, primeiro);
        assert!(proxima > 1_000 && proxima < u64::MAX);
        // Bem depois, ainda no laço (não volta à pose sozinho).
        assert_eq!(a.reacao(60_000), Some("dangle"));
        assert_ne!(a.em(60_000).1, u64::MAX);
        // Uma reação no meio toca por cima e, no fim, o laço continua.
        assert!(a.tocar(&skin, "done_small", 2_000));
        assert_eq!(a.reacao(2_000), Some("done_small"));
        assert_eq!(a.reacao(2_400), Some("dangle"), "done_small dura 400 ms");
        // Largar: o repouso recomeça agora, com a pausa inteira.
        a.largar(&skin, 3_000);
        assert!(!a.segurado());
        assert_eq!(a.reacao(3_000), None);
        assert_eq!(a.em(3_000), (a.pose(), 3_000 + PAUSA_MS));
        // Largar sem segurar não mexe em nada.
        a.largar(&skin, 3_100);
        assert_eq!(a.em(3_100), (a.pose(), 3_000 + PAUSA_MS));
    }

    #[test]
    fn largar_no_meio_de_uma_reacao_deixa_a_reacao_acabar() {
        let skin = skin_de_teste();
        let mut a = Animador::novo(&skin, 0);
        assert!(a.segurar(&skin, "dangle", 0));
        assert!(a.tocar(&skin, "done_small", 100));
        a.largar(&skin, 200);
        assert_eq!(a.reacao(200), Some("done_small"));
        assert_eq!(a.reacao(500), None);
        assert_eq!(
            a.em(500),
            (a.pose(), 500 + PAUSA_MS),
            "repouso no fim da reação"
        );
        assert!(!a.segurar(&skin, "nada_disso", 600), "estado desconhecido");
    }

    #[test]
    fn quadros_curtos_nao_passam_de_30_fps() {
        let skin = skin_com_idle(&[10, 5, 0, 20, 10, 10]);
        let (media, menor) = simular(&Repouso::novo(&skin, 0), 600);
        assert!(menor >= DURACAO_MIN_MS, "trocas a {menor} ms");
        assert!(media <= 2.0, "{media} commits/s");
    }
}
