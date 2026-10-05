//! O Motor: tudo o que o pet decide, sem saber em que sistema está
//! (decisão 0040, T8.0).
//!
//! Antes morava espalhado no laço do daemon (`laco.rs`) e na sessão Wayland
//! (`wl/mod.rs`). Agora é puro e testado com relógio falso e uma janela
//! falsa:
//!
//! - o **cérebro** ([`Cerebro`]) e o prazo dele;
//! - o **personagem** (a skin aprovada), o **pet** (animação) e o **palco**
//!   (D e posição no monitor);
//! - **mostrar e esconder** ([`passo_de_visibilidade`]);
//! - o **estresse** de debug e a contagem de **commits**;
//! - o **painel** do `/v1/estado` e o quadro esperado da checagem de nitidez;
//! - os **prazos** em milissegundos de um relógio monotônico que o laço de
//!   cada sistema injeta: o laço só acorda no [`Motor::proximo_prazo`] e
//!   entrega os eventos da janela ([`EventoOverlay`]).
//!
//! O que é do sistema (pixels no buffer, quadro em voo, região de input,
//! prazos internos da janela) fica atrás do trait [`Overlay`].
//!
//! Desde o M4, o Motor também arrasta e clica ([`arraste`], decisão 0048):
//! o ponteiro chega no palco, o pet anda em múltiplos de D e a área de toque
//! cresce para o palco inteiro só enquanto arrasta. O clique esquerdo leva
//! ao terminal da sessão do aviso mais urgente ([`Motor::clicar`], decisão
//! 0057).
//!
//! No M5, o Motor anota cada decisão no registro de intenções ([`intencoes`],
//! decisão 0077), chama a cada aviso de espera novo e escala o mais velho
//! ([`escalada`], decisão 0075).

pub mod arraste;
pub mod balao;
mod desktop;
pub mod escalada;
pub mod intencoes;
pub mod janelas;
mod pet;
mod poof;
pub mod posicoes;
mod ritmo;
pub mod selos;
pub mod tela;
pub mod viagem;
pub mod voo;

use std::collections::BTreeMap;
use std::rc::Rc;

use serde::Serialize;

use crate::animador;
use crate::cena::{self, Elemento};
use crate::cerebro::{
    self, Agora, Cerebro, ConfigCerebro, EstadoSessao, Evidencia, Pendencia, Reacao, Resumo,
    TipoAviso, TipoEspera,
};
use crate::confete::{self, Chuva, Grade};
use crate::evento::Evento;
use crate::geometria::{Ret, Tamanho};
use crate::memoria::{Lida, Memoria, Recusa, Sossego, Volta};
use crate::plataforma::{
    Alca, Botao, Cursor, Desenho, ErroFoco, EventoDesktop, EventoOverlay, EventoPonteiro, Fase,
    Monitor, Overlay, Passo, Punho, passo_de_visibilidade,
};
use crate::skin::Skin;
use crate::sorteio::Sorteio;

pub use arraste::{Arraste, Gesto};
pub use balao::Balao;
pub use desktop::{EstadoDesktop, PainelDesktop};
pub use pet::{Palco, Pet};
pub use posicoes::{Fracao, Posicoes};
pub use tela::{PainelTela, Prioridade, Selos, Sono};
pub use viagem::{Pouso, Seguir};

use escalada::Escalada;
pub use ritmo::{Commits, Estresse, JANELA_COMMITS_MS};
use viagem::{Decisao, Fase as FaseViagem};

/// Confetes do teste de estresse, sempre com a mesma semente: medições
/// repetidas veem as mesmas trajetórias.
pub const CONFETES: usize = 40;
pub const SEMENTE_CONFETE: u64 = 7;
/// Lado de cada confete, em pixels de arte.
pub const LADO_CONFETE: i32 = 3;
/// O estado que o pet toca em laço enquanto é arrastado.
pub const ARRASTADO: &str = "dangle";
/// O estado que ele toca ao ser solto.
pub const SOLTO: &str = "land";
/// A risadinha do clique.
pub const RISADINHA: &str = "giggle";
/// O bocejo de quando a soneca começa e o despertar de quando ela acaba.
pub const BOCEJO: &str = "yawn";
pub const DESPERTAR: &str = "wake";
/// A soneca do botão direito (decisão 0053).
pub const SONECA_MS: u64 = 30 * 60 * 1000;
/// O pronto e o erro de uma sessão saem depois de tanto tempo com o
/// terminal dela em foco (decisão 0057).
pub const VISTO_PELO_FOCO_MS: u64 = 10_000;
/// O aviso de espera com o terminal da sessão em foco e o Renan presente por
/// isto: ele viu o diálogo, e a escalada não passa mais da L1 (decisão
/// 0090). Um Esc numa pergunta ou um plano recusado não mandam evento
/// nenhum: sem isto, o pet chamaria por um diálogo que não existe mais.
pub const ESPERA_VISTA_MS: u64 = 5_000;
/// A volta do Renan que não chegou à tela (a sessão bloqueada: o compositor
/// não mostra os quadros do voo) espera até isto os quadros voltarem.
pub const VOLTA_POR_MOSTRAR_MS: u64 = 2 * 60 * 1000;
/// Quanto o clique espera o desktop contar que a janela ficou ativa.
pub const CONFIRMAR_FOCO_MS: u64 = 1_500;
/// Quantos focos pedidos pelo clique esperam a confirmação ao mesmo tempo
/// (cliques seguidos; decisão 0062).
const FOCANDO_MAX: usize = 4;
/// Sem teclado nem mouse por isto, o Renan conta como longe: o terminal em
/// foco não dá o pronto como visto (decisão 0062). Menor que o
/// [`VISTO_PELO_FOCO_MS`]: quem saiu logo antes do aviso já é dado como
/// longe antes de o prazo do visto vencer.
pub const OCIOSO_MS: u64 = 5_000;
/// O clique seguinte continua a volta do ciclo dos avisos só até isto depois
/// do anterior; um clique depois recomeça do mais urgente (decisão 0062).
pub const VOLTA_DO_CICLO_MS: u64 = 15_000;
/// O coração da risadinha do clique que leva ao terminal.
pub const CORACAO_MS: u64 = 1_200;
/// O susto de quando um turno acaba num erro da API (decisão 0076).
pub const SUSTO: &str = "error";
/// A chamada de um aviso de espera (a L1) e as rajadas dela (decisão 0075).
pub const CHAMADA: &str = "alert";
/// O selo do aviso pulsa na L4 trocando de cor a cada tanto: um commit por
/// segundo (decisão 0083).
pub const PULSO_MS: u64 = 1_000;
/// Um passo do confete da festa: até 30 quadros por segundo (decisão 0085).
pub const PASSO_CONFETE_MS: u64 = 34;
/// O confete da festa acaba no máximo isto depois de começar.
pub const CONFETE_MAX_MS: u64 = 4_500;
/// O lado dos pedaços, em pixels de arte: a fonte do T2 e a chuva do T3.
pub const LADO_FONTE: i32 = 2;
pub const LADO_CHUVA: i32 = 3;

/// O que o pet publica para o `/v1/estado` (o laço publica; a entrada HTTP
/// só lê).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Painel {
    /// Nome do monitor da janela.
    pub monitor: Option<String>,
    /// Escala do monitor.
    pub escala: Option<f64>,
    /// Pixels do monitor por pixel de arte.
    pub d: Option<i32>,
    /// Célula do sprite no palco (pixels do monitor, relativa a ele) e
    /// recortada a ele (a foto e a checagem de nitidez recortam por aqui).
    pub sprite_disp: Option<Ret>,
    /// Região clicável nas coordenadas da janela (lógicas, no Wayland).
    pub regiao_entrada: Option<Ret>,
    /// O pet está desenhado na tela.
    pub visivel: bool,
    /// Teste de estresse rodando.
    pub estresse: bool,
    /// Commits no último minuto (orçamento: até 120 parado).
    pub commits_por_min: usize,
    pub commits_total: u64,
    /// Memória compartilhada dos buffers (o compositor guarda uma textura do
    /// mesmo tamanho, fora do processo).
    pub shm_bytes: usize,
    /// A reação tocando na tela agora (`nod`, `done_small`, …).
    pub reacao: Option<String>,
    /// O pet está sendo arrastado.
    pub arrastando: bool,
    /// A fase da viagem para o monitor em foco (`poof`, `saindo`,
    /// `chegando`, `entrando`), se houver uma.
    pub viagem: Option<&'static str>,
    /// As linhas do balão na tela, se houver um.
    pub balao: Option<Vec<String>>,
    /// Quanto falta da soneca (o botão direito), se o pet está cochilando.
    pub soneca_restante_s: Option<u64>,
    /// O desktop: a fonte dos eventos, o monitor em foco, a janela ativa (só
    /// o endereço) e o que a conexão sabe fazer.
    pub desktop: PainelDesktop,
    /// O clique pediu o foco desta janela (o endereço) e espera o desktop
    /// contar que ela ficou ativa (decisão 0057); com cliques seguidos, a do
    /// mais novo (decisão 0062).
    pub focando: Option<String>,
    /// As últimas decisões do Motor, da mais nova para a mais velha (decisão
    /// 0077).
    pub intencoes: Vec<intencoes::NoPainel>,
    /// A fotografia da tela de agora: a base, os selos, a escalada, a festa
    /// e a discrição (decisões 0076, 0077 e 0080).
    pub fotografia: PainelTela,
    /// O que a janela está desenhando agora: a base no animador, a fileira
    /// de selos, o voo e o confete (decisão 0086). Só metadados.
    pub desenho: PainelDesenho,
}

/// O desenho de agora no `/v1/estado.desenho` (decisão 0086): o que as
/// intenções viraram na janela. Sem pet na tela, vazio.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PainelDesenho {
    /// O estado da skin que o animador segura (a base, decisão 0082).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// O ritmo dela: `repouso`, `quieto`, `laco` ou `parado`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ritmo: Option<&'static str>,
    /// A fileira de selos ao lado do corpo, se há uma (decisão 0083).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selos: Option<PainelFileira>,
    /// O voo da escalada, se há um (decisão 0084).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voo: Option<PainelVoo>,
    /// Os pedaços de confete da festa na tela (decisão 0085).
    pub confete: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PainelFileira {
    /// O selo do aviso: `normal` ou `aceso` (a metade acesa do pulso).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aviso: Option<&'static str>,
    /// O selo do aviso pulsa (a L4).
    pub pulso: bool,
    pub mais: u32,
    pub corrente: bool,
    pub bandeiras: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PainelVoo {
    /// `subindo`, `pairando` ou `descendo`.
    pub fase: &'static str,
    /// `escalada` ou `voltou`.
    pub motivo: &'static str,
}

/// O que um `tocar` do `/v1/comando` fez (decisão 0033).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tocou {
    /// Tocou na tela a tag `tag` da skin.
    NaTela { tag: String },
    /// A skin sabe tocar (`tag`), mas não há onde mostrar: sem compositor,
    /// ou o pet escondido.
    ForaDaTela { tag: String, motivo: &'static str },
    /// Nenhum personagem aprovado.
    SemPersonagem,
    /// A skin não tem estado, reserva nem tag com esse nome.
    Desconhecida { skin: String },
}

/// O que um clique no pet fez (decisão 0057): o do ponteiro e o do
/// `/v1/comando` `clique`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clicou {
    /// Pediu o foco da janela do terminal da sessão do aviso da vez.
    /// `confirmado`: ela já estava ativa, ou o desktop não conta trocas, e o
    /// aviso já saiu; senão ele sai quando o desktop contar que ela ficou
    /// ativa.
    Focou {
        sid8: String,
        aviso: TipoAviso,
        janela: String,
        confirmado: bool,
    },
    /// Havia aviso, mas não deu para focar: o balão diz o porquê e mostra as
    /// sessões.
    NaoFocou {
        sid8: String,
        aviso: TipoAviso,
        motivo: &'static str,
    },
    /// Nenhum aviso: o balão com as sessões abertas.
    Lista { sessoes: usize },
    /// O botão direito: a soneca começou ou acabou.
    Soneca { cochilando: bool },
    /// Nada (outro botão; sem a janela do pet).
    Nada { motivo: &'static str },
}

/// Um foco pedido pelo clique, à espera de o desktop contar que a janela
/// ficou ativa.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Focando {
    chave: janelas::Chave,
    janela: Alca,
    proj: Option<String>,
    tipo: TipoAviso,
    ate_ms: u64,
}

/// O confete de uma festa na tela (decisão 0085): os pedaços andam um passo a
/// cada [`PASSO_CONFETE_MS`], contados do começo.
#[derive(Debug, Clone)]
struct EfeitoFesta {
    confete: confete::Festa,
    inicio_ms: u64,
    passos: u64,
    fim_ms: u64,
}

/// `t` levado para a frente, até a grade de `passo` que começa em `base`.
fn na_grade(base: u64, passo: u64, t: u64) -> u64 {
    if t <= base {
        return base;
    }
    base + (t - base).div_ceil(passo) * passo
}

/// O aviso de espera que o pet está chamando (decisão 0075): o mais velho
/// (os outros viram o "+N").
#[derive(Debug, Clone, PartialEq, Eq)]
struct Chamando {
    chave: janelas::Chave,
    sid8: String,
    proj: Option<String>,
    /// O tipo da espera anunciado (sobe com o gatilho do mesmo diálogo).
    espera: Option<TipoEspera>,
    /// Quando o aviso abriu (ms desde 1970): outro aviso da mesma sessão é
    /// outra escalada.
    desde_ms: u64,
    escalada: Escalada,
    /// Quando o Renan viu o diálogo no terminal da sessão (relógio do laço;
    /// decisão 0090): daí em diante, nada passa da L1. De antes da partida
    /// num aviso restaurado (decisão 0093).
    vista_ms: Option<cerebro::Instante>,
    /// A mesma hora, na parede, anotada quando ele viu (a memória das
    /// sessões a grava sem refazer a conta a cada batimento; decisão 0095).
    vista_parede: Option<u64>,
}

/// O sprite como deveria estar na tela: o RGBA exato, em pixels do monitor,
/// para a checagem de nitidez comparar com uma captura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuadroEsperado {
    pub monitor: String,
    /// Recorte em pixels do monitor (o `sprite_disp`).
    pub area: Ret,
    /// Canto da célula: origem da grade de blocos D×D.
    pub grade: (i32, i32),
    pub d: i32,
    /// Número do commit que pôs este quadro na tela.
    pub seq: u64,
    /// Há quanto tempo foi esse commit.
    pub idade_ms: u64,
    /// RGBA direto, `area.w * area.h * 4` bytes.
    pub rgba: Vec<u8>,
}

/// O que a memória das sessões trouxe de volta na partida (decisão 0093).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Restauracao {
    pub sessoes: usize,
    pub avisos: usize,
    /// As sessões que voltaram com a janela do terminal.
    pub janelas: usize,
    /// As que ficaram de fora: expiradas, de outra origem, repetidas, além
    /// do teto ou com um campo ruim no arquivo.
    pub de_fora: usize,
    /// A memória era velha (decisão 0095): as esperas voltaram vistas e as
    /// janelas sem o endereço.
    pub velha: bool,
}

/// O pet inteiro, sem sistema. Os tempos são milissegundos do relógio
/// monotônico do laço (o mesmo do `mono_ms` do cérebro).
pub struct Motor {
    cerebro: Cerebro,
    /// O personagem: a skin aprovada (ou a de teste, em debug).
    skin: Option<Rc<Skin>>,
    /// O pet animado; só existe com uma janela (sessão com o compositor).
    pet: Option<Pet>,
    /// Onde o pet fica no palco do monitor atual (D e posição).
    palco: Option<Palco>,
    /// O pet deve estar na tela (`/v1/comando` esconde e mostra).
    visivel: bool,
    estresse: Option<Estresse>,
    commits: Commits,
    /// A próxima troca de quadro da animação.
    proximo_quadro: Option<u64>,
    /// O tamanho do pet na tela (`aparencia.tamanho`, decisão 0042).
    tamanho: Tamanho,
    /// O que os eventos do desktop contaram (decisão 0043).
    desktop: EstadoDesktop,
    /// Arrastar e clicar (decisão 0048).
    arraste: Arraste,
    /// Onde o Renan deixou o pet em cada monitor (decisão 0049).
    posicoes: Posicoes,
    /// As posições mudaram desde a última gravação.
    posicoes_novas: bool,
    /// A chave do monitor do palco de agora.
    chave_do_monitor: Option<String>,
    /// Seguir o monitor ativo (decisão 0051).
    seguir: Seguir,
    /// O balão na tela (decisão 0052).
    balao: Option<Balao>,
    /// Parede − monotônico, do último [`Agora`] que o Motor viu: a hora de
    /// parede de um instante do relógio do laço (o "há quanto tempo" do
    /// balão).
    deslocamento_parede: u64,
    /// Até quando o pet cochila (o botão direito; decisão 0053).
    soneca_ate: Option<u64>,
    /// A janela do terminal de cada sessão (decisão 0055).
    identidades: janelas::Identidades,
    /// Os focos que o clique pediu e o desktop ainda não confirmou, do mais
    /// velho ao mais novo (decisões 0057 e 0062).
    focando: Vec<Focando>,
    /// As sessões que o clique já visitou nesta volta do ciclo de avisos.
    ciclo: Vec<janelas::Chave>,
    /// Quando foi o último clique esquerdo (a volta do ciclo).
    ultimo_clique_ms: Option<u64>,
    /// Desde quando (relógio do laço) a janela ativa de agora está ativa.
    ativa_desde: u64,
    /// Desde quando (relógio do laço) o Renan mexe no teclado ou no mouse,
    /// pelo desktop (decisão 0062).
    presente_desde: u64,
    /// Até quando o coração da risadinha fica na tela.
    coracao_ate: Option<u64>,
    /// Um aviso saiu fora de um evento do Claude ou de um tique (o clique, o
    /// foco): o núcleo publica o cérebro de novo.
    cerebro_mudou: bool,
    /// As decisões, normalizadas (decisão 0077).
    intencoes: intencoes::Registro,
    /// O estado de cada sessão da última vez que o Motor olhou o cérebro
    /// (e desde quando): a entrada no erro e no cansado toca uma vez só
    /// (decisão 0076).
    estados_vistos: BTreeMap<janelas::Chave, (EstadoSessao, u64)>,
    /// Os avisos de espera já chamados (a L1), com a hora em que abriram e
    /// o tipo anunciado: um gatilho do mesmo diálogo não chama de novo
    /// (decisão 0075).
    chamados: BTreeMap<janelas::Chave, (u64, Option<TipoEspera>)>,
    /// A escalada do aviso de espera mais velho.
    chamando: Option<Chamando>,
    /// Desde quando (relógio do laço) o desktop diz que o Renan está longe
    /// do teclado e do mouse (o "sem mexer há 60 s" e a volta).
    ausente_desde: Option<u64>,
    /// O "não perturbe" do Omarchy, pelo `dnd` do último evento do Claude.
    nao_perturbe: bool,
    /// O instante mais novo do relógio do laço que o Motor viu (os prazos
    /// calculados sem um `agora`).
    relogio_ms: u64,
    /// A festa, a base, os selos, o sono e a discrição anunciados (decisão
    /// 0076).
    tela: tela::Tela,
    /// O sorteio de cada pet novo (as micro-ações da base; decisão 0082).
    sorteio: Sorteio,
    /// Desde quando o selo do aviso pulsa (a L4; decisão 0083).
    pulso_desde: Option<u64>,
    /// O voo da escalada até o alto-centro, se há um (decisão 0084).
    voo: Option<voo::Voo>,
    /// Quadros que o voo de agora mandou à janela: com a sessão bloqueada o
    /// compositor segura o primeiro, e o voo acaba sem ninguém ver.
    voo_quadros: u32,
    /// O voo da volta do Renan que não chegou à tela (sem palco, ou sem
    /// quadros mostrados) sai de novo quando a janela mostrar quadros, até
    /// este instante (decisão 0090).
    volta_por_mostrar: Option<u64>,
    /// O confete da festa na tela, se há um (decisão 0085).
    efeito: Option<EfeitoFesta>,
    /// As esperas que voltaram de uma memória velha (decisão 0095), com o
    /// aviso (a hora de parede em que abriu) e a gravação (no relógio do laço
    /// e na parede): dadas como vistas na gravação, nunca passam da L1,
    /// também quando a vez delas chega depois.
    esperas_velhas: BTreeMap<janelas::Chave, (u64, cerebro::Instante, u64)>,
}

impl Motor {
    pub fn novo(config: ConfigCerebro) -> Motor {
        Motor {
            cerebro: Cerebro::novo(config),
            skin: None,
            pet: None,
            palco: None,
            visivel: true,
            estresse: None,
            commits: Commits::default(),
            proximo_quadro: None,
            tamanho: Tamanho::Normal,
            desktop: EstadoDesktop::default(),
            arraste: Arraste::default(),
            posicoes: Posicoes::default(),
            posicoes_novas: false,
            chave_do_monitor: None,
            seguir: Seguir::default(),
            balao: None,
            deslocamento_parede: 0,
            soneca_ate: None,
            identidades: janelas::Identidades::default(),
            focando: Vec::new(),
            ciclo: Vec::new(),
            ultimo_clique_ms: None,
            ativa_desde: 0,
            presente_desde: 0,
            coracao_ate: None,
            cerebro_mudou: false,
            intencoes: intencoes::Registro::default(),
            estados_vistos: BTreeMap::new(),
            chamados: BTreeMap::new(),
            chamando: None,
            ausente_desde: None,
            nao_perturbe: false,
            relogio_ms: 0,
            tela: tela::Tela::default(),
            sorteio: Sorteio::default(),
            pulso_desde: None,
            voo: None,
            voo_quadros: 0,
            volta_por_mostrar: None,
            efeito: None,
            esperas_velhas: BTreeMap::new(),
        }
    }

    /// A semente do sorteio (o daemon semeia pela hora da partida; sem
    /// semear, a fixa dos testes). Vale para o próximo pet.
    pub fn semear(&mut self, semente: u64) {
        self.sorteio = Sorteio::novo(semente);
    }

    /// Um pet novo com o personagem, na base de agora (decisão 0082).
    fn pet_novo(&mut self, skin: Rc<Skin>, agora_ms: u64) -> Pet {
        let mut pet = Pet::com_semente(skin, agora_ms, self.sorteio.proximo());
        pet.definir_base(self.base_desejada(), agora_ms);
        pet
    }

    // --- intenções (decisão 0077) --------------------------------------------

    /// Guarda cada intenção nova até [`Self::tirar_intencoes_novas`] (o
    /// executor dos cenários; o daemon não pede).
    pub fn gravar_todas_as_intencoes(&mut self) {
        self.intencoes.gravar_tudo();
    }

    /// As intenções desde a última vez.
    pub fn tirar_intencoes_novas(&mut self) -> Vec<intencoes::Intencao> {
        self.intencoes.tirar_novas()
    }

    /// As últimas intenções guardadas, da mais velha para a mais nova.
    pub fn intencoes(&self) -> impl Iterator<Item = &intencoes::Intencao> {
        self.intencoes.ultimas()
    }

    fn anotar(&mut self, t_ms: u64, tipo: intencoes::Tipo) {
        self.intencoes.anotar(t_ms, tipo);
    }

    /// Os turnos que o cérebro fechou viram intenções, e as reações dele
    /// viram festas ([`Self::festejar`], com a mesclagem e a soneca; decisão
    /// 0076), mais o que o Motor vê nas sessões: a entrada no erro e no
    /// cansado ([`Self::observar_cerebro`]), os avisos de espera
    /// ([`Self::observar_avisos`]) e a base, os selos e o sono
    /// ([`Self::observar_tela`]). O que volta é o que o animador deve tocar.
    fn depois_do_cerebro(&mut self, reacoes: Vec<Reacao>, agora_ms: u64) -> Vec<Reacao> {
        for registro in self.cerebro.tirar_turnos_fechados() {
            self.anotar(agora_ms, intencoes::Tipo::do_turno(&registro));
        }
        let mut reacoes = self.festejar(reacoes, agora_ms);
        reacoes.extend(self.observar_cerebro(agora_ms));
        reacoes.extend(self.observar_avisos(agora_ms, None));
        reacoes.extend(self.observar_tela(agora_ms));
        reacoes
    }

    /// Acerta o relógio de parede pelo de agora (o núcleo chama a cada lote).
    pub fn acertar_relogio(&mut self, agora: Agora) {
        self.deslocamento_parede = agora.parede_ms.saturating_sub(agora.mono_ms);
        self.relogio_ms = self.relogio_ms.max(agora.mono_ms);
    }

    /// A hora de parede (ms desde 1970) de um instante do relógio do laço.
    fn parede(&self, mono_ms: u64) -> u64 {
        mono_ms + self.deslocamento_parede
    }

    // --- o balão (decisão 0052) ----------------------------------------------

    /// Mostra um balão com `linhas` em cima do pet (some sozinho).
    pub fn mostrar_balao(
        &mut self,
        ov: Option<&mut dyn Overlay>,
        linhas: Vec<String>,
        agora_ms: u64,
    ) {
        self.mostrar_balao_por(ov, linhas, "pedido", agora_ms);
    }

    /// [`Self::mostrar_balao`], com o motivo nas intenções.
    fn mostrar_balao_por(
        &mut self,
        ov: Option<&mut dyn Overlay>,
        linhas: Vec<String>,
        motivo: &'static str,
        agora_ms: u64,
    ) {
        let balao = Balao::novo(linhas, agora_ms);
        self.anotar(
            agora_ms,
            intencoes::Tipo::Balao {
                linhas: balao.linhas.clone(),
                motivo,
            },
        );
        self.balao = Some(balao);
        if let Some(ov) = ov {
            self.desenhar(ov, agora_ms, false);
        }
    }

    /// O balão na tela agora.
    pub fn balao(&self, agora_ms: u64) -> Option<&Balao> {
        self.balao.as_ref().filter(|b| agora_ms < b.ate_ms)
    }

    /// As sessões abertas, uma por linha (projeto, estado e há quanto
    /// tempo), as reais antes das de teste, a mais recente primeiro.
    /// Na discrição do compartilhamento de tela, cada sessão é "sessão N"
    /// (decisão 0076).
    pub fn linhas_das_sessoes(&self, agora_ms: u64) -> Vec<String> {
        let agora = self.parede(agora_ms);
        let sessoes = self.sessoes_da_lista();
        if sessoes.is_empty() {
            return vec!["nenhuma sessão do Claude aberta".into()];
        }
        let nomes = self.nomes_visiveis();
        sessoes
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let (estado, desde) = balao::estado(s);
                let rotulo = if nomes {
                    s.proj.clone()
                } else {
                    Some(format!("sessão {}", i + 1))
                };
                balao::linha_da_sessao(rotulo.as_deref(), estado, desde, agora, s.teste)
            })
            .collect()
    }

    /// As sessões na ordem da lista do clique: as reais antes das de teste,
    /// a de evento mais novo primeiro.
    fn sessoes_da_lista(&self) -> Vec<cerebro::ResumoSessao> {
        let mut sessoes = self.cerebro.resumo_das_sessoes();
        sessoes.sort_by_key(|s| (s.teste, std::cmp::Reverse(s.ultimo_evento_ms)));
        sessoes
    }

    // --- os selos e o selo do aviso (decisão 0083) ---------------------------

    /// O que mudou na tela (os selos, o aviso, o pulso, a base) vai para o
    /// próximo quadro, já. Sem mudança de verdade, a janela não faz commit.
    pub(super) fn redesenhar_ja(&mut self, agora_ms: u64) {
        if self.pet.is_some() {
            self.proximo_quadro = Some(self.proximo_quadro.map_or(agora_ms, |p| p.min(agora_ms)));
        }
    }

    /// A fileira de selos de agora: o "!" do aviso de espera que o pet chama
    /// (no pulso da L4, aceso nos segundos ímpares), o "+N", o "…" e as
    /// bandeirinhas.
    fn fileira(&self, agora_ms: u64) -> selos::Fileira {
        let anunciados = self.selos_na_tela();
        let aviso = self.chamando.as_ref().map(|_| match self.pulso_desde {
            Some(desde) if (agora_ms.saturating_sub(desde) / PULSO_MS) % 2 == 1 => {
                selos::Aviso::Aceso
            }
            _ => selos::Aviso::Normal,
        });
        selos::Fileira {
            aviso,
            mais: anunciados.mais,
            corrente: anunciados.corrente,
            bandeiras: anunciados.bandeiras.clone(),
        }
    }

    /// A próxima troca de cor do selo do aviso, se ele pulsa.
    fn proxima_troca_do_pulso(&self, agora_ms: u64) -> Option<u64> {
        let desde = self.pulso_desde.filter(|_| self.chamando.is_some())?;
        let passos = agora_ms.saturating_sub(desde) / PULSO_MS;
        Some(desde + (passos + 1) * PULSO_MS)
    }

    // --- o voo da escalada (decisão 0084) -------------------------------------

    /// O voo até o alto-centro do monitor e de volta (a L3 e a volta do
    /// Renan). Nunca arrastando, viajando entre monitores (o poof inclusive),
    /// escondido, na proteção de tela, na soneca nem com o "não perturbe"; um
    /// de cada vez. A célula anda e a casa volta no fim: nada grava posição.
    /// `true` se o voo começou.
    fn comecar_voo(&mut self, motivo: &'static str, agora_ms: u64) -> bool {
        // Um voo que acabou sem quadros (a tela apagada, a sessão bloqueada:
        // o desenho espera o compositor) termina agora, pelo relógio, e não
        // segura o próximo (a volta do Renan).
        self.andar_voo(agora_ms);
        if self.voo.is_some()
            || !self.na_tela()
            || self.arraste.segurando()
            || self.seguir.em_viagem()
            || self.soneca(agora_ms).is_some()
            || self.nao_perturbe
            || self.tela.discreto
            || self.estresse.is_some()
        {
            return false;
        }
        let (Some(palco), Some(pet)) = (self.palco, self.pet.as_mut()) else {
            return false;
        };
        let alvo = alvo_do_voo(&palco, pet);
        pet.segurar(ARRASTADO, agora_ms);
        let voo = voo::Voo::novo(agora_ms, (palco.x, palco.y), alvo, palco.d, motivo);
        info!(
            "voo da escalada ({motivo}): da célula em ({}, {}) a ({}, {})",
            voo.casa.0, voo.casa.1, voo.alvo.0, voo.alvo.1
        );
        self.voo = Some(voo);
        self.voo_quadros = 0;
        if motivo == "voltou" {
            self.volta_por_mostrar = None;
        }
        self.redesenhar_ja(agora_ms);
        true
    }

    /// O voo anda até `agora_ms`: a célula na posição dele; no fim, a casa, o
    /// pet larga o voo e pousa. O voo da volta que acabou sem a janela
    /// mostrar quadro nenhum (a sessão bloqueada) fica para quando ela
    /// mostrar (decisão 0090).
    fn andar_voo(&mut self, agora_ms: u64) {
        let Some(voo) = self.voo else {
            return;
        };
        let Some(palco) = self.palco.as_mut() else {
            // Sem palco (a janela fechou no meio do voo), o voo acaba, e o pet
            // larga o voo: senão ficaria batendo as asas em laço na janela
            // nova, com uns 12 commits por segundo.
            self.voo = None;
            if let Some(pet) = self.pet.as_mut() {
                pet.largar(agora_ms);
            }
            return;
        };
        (palco.x, palco.y) = voo.posicao(agora_ms);
        if voo.fase(agora_ms).is_none() {
            self.voo = None;
            if voo.motivo == "voltou" && self.voo_quadros < 2 && self.chamando.is_some() {
                self.volta_por_mostrar = Some(agora_ms + VOLTA_POR_MOSTRAR_MS);
            }
            if let Some(pet) = self.pet.as_mut() {
                pet.largar(agora_ms);
                // O pouso só na hora: com a tela apagada o voo não andou, e o
                // pet não pousa do nada quando ela acende.
                if agora_ms < voo.fim_ms() + 1_000 {
                    pet.tocar(SOLTO, agora_ms);
                }
            }
        }
    }

    /// A janela mostra quadros (o primeiro palco, ou um quadro que estava
    /// preso foi mostrado): o voo da volta que não chegou à tela sai agora,
    /// se o aviso ainda espera (decisão 0090).
    fn mostrar_a_volta(&mut self, agora_ms: u64) {
        let Some(ate) = self.volta_por_mostrar else {
            return;
        };
        if agora_ms >= ate || self.chamando.is_none() {
            self.volta_por_mostrar = None;
            return;
        }
        if self.voo.is_none() && self.comecar_voo("voltou", agora_ms) {
            info!("o voo da volta do Renan sai agora, com a janela mostrando os quadros");
        }
    }

    /// O voo acaba já: o pet larga o voo e, com `restaurar`, volta para a
    /// casa (esconder, viajar); sem, fica onde está (o arraste pegou ele no
    /// ar; um palco novo é montado na posição salva).
    fn cancelar_voo(&mut self, restaurar: bool, agora_ms: u64) {
        let Some(voo) = self.voo.take() else {
            return;
        };
        if restaurar && let Some(palco) = self.palco.as_mut() {
            (palco.x, palco.y) = voo.casa;
        }
        if let Some(pet) = self.pet.as_mut() {
            pet.largar(agora_ms);
        }
    }

    /// Manda o voo de volta para a casa (a escalada acabou, a soneca, o "não
    /// perturbe").
    fn voltar_do_voo(&mut self, agora_ms: u64) {
        if let Some(voo) = self.voo.as_mut() {
            voo.voltar(agora_ms);
            self.redesenhar_ja(agora_ms);
        }
    }

    /// O voo em curso, se há um.
    pub fn voo(&self) -> Option<&voo::Voo> {
        self.voo.as_ref()
    }

    // --- o confete da festa (decisão 0085) ------------------------------------

    /// O confete de uma festa: a chuva do alto (o T3) ou a fonte da cabeça (o
    /// T2), com `quantos` pedaços, no lugar de um que estiver caindo (a festa
    /// mesclada que sobe de nível).
    pub(super) fn comecar_confete(&mut self, chuva: bool, quantos: u32, agora_ms: u64) {
        let (Some(palco), Some(pet)) = (self.palco, self.pet.as_ref()) else {
            return;
        };
        if quantos == 0 {
            return;
        }
        let grade = Grade {
            x: palco.x,
            y: palco.y,
            d: palco.d,
        };
        let toque = pet.skin().ancoras.toque(false);
        let semente = self.sorteio.proximo();
        let confete = if chuva {
            confete::Festa::chuva(quantos as usize, palco.tela, grade, LADO_CHUVA, semente)
        } else {
            let cabeca = (toque.x + toque.w / 2, toque.y);
            confete::Festa::fonte(
                quantos as usize,
                palco.tela,
                grade,
                cabeca,
                LADO_FONTE,
                semente,
            )
        };
        self.efeito = Some(EfeitoFesta {
            confete,
            inicio_ms: agora_ms,
            passos: 0,
            fim_ms: agora_ms + CONFETE_MAX_MS,
        });
        self.redesenhar_ja(agora_ms);
    }

    /// Os pedaços de confete na tela em `agora_ms`, pelo relógio (0 sem
    /// festa): os passos que venceram andam numa cópia, como a janela vai
    /// mostrar (com a tela apagada, o desenho espera o compositor).
    pub fn confete_na_tela(&self, agora_ms: u64) -> usize {
        let Some(efeito) = self.efeito.as_ref().filter(|e| agora_ms < e.fim_ms) else {
            return 0;
        };
        let mut confete = efeito.confete.clone();
        let devidos = agora_ms.saturating_sub(efeito.inicio_ms) / PASSO_CONFETE_MS;
        for _ in efeito.passos..devidos {
            if confete.acabou() {
                break;
            }
            confete.passo();
        }
        confete.na_tela().count()
    }

    // --- posições salvas (decisão 0049) --------------------------------------

    /// As posições lidas de `/state` na partida.
    pub fn definir_posicoes(&mut self, posicoes: Posicoes) {
        self.posicoes = posicoes;
        self.posicoes_novas = false;
    }

    pub fn posicoes(&self) -> &Posicoes {
        &self.posicoes
    }

    /// O arquivo das posições, se elas mudaram desde a última vez (quem
    /// grava é o núcleo do daemon).
    pub fn posicoes_para_gravar(&mut self) -> Option<String> {
        if !self.posicoes_novas {
            return None;
        }
        self.posicoes_novas = false;
        Some(self.posicoes.json())
    }

    /// Guarda onde o pet está agora, no monitor do palco.
    fn guardar_posicao(&mut self) {
        let (Some(palco), Some(pet), Some(chave)) = (
            self.palco,
            self.pet.as_ref(),
            self.chave_do_monitor.as_deref(),
        ) else {
            return;
        };
        let fracao = Fracao::da_celula(palco.x, palco.y, palco.d, &pet.skin().ancoras, palco.tela);
        if self.posicoes.de(chave) != Some(fracao) {
            self.posicoes.guardar(chave, fracao);
            self.posicoes_novas = true;
        }
    }

    // --- cérebro -----------------------------------------------------------

    pub fn config_do_cerebro(&self) -> &ConfigCerebro {
        self.cerebro.config()
    }

    pub fn reconfigurar_cerebro(&mut self, config: ConfigCerebro) {
        self.cerebro.reconfigurar(config);
    }

    /// O que mudou nas sessões desde a última olhada e pede uma reação: a
    /// entrada no erro (o susto e "Deu ruim...") e no cansado (o bocejo e
    /// "Cansei..."), uma vez por entrada (decisão 0076).
    fn observar_cerebro(&mut self, agora_ms: u64) -> Vec<Reacao> {
        let sessoes = self.cerebro.resumo_das_sessoes();
        let mut reacoes = Vec::new();
        let mut vistos = BTreeMap::new();
        for s in &sessoes {
            let agora_dela = (s.estado, s.estado_desde_ms);
            let nova_entrada = self.estados_vistos.get(&s.chave) != Some(&agora_dela);
            vistos.insert(s.chave.clone(), agora_dela);
            if !nova_entrada {
                continue;
            }
            // Escondido ou na proteção de tela, nem a linha nem o balão
            // (decisões 0079 e 0091): a entrada fica vista, e nada se repete
            // na volta.
            if !self.na_tela() {
                continue;
            }
            let (mut nome, motivo, linha) = match s.estado {
                EstadoSessao::Erro => (
                    SUSTO,
                    "erro",
                    balao::com_projeto(
                        "Deu ruim...",
                        self.rotulo(&s.chave, s.proj.as_deref()).as_deref(),
                    ),
                ),
                EstadoSessao::Cansado => (BOCEJO, "cansado", "Cansei...".to_owned()),
                _ => continue,
            };
            // Na soneca, só as reações pequenas (decisão 0053).
            if self.soneca(agora_ms).is_some() {
                nome = cerebro::ACENO;
            }
            self.anotar(
                agora_ms,
                intencoes::Tipo::Reacao {
                    nome: nome.to_owned(),
                    motivo,
                    sid8: Some(s.sid8.clone()),
                    nivel: None,
                },
            );
            self.balao_decidido(vec![linha], motivo, agora_ms);
            reacoes.push(Reacao {
                nome,
                sid8: s.sid8.clone(),
                proj: s.proj.clone(),
                ts: self.parede(agora_ms),
                nivel: None,
                teste: s.teste,
                discreta: false,
            });
        }
        self.estados_vistos = vistos;
        reacoes
    }

    // --- avisos de espera e a escalada (decisão 0075) --------------------------

    /// O pet pode aparecer para chamar ou festejar: ninguém mandou esconder e
    /// a proteção de tela não está na tela. Não depende de haver personagem:
    /// as intenções são as mesmas sem skin.
    fn na_tela(&self) -> bool {
        self.visivel && !self.desktop.protetor_ativo()
    }

    /// Um balão que o Motor decidiu (o erro, a chamada): na intenção e na
    /// tela, se o pet pode aparecer (escondido, nada toca). Sem a janela
    /// aqui, o próximo quadro o desenha, já.
    fn balao_decidido(&mut self, linhas: Vec<String>, motivo: &'static str, agora_ms: u64) {
        if !self.na_tela() {
            return;
        }
        self.mostrar_balao_por(None, linhas, motivo, agora_ms);
        if self.pet.is_some() {
            self.proximo_quadro = Some(self.proximo_quadro.map_or(agora_ms, |p| p.min(agora_ms)));
        }
    }

    /// O que a escalada sabe agora: se o Renan precisa ser chamado (não está
    /// olhando o terminal da sessão que espera, ou está sem mexer há 60 s ou
    /// mais; o desktop conta o longe depois de [`OCIOSO_MS`] sem mexer; e
    /// nunca depois de ele ter visto o diálogo lá), os tetos e se o pet pode
    /// aparecer.
    fn contexto_da_escalada(&self, agora_ms: u64) -> escalada::Contexto {
        let parado_desde = match (self.desktop.ocioso, self.ausente_desde) {
            (Some(true), Some(desde)) => Some(desde.saturating_sub(OCIOSO_MS)),
            _ => None,
        };
        let vista = self.chamando.as_ref().is_some_and(|c| c.vista_ms.is_some());
        let chama_em = parado_desde
            .map(|d| d + escalada::PARADO_PARA_CHAMAR_MS)
            .filter(|_| !vista);
        let chama = !vista && (!self.olhando_a_espera() || chama_em.is_some_and(|t| agora_ms >= t));
        escalada::Contexto {
            chama,
            chama_em: chama_em.filter(|_| !chama),
            // A tela compartilhada é discreta como o "não perturbe": nada
            // acima da L1 (decisões 0010 e 0091).
            teto_l1: self.nao_perturbe || self.tela.discreto || self.soneca(agora_ms).is_some(),
            visivel: self.na_tela(),
        }
    }

    /// A janela da sessão `chave`, se o Motor tem certeza dela (decisão 0055).
    fn janela_certa(&self, chave: &janelas::Chave) -> Option<&Alca> {
        self.identidades
            .de(chave)
            .filter(|i| i.certeza == janelas::Certeza::Certa)
            .and_then(|i| i.janela.as_ref())
    }

    /// O Renan está olhando o diálogo do aviso que o pet chama: o terminal da
    /// sessão dele em foco, se a janela dela é certa; senão, qualquer terminal
    /// do Claude, como antes (decisão 0090). Olhar o terminal de outra sessão
    /// não é ver o diálogo desta.
    fn olhando_a_espera(&self) -> bool {
        match self
            .chamando
            .as_ref()
            .and_then(|c| self.janela_certa(&c.chave))
        {
            Some(janela) => self.janela_em_foco() == Some(janela),
            None => self.desktop.olhando_claude,
        }
    }

    /// Quando o Renan terá visto o diálogo do aviso que o pet chama: com o
    /// terminal certo da sessão em foco e ele presente, [`ESPERA_VISTA_MS`]
    /// depois do mais tarde entre o aviso, a janela ficar ativa e ele voltar
    /// a mexer (decisão 0090).
    fn prazo_da_espera_vista(&self) -> Option<u64> {
        let c = self.chamando.as_ref().filter(|c| c.vista_ms.is_none())?;
        let janela = self.janela_certa(&c.chave)?;
        if self.janela_em_foco() != Some(janela) || self.desktop.ocioso != Some(false) {
            return None;
        }
        Some(
            cerebro::depois(c.escalada.desde, 0)
                .max(self.ativa_desde)
                .max(self.presente_desde)
                + ESPERA_VISTA_MS,
        )
    }

    /// O Renan viu o diálogo no terminal da sessão: a escalada não passa
    /// mais da L1 (um voo no ar volta para a casa, o pulso desliga no
    /// próximo passo dela), e a base da espera sai depois de
    /// [`tela::ESPERA_VISTA_NA_BASE_MS`] (decisão 0090).
    fn ver_a_espera(&mut self, agora_ms: u64) {
        if !self
            .prazo_da_espera_vista()
            .is_some_and(|prazo| agora_ms >= prazo)
        {
            return;
        }
        let Some(c) = self.chamando.as_mut() else {
            return;
        };
        c.vista_ms = Some(cerebro::instante(agora_ms));
        c.vista_parede = Some(agora_ms + self.deslocamento_parede);
        let (sid8, nivel) = (c.sid8.clone(), c.escalada.nivel);
        info!("o aviso de espera da sessão {sid8} foi visto no terminal dela");
        self.anotar(
            agora_ms,
            intencoes::Tipo::Escalada {
                sid8,
                nivel,
                espera: None,
                motivo: "vista",
            },
        );
        self.voltar_do_voo(agora_ms);
    }

    /// Os avisos de espera depois de o cérebro mudar (ou de um aviso ser
    /// visto, `visto`): cada aviso novo chama (a L1: a chamada e o balão do
    /// tipo, na hora; decisão 0075), um gatilho do mesmo diálogo que sobe o
    /// tipo só troca o balão, e a escalada segue o mais velho. Devolve as
    /// chamadas para o animador.
    fn observar_avisos(&mut self, agora_ms: u64, visto: Option<&janelas::Chave>) -> Vec<Reacao> {
        let esperando: Vec<Pendencia> = self
            .cerebro
            .pendencias()
            .into_iter()
            .filter(|p| p.aviso.tipo == TipoAviso::Esperando)
            .collect();
        let mut reacoes = Vec::new();
        let mut chamados = BTreeMap::new();
        let mut novos = Vec::new();
        // Uma espera da memória velha que já saiu não volta mais.
        self.esperas_velhas.retain(|chave, (desde, _, _)| {
            esperando
                .iter()
                .any(|p| p.chave == *chave && p.aviso.desde_ms == *desde)
        });
        for p in &esperando {
            let antes = self.chamados.get(&p.chave).copied();
            chamados.insert(p.chave.clone(), (p.aviso.desde_ms, p.aviso.espera));
            match antes {
                Some((desde, espera)) if desde == p.aviso.desde_ms => {
                    if p.aviso.espera > espera {
                        if let Some(c) = self.chamando.as_mut().filter(|c| c.chave == p.chave) {
                            c.espera = p.aviso.espera;
                        }
                        let nome = self.rotulo(&p.chave, p.proj.as_deref());
                        let linhas = balao::linhas_da_espera(p.aviso.espera, nome.as_deref());
                        self.balao_decidido(linhas, "aviso_refinado", agora_ms);
                    }
                }
                _ => {
                    novos.push(p.chave.clone());
                    reacoes.extend(self.chamar(p, agora_ms));
                }
            }
        }
        self.chamados = chamados;
        let mais_velho = esperando.first();
        let mesmo = match (&self.chamando, mais_velho) {
            (Some(c), Some(p)) => c.chave == p.chave && c.desde_ms == p.aviso.desde_ms,
            (None, None) => true,
            _ => false,
        };
        if !mesmo {
            if let Some(c) = self.chamando.take() {
                let motivo = if esperando
                    .iter()
                    .any(|p| p.chave == c.chave && p.aviso.desde_ms == c.desde_ms)
                {
                    "outro_aviso"
                } else if visto == Some(&c.chave) {
                    "visto"
                } else if self.cerebro.tem_sessao(&c.chave) {
                    "andou"
                } else {
                    "sessao_saiu"
                };
                self.encerrar_escalada(c, motivo, agora_ms);
            }
            if let Some(p) = mais_velho {
                let novo = novos.contains(&p.chave);
                self.anotar(
                    agora_ms,
                    intencoes::Tipo::Escalada {
                        sid8: p.sid8.clone(),
                        nivel: 1,
                        espera: p.aviso.espera,
                        motivo: if novo { "aviso" } else { "vez" },
                    },
                );
                let velha = self
                    .esperas_velhas
                    .get(&p.chave)
                    .filter(|(desde, _, _)| *desde == p.aviso.desde_ms);
                self.chamando = Some(Chamando {
                    chave: p.chave.clone(),
                    sid8: p.sid8.clone(),
                    proj: p.proj.clone(),
                    espera: p.aviso.espera,
                    desde_ms: p.aviso.desde_ms,
                    escalada: Escalada::nova(p.aviso.desde_mono),
                    vista_ms: velha.map(|(_, laco, _)| *laco),
                    vista_parede: velha.map(|(_, _, parede)| *parede),
                });
            }
        }
        reacoes
    }

    /// A L1 de um aviso novo: a chamada e o balão do tipo dele. Escondido,
    /// nada toca; na soneca, a chamada vira o aceno (decisão 0053).
    fn chamar(&mut self, p: &Pendencia, agora_ms: u64) -> Option<Reacao> {
        if !self.na_tela() {
            return None;
        }
        let nome = if self.soneca(agora_ms).is_some() {
            cerebro::ACENO
        } else {
            CHAMADA
        };
        self.anotar(
            agora_ms,
            intencoes::Tipo::Reacao {
                nome: nome.to_owned(),
                motivo: "aviso",
                sid8: Some(p.sid8.clone()),
                nivel: None,
            },
        );
        let rotulo = self.rotulo(&p.chave, p.proj.as_deref());
        let linhas = balao::linhas_da_espera(p.aviso.espera, rotulo.as_deref());
        self.balao_decidido(linhas, "aviso", agora_ms);
        Some(Reacao {
            nome,
            sid8: p.sid8.clone(),
            proj: p.proj.clone(),
            ts: self.parede(agora_ms),
            nivel: None,
            teste: p.chave.0,
            discreta: false,
        })
    }

    /// A escalada acabou: o pulso desliga e o nível volta a 0.
    fn encerrar_escalada(&mut self, c: Chamando, motivo: &'static str, agora_ms: u64) {
        self.pulso_desde = None;
        self.voltar_do_voo(agora_ms);
        if c.escalada.pulso {
            self.anotar(
                agora_ms,
                intencoes::Tipo::Pulso {
                    ligado: false,
                    sid8: c.sid8.clone(),
                },
            );
        }
        self.anotar(
            agora_ms,
            intencoes::Tipo::Escalada {
                sid8: c.sid8,
                nivel: 0,
                espera: None,
                motivo,
            },
        );
    }

    /// O que a escalada pediu vira intenção; as rajadas também vão para o
    /// animador (a chamada de novo).
    fn aplicar_escalada(
        &mut self,
        passos: Vec<escalada::Passo>,
        motivo: &'static str,
        agora_ms: u64,
    ) -> Vec<Reacao> {
        let Some(c) = self.chamando.as_ref() else {
            return Vec::new();
        };
        let (sid8, proj, teste, nivel) =
            (c.sid8.clone(), c.proj.clone(), c.chave.0, c.escalada.nivel);
        // A L3 que vem com o voo da volta é da volta, mesmo quando ela sai
        // num prazo (a proteção de tela fechou depois; decisão 0090).
        let com_a_volta = passos.contains(&escalada::Passo::Voo { volta: true });
        let mut reacoes = Vec::new();
        for passo in passos {
            let tipo = match passo {
                escalada::Passo::Nivel(n) => intencoes::Tipo::Escalada {
                    sid8: sid8.clone(),
                    nivel: n,
                    espera: None,
                    motivo: if n == 3 && com_a_volta {
                        "voltou"
                    } else {
                        motivo
                    },
                },
                escalada::Passo::Rajada => {
                    reacoes.push(Reacao {
                        nome: CHAMADA,
                        sid8: sid8.clone(),
                        proj: proj.clone(),
                        ts: self.parede(agora_ms),
                        nivel: None,
                        teste,
                        discreta: false,
                    });
                    intencoes::Tipo::Rajada {
                        sid8: sid8.clone(),
                        nivel,
                    }
                }
                escalada::Passo::Voo { volta } => {
                    let motivo = if volta { "voltou" } else { "escalada" };
                    // A volta que a janela não consegue mostrar agora (sem
                    // palco: a camada ainda voltando da proteção de tela) sai
                    // quando ela mostrar (decisão 0090).
                    if !self.comecar_voo(motivo, agora_ms) && volta {
                        self.volta_por_mostrar = Some(agora_ms + VOLTA_POR_MOSTRAR_MS);
                    }
                    intencoes::Tipo::Voo {
                        destino: "alto_centro",
                        motivo,
                        sid8: Some(sid8.clone()),
                    }
                }
                escalada::Passo::Pulso(ligado) => {
                    self.pulso_desde = ligado.then_some(agora_ms);
                    intencoes::Tipo::Pulso {
                        ligado,
                        sid8: sid8.clone(),
                    }
                }
            };
            self.anotar(agora_ms, tipo);
        }
        // O teto (a L4) segura só a pose da espera (decisão 0082), e o selo
        // do aviso pulsa (decisão 0083).
        self.sincronizar_base(agora_ms);
        self.redesenhar_ja(agora_ms);
        reacoes
    }

    /// Os prazos da escalada que venceram.
    fn vencer_escalada(&mut self, agora_ms: u64) -> Vec<Reacao> {
        let ctx = self.contexto_da_escalada(agora_ms);
        let Some(c) = self.chamando.as_mut() else {
            return Vec::new();
        };
        let passos = c.escalada.vencer(agora_ms, ctx);
        self.aplicar_escalada(passos, "tempo", agora_ms)
    }

    /// Quando a escalada muda de novo (no relógio do último instante visto:
    /// um prazo que já passou vence no próximo giro do laço).
    fn prazo_da_escalada(&self) -> Option<u64> {
        let c = self.chamando.as_ref()?;
        let agora = self.relogio_ms;
        c.escalada.proximo(agora, self.contexto_da_escalada(agora))
    }

    /// O nível da escalada do aviso que o pet chama (0 sem nenhum).
    pub fn nivel_da_escalada(&self) -> u8 {
        self.chamando.as_ref().map_or(0, |c| c.escalada.nivel)
    }

    /// Um evento do Claude Code, no relógio da chegada (decisão 0032).
    ///
    /// No prompt do teclado e no começo de uma sessão que o cérebro
    /// acompanha, casa a janela do terminal dela: a que o anel de ativações
    /// diz que estava ativa na hora (`ts`) do hook (decisão 0055). O começo
    /// só preenche, a compactação nunca casa (decisão 0060), e o prompt só
    /// casa se foi digitado: a notificação de uma tarefa e o tique de um laço
    /// não (o cérebro decide, com a evidência daqui), nem um prompt que chega
    /// com o Renan longe do teclado (decisão 0073).
    pub fn evento(&mut self, ev: &Evento, recebido_ms: u64, agora: Agora) -> Vec<Reacao> {
        self.acertar_relogio(agora);
        // O "não perturbe" do Omarchy vem em todo evento (decisão 0075).
        self.nao_perturbe = ev.dnd;
        if self.nao_perturbe {
            // Com o "não perturbe", nenhum voo pela tela (decisão 0075).
            self.voltar_do_voo(agora.mono_ms);
        }
        // Um evento de uma sessão que o cérebro acompanha acorda o pet
        // (decisão 0076).
        let aceito = ev.sid.is_some()
            && ev
                .ent
                .as_deref()
                .is_some_and(|e| self.cerebro.config().origens.iter().any(|o| o == e));
        let mut reacoes = if aceito {
            self.acordar(agora.mono_ms, true)
        } else {
            Vec::new()
        };
        let evidencia = self.evidencia(ev, recebido_ms);
        let do_cerebro = self.cerebro.receber_com(ev, recebido_ms, agora, evidencia);
        reacoes.extend(self.depois_do_cerebro(do_cerebro, agora.mono_ms));
        if let Some(origem) = janelas::origem(&ev.e, ev.src.as_deref())
            && let Some(sid) = &ev.sid
        {
            let chave = (ev.teste, sid.clone());
            // Um prompt que o cérebro ignorou (repetido, de um turno fechado)
            // casa como no M4: o `observar` já descarta o atrasado.
            let digitado = origem != janelas::Origem::Prompt
                || (!evidencia.ausente
                    && self
                        .cerebro
                        .origem_do_turno(&chave, ev.turno.as_deref())
                        .is_none_or(|o| !o.maquina()));
            if self.cerebro.tem_sessao(&chave) && digitado {
                let ts = cerebro::hora_do_evento(ev.ts, recebido_ms);
                let achado = self.desktop.anel.em(ts);
                self.identidades
                    .observar(chave, achado, ev.term.clone(), ts, origem);
            }
        }
        self.esquecer_janelas_sem_sessao();
        reacoes
    }

    /// O que o Motor sabe da hora de um prompt (decisão 0073): o Renan longe
    /// do teclado e do mouse, ou a janela certa da sessão fora de foco na
    /// hora do hook. Só o prompt pede.
    fn evidencia(&self, ev: &Evento, recebido_ms: u64) -> Evidencia {
        if ev.e != "UserPromptSubmit" {
            return Evidencia::default();
        }
        let ts = cerebro::hora_do_evento(ev.ts, recebido_ms);
        let outra_janela = ev.sid.as_ref().is_some_and(|sid| {
            let chave = (ev.teste, sid.clone());
            match (self.identidades.de(&chave), self.desktop.anel.em(ts)) {
                (Some(identidade), janelas::Achado::Janela(ativa)) => {
                    identidade.certeza == janelas::Certeza::Certa
                        && identidade.janela.as_ref().is_some_and(|j| *j != ativa)
                }
                _ => false,
            }
        });
        Evidencia {
            ausente: self.desktop.ocioso == Some(true),
            outra_janela,
        }
    }

    /// O prazo do cérebro venceu (acomodação do Stop, sessões e avisos que
    /// expiram, o pronto visto pelo foco, a escalada).
    pub fn tique(&mut self, agora: Agora) -> Vec<Reacao> {
        self.acertar_relogio(agora);
        let reacoes = self.cerebro.tique(agora);
        let mut reacoes = self.depois_do_cerebro(reacoes, agora.mono_ms);
        self.esquecer_janelas_sem_sessao();
        self.ver_pelo_foco(agora.mono_ms);
        self.ver_a_espera(agora.mono_ms);
        reacoes.extend(self.vencer_escalada(agora.mono_ms));
        reacoes.extend(self.vencer_tela(agora.mono_ms));
        reacoes
    }

    fn esquecer_janelas_sem_sessao(&mut self) {
        let cerebro = &self.cerebro;
        self.identidades.manter(|chave| cerebro.tem_sessao(chave));
    }

    /// Quando chamar [`Self::tique`]: os prazos do cérebro, o do pronto
    /// visto pelo foco, o da escalada e os da tela.
    pub fn prazo_do_cerebro(&self) -> Option<u64> {
        [
            self.cerebro.proximo_prazo(),
            self.prazo_visto_pelo_foco(),
            self.prazo_da_espera_vista(),
            self.prazo_da_escalada(),
            self.prazo_da_tela(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// O resumo do cérebro, com a janela de cada sessão.
    pub fn resumo(&self) -> Resumo {
        let mut resumo = self.cerebro.resumo();
        for sessao in &mut resumo.sessoes {
            sessao.janela = self.identidades.resumo(&sessao.chave);
        }
        resumo
    }

    // --- a memória das sessões (decisão 0093) ---------------------------------

    /// A memória das sessões de agora, para o daemon gravar em `/state`: as
    /// sessões reais do cérebro, com a janela de cada uma (o endereço e a
    /// instância do compositor) e, no aviso de espera que o pet chama, o
    /// nível da escalada e quando o Renan viu o diálogo; o relógio do laço da
    /// gravação e o sossego (o "não perturbe", a soneca e a discrição; decisão
    /// 0095). Só metadados.
    pub fn memoria(&self, agora: Agora, boot: Option<String>) -> Memoria {
        let mut memoria = Memoria::nova(agora.parede_ms, boot);
        memoria.laco_ms = Some(agora.mono_ms);
        let (discricao, sinal) = self.discricao_guardada();
        memoria.sossego = Sossego {
            nao_perturbe: self.nao_perturbe,
            soneca_ate_laco_ms: self.soneca(agora.mono_ms).map(cerebro::instante),
            discricao,
            discricao_sinal_laco_ms: sinal,
        };
        memoria.sessoes = self.cerebro.guardar(agora);
        for g in &mut memoria.sessoes {
            let chave = (false, g.sid.clone());
            g.janela = self.identidades.guardada(&chave);
            if let (Some(aviso), Some(c)) = (g.aviso.as_mut(), self.chamando.as_ref())
                && c.chave == chave
                && c.desde_ms == aviso.desde_ms
            {
                aviso.nivel = Some(c.escalada.nivel);
                aviso.vista_ms = c.vista_parede;
                aviso.vista_laco_ms = c.vista_ms;
            }
        }
        memoria
    }

    /// Restaura a memória das sessões na partida do pet (decisão 0093), em
    /// `agora` (o relógio do laço recomeçou do zero; a parede andou), se ela
    /// é desta partida da máquina (o boot id `boot`): o sossego de antes (o
    /// "não perturbe", a soneca e a discrição que ainda valem; decisão 0095),
    /// as sessões e os avisos que ainda valem ([`Cerebro::restaurar`]) e a
    /// janela de cada uma. Tudo quieto: nenhuma reação, festa, balão nem
    /// chamada de novo; a entrada no erro e o aviso de espera contam como já
    /// vistos, e a escalada do mais velho segue do tempo que passou
    /// ([`Escalada::retomada`]). A memória velha ([`Volta::velha`], decisão
    /// 0095) dá as esperas como vistas na gravação (nada passa da L1) e traz as
    /// janelas sem o endereço: a resposta, o Stop, o `idle_prompt` e uma janela
    /// fechada podem ter se perdido na parada. A tela anuncia a base e os
    /// selos de agora. Nada de turno nem corrente.
    pub fn restaurar(
        &mut self,
        lida: &Lida,
        boot: Option<&str>,
        agora: Agora,
    ) -> Result<Restauracao, Recusa> {
        self.acertar_relogio(agora);
        if let Err(recusa) = lida.memoria.conferir_boot(boot) {
            self.recusar_memoria(recusa, agora.mono_ms);
            return Err(recusa);
        }
        let volta = Volta::de(&lida.memoria, agora);
        let velha = volta.velha();
        // O sossego antes de a escalada seguir: ela olha o teto da L1.
        let sossego = self.restaurar_sossego(&lida.memoria.sossego, volta);
        let r = self.cerebro.restaurar(&lida.memoria.sessoes, volta);
        let guardada = |chave: &janelas::Chave| {
            lida.memoria
                .sessoes
                .iter()
                .find(|g| !chave.0 && g.sid == chave.1)
        };
        let mut janelas = 0;
        for chave in &r.chaves {
            if let Some(j) = guardada(chave).and_then(|g| g.janela.as_ref()) {
                self.identidades.restaurar(chave.clone(), j, velha);
                if self
                    .identidades
                    .de(chave)
                    .is_some_and(|i| i.janela.is_some())
                {
                    janelas += 1;
                }
            }
        }
        // O que a sessão já era não toca de novo: o susto do erro, o bocejo
        // do cansado e a chamada da espera foram antes da partida.
        for s in self.cerebro.resumo_das_sessoes() {
            if !r.chaves.contains(&s.chave) {
                continue;
            }
            self.estados_vistos
                .insert(s.chave.clone(), (s.estado, s.estado_desde_ms));
            if let Some(a) = s.aviso.filter(|a| a.tipo == TipoAviso::Esperando) {
                self.chamados
                    .insert(s.chave.clone(), (a.desde_ms, a.espera));
                if velha {
                    self.esperas_velhas.insert(
                        s.chave.clone(),
                        (a.desde_ms, volta.gravacao(), volta.gravada_ms),
                    );
                }
            }
        }
        // A escalada do aviso de espera mais velho segue do tempo que passou.
        let mais_velho = self
            .cerebro
            .pendencias()
            .into_iter()
            .find(|p| p.aviso.tipo == TipoAviso::Esperando && r.chaves.contains(&p.chave));
        if self.chamando.is_none()
            && let Some(p) = mais_velho
        {
            let antes = guardada(&p.chave).and_then(|g| g.aviso);
            let escalada = Escalada::retomada(
                p.aviso.desde_mono,
                antes.and_then(|a| a.nivel),
                agora.mono_ms,
            );
            // O diálogo que o Renan já tinha visto continua visto; numa
            // memória velha, a espera conta como vista na gravação.
            let vista = antes
                .and_then(|a| a.vista_ms.map(|w| (volta.instante(a.vista_laco_ms, w), w)))
                .or_else(|| {
                    self.esperas_velhas
                        .get(&p.chave)
                        .map(|(_, laco, parede)| (*laco, *parede))
                });
            self.anotar(
                agora.mono_ms,
                intencoes::Tipo::Escalada {
                    sid8: p.sid8.clone(),
                    nivel: escalada.nivel,
                    espera: p.aviso.espera,
                    motivo: "restaurada",
                },
            );
            self.chamando = Some(Chamando {
                chave: p.chave.clone(),
                sid8: p.sid8.clone(),
                proj: p.proj.clone(),
                espera: p.aviso.espera,
                desde_ms: p.aviso.desde_ms,
                escalada,
                vista_ms: vista.map(|(laco, _)| laco),
                vista_parede: vista.map(|(_, parede)| parede),
            });
        }
        let restauracao = Restauracao {
            sessoes: r.chaves.len(),
            avisos: r.avisos,
            janelas,
            de_fora: r.expiradas + r.de_fora + lida.descartadas,
            velha,
        };
        self.anotar(
            agora.mono_ms,
            intencoes::Tipo::Restauracao {
                sessoes: u32::try_from(restauracao.sessoes).unwrap_or(u32::MAX),
                avisos: u32::try_from(restauracao.avisos).unwrap_or(u32::MAX),
                de_fora: u32::try_from(restauracao.de_fora).unwrap_or(u32::MAX),
                velha,
                sossego,
                motivo: None,
            },
        );
        // A base e os selos de agora; nada dorme nem acorda num pet que
        // acabou de nascer.
        let reacoes = self.observar_tela(agora.mono_ms);
        debug_assert!(reacoes.is_empty(), "a restauração tocou {reacoes:?}");
        self.cerebro_mudou = true;
        Ok(restauracao)
    }

    /// O sossego de antes da partida (decisão 0095): o "não perturbe" do
    /// último evento (vale até o próximo, como no pet que não reinicia), a
    /// soneca e a discrição do compartilhamento de tela que ainda valem, pelo
    /// tempo acordado mais a parada. Devolve o que voltou.
    fn restaurar_sossego(&mut self, s: &Sossego, volta: Volta) -> Vec<&'static str> {
        let agora_ms = volta.agora.mono_ms;
        let mut voltou = Vec::new();
        if s.nao_perturbe {
            self.nao_perturbe = true;
            voltou.push("nao_perturbe");
        }
        if let Some(ate) = s.soneca_ate_laco_ms {
            let ate = cerebro::depois(volta.instante(Some(ate), volta.gravada_ms), 0);
            if ate > agora_ms {
                self.soneca_ate = Some(ate);
                voltou.push("soneca");
            }
        }
        if s.discricao {
            // Sem o último sinal, ele ainda estava aceso na gravação.
            let sinal = match s.discricao_sinal_laco_ms {
                Some(x) => volta.instante(Some(x), volta.gravada_ms),
                None => volta.gravacao(),
            };
            if self.restaurar_discricao(sinal, agora_ms) {
                voltou.push("discricao");
            }
        }
        voltou
    }

    /// A memória das sessões não pôde voltar (o arquivo ruim, de outra
    /// versão, outra partida da máquina): fica nas intenções.
    pub fn recusar_memoria(&mut self, recusa: Recusa, agora_ms: u64) {
        self.anotar(
            agora_ms,
            intencoes::Tipo::Restauracao {
                sessoes: 0,
                avisos: 0,
                de_fora: 0,
                velha: false,
                sossego: Vec::new(),
                motivo: Some(recusa.motivo()),
            },
        );
    }

    /// A instância do compositor de agora (no Hyprland, a assinatura que a
    /// descoberta achou; decisão 0093). Noutra instância (um logout e um
    /// login sem reiniciar a máquina), as sessões ficam e as janelas delas
    /// saem: o mesmo endereço não é mais a mesma janela, e o próximo prompt
    /// digitado casa de novo. O anel e a janela ativa de antes também eram da
    /// outra. Devolve quantas janelas saíram.
    pub fn definir_compositor(&mut self, instancia: Option<String>) -> usize {
        let antes = self.identidades.compositor().map(str::to_owned);
        if let (Some(antes), Some(nova)) = (&antes, &instancia)
            && antes != nova
        {
            self.desktop.anel = janelas::Anel::default();
            self.desktop.janela_ativa = None;
        }
        let sairam = self.identidades.definir_compositor(instancia);
        if sairam > 0 {
            info!(
                "outra instância do compositor: {sairam} janela(s) de sessão ficaram sem endereço"
            );
            self.cerebro_mudou = true;
        }
        sairam
    }

    // --- personagem e janela -----------------------------------------------

    pub fn skin(&self) -> Option<&Rc<Skin>> {
        self.skin.as_ref()
    }

    /// Troca o personagem sem janela aberta (vale na próxima sessão).
    pub fn definir_skin(&mut self, skin: Option<Rc<Skin>>) {
        self.skin = skin;
    }

    pub fn tamanho(&self) -> Tamanho {
        self.tamanho
    }

    /// Troca o tamanho do pet (o config relido, decisão 0042). Com a janela
    /// pronta, o palco é refeito e o quadro novo redesenha a tela toda; sem,
    /// vale no próximo palco. A skin não muda.
    pub fn definir_tamanho(
        &mut self,
        tamanho: Tamanho,
        ov: Option<&mut dyn Overlay>,
        agora_ms: u64,
    ) {
        if tamanho == self.tamanho {
            return;
        }
        self.tamanho = tamanho;
        if let Some(ov) = ov {
            self.redesenhar_palco(ov, agora_ms);
        }
    }

    /// Refaz o palco com o tamanho de agora e redesenha a tela toda já, se o
    /// pet está num palco: o tamanho mudou e a skin não (decisão 0046).
    pub fn redesenhar_palco(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        if self.palco.is_some() {
            ov.esquecer_cena();
            self.refazer_palco(ov, agora_ms);
        }
    }

    /// O pet deve estar na tela (pedido do `/v1/comando`).
    pub fn visivel(&self) -> bool {
        self.visivel
    }

    pub fn definir_visivel(&mut self, visivel: bool) {
        self.visivel = visivel;
    }

    /// O pet deve aparecer agora: há personagem, ninguém mandou esconder e
    /// a proteção de tela não está na tela (decisão 0053).
    pub fn quer_mostrar(&self) -> bool {
        self.visivel && self.pet.is_some() && !self.desktop.protetor_ativo()
    }

    /// Uma janela nova (o compositor conectou): o pet recomeça a animação
    /// agora, sem palco até a janela ficar pronta. A camada nova nasce no
    /// monitor em foco: o alvo de antes não vale mais (decisão 0059).
    pub fn conectou(&mut self, agora_ms: u64) {
        self.pet = self.skin.clone().map(|skin| self.pet_novo(skin, agora_ms));
        self.palco = None;
        self.arraste.cancelar();
        self.seguir.esquecer();
        self.estresse = None;
        self.commits = Commits::default();
        self.proximo_quadro = None;
    }

    /// A janela acabou com a sessão (o compositor caiu). Um foco pedido pelo
    /// clique morre com ela (o handle era dela), e nada que só existe na
    /// janela (o balão, o coração, a viagem e o prazo de seguir o foco) fica
    /// com prazo armado: sem janela, ninguém o venceria (decisão 0059). A
    /// soneca fica, e acaba no prazo dela ([`Self::vencer_sem_conexao`]).
    pub fn desconectou(&mut self) {
        self.pet = None;
        self.palco = None;
        self.voo = None;
        self.efeito = None;
        self.focando.clear();
        self.coracao_ate = None;
        // Quem contava se o Renan está era a conexão (decisão 0062).
        self.desktop.ocioso = None;
        self.ausente_desde = None;
        self.balao = None;
        self.arraste.cancelar();
        self.seguir.esquecer();
        self.estresse = None;
        self.commits = Commits::default();
        self.proximo_quadro = None;
    }

    /// Pede a janela nova (no Wayland, uma camada com output NULL: ela cai
    /// no monitor em foco) e larga o palco até ela ficar pronta.
    fn criar_janela(&mut self, ov: &mut dyn Overlay) {
        ov.criar();
        self.palco = None;
        self.seguir.criou_camada();
    }

    /// A janela ficou pronta em `monitor`. Sem foco novo desde que ela foi
    /// pedida, esse é o monitor em foco: ele corrige um alvo velho, que
    /// mandaria o pet viajar sem fim, e o `/v1/estado` passa a saber o
    /// monitor em foco antes da primeira troca (decisão 0059). `true` se a
    /// janela está no monitor em foco (nada a conferir).
    fn pousou(&mut self, monitor: &Monitor) -> bool {
        let Some(nome) = monitor.nome.as_deref() else {
            return false;
        };
        let antes = self.seguir.alvo.clone();
        let Some(em_foco) = self.seguir.pousou(nome) else {
            return false;
        };
        if antes.as_deref().is_some_and(|a| a != em_foco) {
            info!(
                "seguir: o foco contado era {}, mas a camada nova caiu em {em_foco} sem foco novo \
                 depois: o pet fica em {em_foco}",
                antes.as_deref().unwrap_or("?")
            );
        }
        self.desktop.monitor_em_foco = Some(em_foco);
        true
    }

    /// Leva a janela à visibilidade pedida. Numa viagem para outro monitor,
    /// quem cria a janela nova é a viagem; esconder no meio desiste dela.
    pub fn aplicar_visibilidade(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        match self.seguir.fase() {
            Some(FaseViagem::Entrando { .. }) | None => {}
            Some(_) if self.quer_mostrar() => return,
            Some(_) => {
                self.seguir.terminar();
            }
        }
        let passo = passo_de_visibilidade(ov.fase(), self.quer_mostrar());
        self.aplicar_passo(ov, passo, agora_ms);
    }

    fn aplicar_passo(&mut self, ov: &mut dyn Overlay, passo: Passo, agora_ms: u64) {
        match passo {
            Passo::Nada => {}
            Passo::Criar => self.criar_janela(ov),
            Passo::Cancelar => {
                ov.cancelar_saida();
                // Como esconder, mostrar é uma ordem e não animação: o pet
                // volta já, mesmo com um quadro em voo (que, com a tela
                // apagada, só seria mostrado quando ela acendesse).
                self.desenhar(ov, agora_ms, true);
            }
            Passo::ApagarEDestruir => {
                self.cancelar_voo(true, agora_ms);
                self.efeito = None;
                self.largar_o_arraste(agora_ms);
                self.proximo_quadro = None;
                self.estresse = None;
                let commit = match &self.pet {
                    Some(pet) => ov.apagar_e_destruir(pet.skin()),
                    None => {
                        ov.destruir();
                        false
                    }
                };
                if commit {
                    self.commits.contar(agora_ms);
                }
            }
            Passo::Destruir => {
                self.cancelar_voo(true, agora_ms);
                self.efeito = None;
                self.largar_o_arraste(agora_ms);
                self.proximo_quadro = None;
                self.estresse = None;
                ov.destruir();
            }
        }
    }

    /// Troca o personagem com a janela aberta (aprovação ou revogação,
    /// decisão 0026). Sem skin, o pet se esconde com o quadro transparente
    /// (desenhado com a skin velha). Com skin nova, a cena velha é esquecida
    /// e o palco (D e posição, que dependem da skin) é refeito: na hora, se a
    /// janela está pronta, ou quando ela ficar. O quadro novo redesenha a tela
    /// toda, sem fantasma da skin velha.
    pub fn trocar_skin(&mut self, ov: &mut dyn Overlay, skin: Option<Rc<Skin>>, agora_ms: u64) {
        match skin {
            None => {
                let passo = passo_de_visibilidade(ov.fase(), false);
                self.aplicar_passo(ov, passo, agora_ms);
                self.pet = None;
                self.skin = None;
            }
            Some(skin) => {
                self.proximo_quadro = None;
                self.estresse = None;
                self.skin = Some(Rc::clone(&skin));
                self.pet = Some(self.pet_novo(skin, agora_ms));
                self.palco = None;
                ov.esquecer_cena();
                self.aplicar_visibilidade(ov, agora_ms);
                self.refazer_palco(ov, agora_ms);
            }
        }
    }

    /// Palco do pet num monitor pronto, na posição salva daquele monitor ou
    /// no canto inferior direito da área útil, com o log de onde ele ficou.
    fn montar_palco(&mut self, monitor: &Monitor) {
        let Some(pet) = &self.pet else {
            return;
        };
        let mut palco = pet.palco(monitor, self.tamanho);
        self.chave_do_monitor = posicoes::chave(monitor);
        if let Some(fracao) = self
            .chave_do_monitor
            .as_deref()
            .and_then(|c| self.posicoes.de(c))
        {
            let (x, y) = fracao.celula(palco.d, &pet.skin().ancoras, palco.tela);
            (palco.x, palco.y) = pet.prender(&palco, x, y);
        }
        info!(
            "pet «{}» com D={} (tamanho {}) e célula em ({}, {}) pixels do monitor",
            pet.skin().id,
            palco.d,
            self.tamanho.nome(),
            palco.x,
            palco.y
        );
        self.palco = Some(palco);
    }

    /// Refaz o palco numa janela que já está pronta (ela só avisa quando o
    /// monitor ou a escala mudam, não quando a skin muda) e desenha já, mesmo
    /// com um quadro em voo. Janela ainda não pronta ou saindo: nada; o palco
    /// vem com [`EventoOverlay::Pronta`].
    fn refazer_palco(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        if ov.fase() == Fase::Saindo {
            return;
        }
        let Some(monitor) = ov.pronta() else {
            return;
        };
        self.cancelar_voo(false, agora_ms);
        self.efeito = None;
        self.montar_palco(&monitor);
        self.desenhar(ov, agora_ms, true);
    }

    /// Um evento da janela. Recebe a conexão inteira ([`Punho`]): o clique
    /// do M4 foca janelas pelo desktop.
    pub fn evento_overlay(&mut self, punho: &mut dyn Punho, evento: EventoOverlay, agora_ms: u64) {
        let ov = punho.janela();
        match evento {
            // O palco sai do monitor de agora, não de quando o evento entrou
            // na fila: dois na mesma leva (escala e `configure` juntos) não
            // desenham com um palco velho no buffer novo.
            EventoOverlay::Pronta => {
                if self.pet.is_some()
                    && let Some(monitor) = ov.pronta()
                {
                    // Um palco novo (outro monitor, outra escala): o voo
                    // acaba, e o pet volta para a posição salva; o confete,
                    // que anda na grade do palco velho, sai.
                    self.cancelar_voo(false, agora_ms);
                    self.efeito = None;
                    if self.seguir.fase() == Some(FaseViagem::Chegando) {
                        self.chegar(ov, &monitor, agora_ms);
                    } else {
                        self.montar_palco(&monitor);
                        let no_foco = self.pousou(&monitor);
                        self.desenhar(ov, agora_ms, false);
                        // Um foco que chegou depois de a janela ser pedida
                        // pode estar noutro monitor.
                        if !no_foco {
                            self.seguir.conferir_em(agora_ms);
                        }
                    }
                    // A volta do Renan que esperava o pet aparecer (a proteção
                    // de tela fechou e a camada voltou; decisão 0090).
                    self.mostrar_a_volta(agora_ms);
                }
            }
            EventoOverlay::Redesenhar => {
                // O quadro em voo foi mostrado. Um voo que acabou pelo relógio
                // enquanto ele estava preso (a sessão bloqueada) termina aqui;
                // se era a volta do Renan, ninguém a viu, e ela sai agora.
                self.andar_voo(agora_ms);
                self.mostrar_a_volta(agora_ms);
                self.desenhar(ov, agora_ms, false);
            }
            EventoOverlay::Sumiu => {
                // A janela fechou no meio de um voo ou de um confete: os dois
                // acabam (o pet larga o voo na casa), como no esconder.
                self.cancelar_voo(true, agora_ms);
                self.efeito = None;
                self.largar_o_arraste(agora_ms);
                self.proximo_quadro = None;
                self.palco = None;
                // O compositor fechou a janela no meio da viagem (o monitor
                // saiu): a nova vem com o `Recriar`.
                if self.seguir.em_viagem() {
                    self.seguir.mudar_fase(FaseViagem::Chegando);
                }
            }
            EventoOverlay::Recriar => {
                if self.quer_mostrar() && ov.fase() == Fase::Ausente {
                    self.criar_janela(ov);
                }
            }
            EventoOverlay::Ponteiro(ponteiro) => self.ponteiro(punho, ponteiro, agora_ms),
            // A janela velha morreu no meio da viagem: a nova nasce no
            // monitor em foco (output NULL).
            EventoOverlay::Saiu => {
                if self.seguir.fase() == Some(FaseViagem::Saindo) {
                    if self.quer_mostrar() && ov.fase() == Fase::Ausente {
                        self.criar_janela(ov);
                        self.seguir.mudar_fase(FaseViagem::Chegando);
                    } else {
                        self.seguir.terminar();
                    }
                }
            }
        }
    }

    // --- seguir o monitor ativo (decisão 0051) -------------------------------

    /// Começa a viagem para o monitor em foco: o poof de saída (se o pet está
    /// desenhado e a viagem não é rápida) ou a saída direta.
    fn comecar_viagem(&mut self, ov: &mut dyn Overlay, pouso: Option<Pouso>, agora_ms: u64) {
        self.cancelar_voo(true, agora_ms);
        self.efeito = None;
        self.largar_o_arraste(agora_ms);
        self.balao = None;
        self.coracao_ate = None;
        let desenhado = ov.fase() == (Fase::Viva { conteudo: true }) && self.palco.is_some();
        let rapida = self.seguir.comecar(agora_ms, pouso, desenhado);
        info!(
            "indo para o monitor em foco ({}){}",
            self.seguir.alvo.as_deref().unwrap_or("?"),
            if rapida { ", sem poof" } else { "" }
        );
        if rapida {
            self.sair_para_viajar(ov, agora_ms);
        } else {
            self.desenhar(ov, agora_ms, true);
        }
    }

    /// O poof de saída acabou (ou não houve): quadro transparente e a janela
    /// morre; sem nada desenhado, morre já e a nova nasce em seguida.
    fn sair_para_viajar(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        self.seguir.mudar_fase(FaseViagem::Saindo);
        let passo = passo_de_visibilidade(ov.fase(), false);
        self.aplicar_passo(ov, passo, agora_ms);
        if ov.fase() == Fase::Ausente {
            if self.quer_mostrar() {
                self.criar_janela(ov);
                self.seguir.mudar_fase(FaseViagem::Chegando);
            } else {
                self.seguir.terminar();
            }
        }
    }

    /// A janela nova ficou pronta: o palco na posição salva daquele monitor
    /// (ou no ponto onde o pet foi solto) e o poof de chegada. Se ela caiu
    /// noutro monitor que não o alvo e nenhum foco novo chegou, o alvo era
    /// velho: o pet fica onde caiu ([`Self::pousou`]).
    fn chegar(&mut self, ov: &mut dyn Overlay, monitor: &Monitor, agora_ms: u64) {
        self.montar_palco(monitor);
        self.pousou(monitor);
        let viagem = self.seguir.viagem;
        if let Some(pouso) = viagem.and_then(|v| v.pouso)
            && let Some(origem) = monitor.origem
        {
            let e = monitor.escala;
            let x = ((pouso.global.0 - pouso.pegada.0 - origem.0 as f64) * e).round() as i32;
            let y = ((pouso.global.1 - pouso.pegada.1 - origem.1 as f64) * e).round() as i32;
            self.mover_para((x, y));
            if let Some(pet) = self.pet.as_mut() {
                pet.tocar(SOLTO, agora_ms);
            }
            self.guardar_posicao();
        }
        if viagem.is_none_or(|v| v.rapida) {
            self.seguir.terminar();
            self.seguir.conferir_em(agora_ms);
        } else {
            self.seguir.mudar_fase(FaseViagem::Entrando {
                inicio_ms: agora_ms,
            });
        }
        self.desenhar(ov, agora_ms, false);
    }

    /// Os prazos da viagem: o fim dos poofs e a hora de conferir o foco.
    fn vencer_viagem(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        match self.seguir.fase() {
            Some(FaseViagem::Poof { inicio_ms }) if agora_ms >= inicio_ms + poof::DURACAO_MS => {
                self.sair_para_viajar(ov, agora_ms);
            }
            Some(FaseViagem::Entrando { inicio_ms })
                if agora_ms >= inicio_ms + poof::DURACAO_MS =>
            {
                self.seguir.terminar();
                self.desenhar(ov, agora_ms, false);
                self.seguir.conferir_em(agora_ms);
            }
            _ => {}
        }
        let monitor = ov.pronta().and_then(|m| m.nome);
        let livre = self.quer_mostrar()
            && !self.arraste.segurando()
            && matches!(ov.fase(), Fase::Viva { .. });
        if self.seguir.decidir(agora_ms, monitor.as_deref(), livre) == Decisao::Viajar {
            self.comecar_viagem(ov, None, agora_ms);
        }
    }

    /// O pouso no monitor onde o botão subiu, se ele subiu fora deste: o
    /// ponto no desktop (lógico) e a pegada, pela origem do monitor.
    fn pouso_fora(&self, ov: &dyn Overlay, x: i32, y: i32, pegada: (i32, i32)) -> Option<Pouso> {
        let palco = self.palco?;
        let fora = x < 0 || y < 0 || x >= palco.tela.0 || y >= palco.tela.1;
        if !fora {
            return None;
        }
        let monitor = ov.pronta()?;
        let origem = monitor.origem?;
        let e = monitor.escala;
        Some(Pouso {
            global: (
                origem.0 as f64 + x as f64 / e,
                origem.1 as f64 + y as f64 / e,
            ),
            pegada: (pegada.0 as f64 / e, pegada.1 as f64 / e),
        })
    }

    // --- arrastar e clicar (decisão 0048) ------------------------------------

    /// Um evento do ponteiro, no palco.
    fn ponteiro(&mut self, punho: &mut dyn Punho, evento: EventoPonteiro, agora_ms: u64) {
        // Um aperto no pet é o Renan aqui, mesmo se o desktop ainda não
        // contou que ele voltou (o `resumed` pode vir na mesma leva, depois).
        if matches!(evento, EventoPonteiro::Apertou { .. }) && self.desktop.ocioso == Some(true) {
            self.desktop.ocioso = Some(false);
            self.presente_desde = agora_ms;
            // Quem aperta o pet já está olhando para ele: nenhum voo de volta.
            self.ausente_desde = None;
        }
        let (Some(palco), true) = (self.palco, self.pet.is_some()) else {
            self.arraste.cancelar();
            return;
        };
        let no_corpo = match evento {
            EventoPonteiro::Apertou { x, y, .. } => self.acerta_o_pet(x, y),
            _ => false,
        };
        let limiar = (arraste::LIMIAR_LOGICO * palco.escala).round() as i32;
        let gesto = self.arraste.ponteiro(
            evento,
            agora_ms,
            (palco.x, palco.y),
            no_corpo,
            limiar,
            palco.d,
        );
        self.aplicar_gesto(punho, gesto, agora_ms);
    }

    fn aplicar_gesto(&mut self, punho: &mut dyn Punho, gesto: Gesto, agora_ms: u64) {
        match gesto {
            Gesto::Nada => {}
            Gesto::Apertou => punho.janela().cursor(Cursor::Agarrar),
            Gesto::Comecou => {
                self.balao = None;
                self.coracao_ate = None;
                // Pegou o pet no meio do voo: o voo acaba onde ele está, e o
                // arraste segue dali (decisão 0084).
                self.cancelar_voo(false, agora_ms);
                if let Some(pet) = self.pet.as_mut() {
                    pet.segurar(ARRASTADO, agora_ms);
                }
                self.ir_para_o_alvo();
                self.desenhar(punho.janela(), agora_ms, false);
            }
            Gesto::Moveu => {
                if self.ir_para_o_alvo() {
                    self.desenhar(punho.janela(), agora_ms, false);
                }
            }
            Gesto::Soltou {
                x,
                y,
                celula,
                pegada,
            } => {
                self.mover_para(celula);
                let ov = punho.janela();
                match self.pouso_fora(ov, x, y, pegada) {
                    // Solto fora do monitor: vai já para o monitor debaixo do
                    // ponteiro (o compositor já o focou), e pousa lá.
                    Some(pouso) => {
                        ov.cursor(Cursor::Pegar);
                        if let Some(pet) = self.pet.as_mut() {
                            pet.largar(agora_ms);
                        }
                        self.comecar_viagem(ov, Some(pouso), agora_ms);
                    }
                    None => self.pousar(ov, agora_ms),
                }
            }
            Gesto::Cancelou => self.pousar(punho.janela(), agora_ms),
            Gesto::Desistiu => punho.janela().cursor(Cursor::Pegar),
            Gesto::Clique(botao) => {
                punho.janela().cursor(Cursor::Pegar);
                self.clicar(punho, botao, agora_ms);
            }
        }
    }

    /// Leva a célula ao alvo do arraste (preso na área útil). `true` se ela
    /// mudou de lugar.
    fn ir_para_o_alvo(&mut self) -> bool {
        let Some(d) = self.palco.map(|p| p.d) else {
            return false;
        };
        match self.arraste.alvo(d) {
            Some(alvo) => self.mover_para(alvo),
            None => false,
        }
    }

    /// Põe a célula em `celula` (presa na área útil). `true` se mudou.
    fn mover_para(&mut self, celula: (i32, i32)) -> bool {
        let (Some(palco), Some(pet)) = (self.palco.as_mut(), self.pet.as_ref()) else {
            return false;
        };
        let (x, y) = pet.prender(palco, celula.0, celula.1);
        let mudou = (x, y) != (palco.x, palco.y);
        palco.x = x;
        palco.y = y;
        mudou
    }

    /// O arraste acabou (soltou ou o fail-safe): o cursor volta, o pet larga
    /// o laço, pousa, a área de toque volta ao corpo e a posição fica
    /// guardada para aquele monitor (decisão 0049).
    fn pousar(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        ov.cursor(Cursor::Pegar);
        if let Some(pet) = self.pet.as_mut() {
            pet.largar(agora_ms);
            pet.tocar(SOLTO, agora_ms);
        }
        self.guardar_posicao();
        self.desenhar(ov, agora_ms, false);
        // O foco pode ter mudado de monitor durante o arraste.
        self.seguir.conferir_em(agora_ms);
    }

    /// Esconder, a janela fechada ou o fim da conexão largam o arraste sem
    /// desenhar (a área de toque volta ao corpo no próximo quadro).
    fn largar_o_arraste(&mut self, agora_ms: u64) {
        if self.arraste.cancelar()
            && let Some(pet) = self.pet.as_mut()
        {
            pet.largar(agora_ms);
        }
    }

    // --- avisos e o clique (decisão 0057) ------------------------------------

    /// Um clique no pet. O esquerdo vai ao aviso da vez (o mais urgente que
    /// esta volta do ciclo ainda não visitou): foca o terminal da sessão
    /// dele, com a risadinha e o coração, e o aviso sai quando o desktop
    /// conta que a janela ficou ativa. Sem como focar, o balão diz o porquê e
    /// mostra as sessões; sem aviso, só as sessões. O direito, a soneca
    /// (decisão 0053).
    pub fn clicar(&mut self, punho: &mut dyn Punho, botao: Botao, agora_ms: u64) -> Clicou {
        // O clique acorda o pet (a risadinha ou a soneca tocam por cima).
        self.acordar(agora_ms, false);
        let clicou = match botao {
            Botao::Esquerdo => self.clique_esquerdo(punho, agora_ms),
            Botao::Direito => {
                self.alternar_soneca(punho.janela(), agora_ms);
                Clicou::Soneca {
                    cochilando: self.soneca(agora_ms).is_some(),
                }
            }
            _ => Clicou::Nada {
                motivo: "o pet só atende o botão esquerdo e o direito",
            },
        };
        let (resultado, sid8) = match &clicou {
            Clicou::Focou { sid8, .. } => ("focou", Some(sid8.clone())),
            Clicou::NaoFocou { sid8, .. } => ("nao_focou", Some(sid8.clone())),
            Clicou::Lista { .. } => ("lista", None),
            Clicou::Soneca { .. } => ("soneca", None),
            Clicou::Nada { .. } => ("nada", None),
        };
        self.anotar(agora_ms, intencoes::Tipo::Clique { resultado, sid8 });
        self.observar_tela(agora_ms);
        clicou
    }

    fn clique_esquerdo(&mut self, punho: &mut dyn Punho, agora_ms: u64) -> Clicou {
        // Um clique muito depois do anterior começa outra volta: o primeiro
        // vai sempre ao mais urgente, mesmo que um clique de horas atrás o
        // tenha visitado sem conseguir focar.
        if self
            .ultimo_clique_ms
            .is_none_or(|t| agora_ms.saturating_sub(t) > VOLTA_DO_CICLO_MS)
        {
            self.ciclo.clear();
        }
        self.ultimo_clique_ms = Some(agora_ms);
        let pendencias = self.cerebro.pendencias();
        self.ciclo
            .retain(|chave| pendencias.iter().any(|p| &p.chave == chave));
        let da_vez = match pendencias.iter().find(|p| !self.ciclo.contains(&p.chave)) {
            Some(p) => Some(p.clone()),
            // Todas já visitadas nesta volta: começa outra.
            None => {
                self.ciclo.clear();
                pendencias.first().cloned()
            }
        };
        let Some(alvo) = da_vez else {
            self.anotar_risadinha(agora_ms);
            self.tocar(Some(punho.janela()), RISADINHA, agora_ms);
            let linhas = self.linhas_das_sessoes(agora_ms);
            let sessoes = self.cerebro.resumo().sessoes.len();
            self.mostrar_balao_por(Some(punho.janela()), linhas, "lista", agora_ms);
            return Clicou::Lista { sessoes };
        };
        self.ciclo.push(alvo.chave.clone());
        let tipo = alvo.aviso.tipo;
        let motivo = match self.janela_da_sessao(&alvo.chave).cloned() {
            None => self
                .identidades
                .de(&alvo.chave)
                .map_or(janelas::Certeza::SemAnel, |i| i.certeza)
                .motivo(),
            Some(janela) => match punho.desktop().focar(&janela) {
                Ok(()) => return self.focou(punho, alvo, janela, agora_ms),
                Err(ErroFoco::NaoSuportado) => "aqui eu não sei focar janelas",
                Err(ErroFoco::JanelaSumiu) => "a janela dela sumiu",
                Err(ErroFoco::Recusado(_)) => "o sistema recusou o foco",
            },
        };
        info!(
            "clique: o {} da sessão {} ficou sem foco: {motivo}",
            tipo.nome(),
            alvo.sid8
        );
        self.balao_sem_foco(
            punho.janela(),
            &alvo.chave,
            alvo.proj.as_deref(),
            tipo,
            motivo,
            agora_ms,
        );
        Clicou::NaoFocou {
            sid8: alvo.sid8,
            aviso: tipo,
            motivo,
        }
    }

    /// O desktop aceitou focar a janela do aviso: a risadinha com o coração.
    /// Se ele conta as trocas de janela e ela ainda não é a ativa, o aviso só
    /// sai quando ele contar que ela ficou (o Hyprland recusa o foco com a
    /// sessão bloqueada sem dizer nada); senão, sai já.
    fn focou(
        &mut self,
        punho: &mut dyn Punho,
        alvo: Pendencia,
        janela: Alca,
        agora_ms: u64,
    ) -> Clicou {
        // Com o Renan longe (a sessão bloqueada, um clique de script), a
        // janela "ativa" pode ser só a última que teve o foco: não conta
        // como vista na hora (decisão 0062).
        let ja_ativa = self.desktop.janela_ativa.as_ref() == Some(&janela)
            && self.desktop.ocioso != Some(true);
        let conta_trocas =
            self.desktop.ligado == Some(true) || punho.ver_desktop().capacidades().janela_ativa;
        let esperar = conta_trocas && !ja_ativa;
        info!(
            "clique: focando a janela {} da sessão {} ({})",
            janela.0,
            alvo.sid8,
            alvo.aviso.tipo.nome()
        );
        if esperar {
            // Cliques seguidos: cada foco espera a confirmação dele.
            self.focando.retain(|f| f.chave != alvo.chave);
            if self.focando.len() == FOCANDO_MAX {
                self.focando.remove(0);
            }
            self.focando.push(Focando {
                chave: alvo.chave.clone(),
                janela: janela.clone(),
                proj: alvo.proj.clone(),
                tipo: alvo.aviso.tipo,
                ate_ms: agora_ms + CONFIRMAR_FOCO_MS,
            });
        } else {
            self.ver(&alvo.chave, agora_ms);
        }
        self.balao = None;
        self.coracao_ate = Some(agora_ms + CORACAO_MS);
        self.anotar_risadinha(agora_ms);
        if !self.tocar(Some(punho.janela()), RISADINHA, agora_ms) {
            self.desenhar(punho.janela(), agora_ms, false);
        }
        Clicou::Focou {
            sid8: alvo.sid8,
            aviso: alvo.aviso.tipo,
            janela: janela.0,
            confirmado: !esperar,
        }
    }

    fn anotar_risadinha(&mut self, agora_ms: u64) {
        self.anotar(
            agora_ms,
            intencoes::Tipo::Reacao {
                nome: RISADINHA.to_owned(),
                motivo: "clique",
                sid8: None,
                nivel: None,
            },
        );
    }

    /// O balão de um aviso sem foco: a sessão, o porquê e as sessões.
    fn balao_sem_foco(
        &mut self,
        ov: &mut dyn Overlay,
        chave: &janelas::Chave,
        proj: Option<&str>,
        tipo: TipoAviso,
        motivo: &str,
        agora_ms: u64,
    ) {
        let nome = self.rotulo(chave, proj);
        let mut linhas = balao::linhas_sem_foco(nome.as_deref(), tipo, motivo);
        linhas.extend(self.linhas_das_sessoes(agora_ms));
        self.mostrar_balao_por(Some(ov), linhas, "sem_foco", agora_ms);
    }

    /// O Renan viu o aviso da sessão: ele sai (e a escalada dele, se era o
    /// que o pet chamava), e o núcleo publica o cérebro.
    fn ver(&mut self, chave: &janelas::Chave, agora_ms: u64) -> Option<TipoAviso> {
        let tipo = self.cerebro.ver(chave);
        if tipo.is_some() {
            self.cerebro_mudou = true;
            self.observar_avisos(agora_ms, Some(chave));
            self.observar_tela(agora_ms);
        }
        self.ciclo.retain(|c| c != chave);
        tipo
    }

    /// Um aviso saiu fora de um evento do Claude ou de um tique desde a
    /// última pergunta: o núcleo publica o cérebro de novo.
    pub fn tirar_mudanca_do_cerebro(&mut self) -> bool {
        std::mem::take(&mut self.cerebro_mudou)
    }

    /// A janela ativa, se a fonte das trocas está ligada (com ela fora, a
    /// de antes pode não ser mais a ativa).
    fn janela_em_foco(&self) -> Option<&Alca> {
        if self.desktop.ligado != Some(true) {
            return None;
        }
        self.desktop.janela_ativa.as_ref()
    }

    fn janela_da_sessao(&self, chave: &janelas::Chave) -> Option<&Alca> {
        self.identidades.de(chave).and_then(|i| i.janela.as_ref())
    }

    /// As sessões com o pronto ou o erro pendente cujo terminal está em foco
    /// e desde quando contam os [`VISTO_PELO_FOCO_MS`]: o mais cedo.
    fn vistas_pelo_foco(&self) -> Vec<(Pendencia, u64)> {
        let Some(ativa) = self.janela_em_foco() else {
            return Vec::new();
        };
        // O terminal em foco só diz que o Renan viu com ele mexendo no
        // teclado ou no mouse: bloqueado, com a tela apagada ou longe, a
        // janela que teve o foco por último continua "ativa" (decisão 0062).
        // Sem saber se ele está, também não.
        if self.desktop.ocioso != Some(false) {
            return Vec::new();
        }
        self.cerebro
            .pendencias()
            .into_iter()
            .filter(|p| p.aviso.tipo != TipoAviso::Esperando)
            .filter(|p| self.janela_da_sessao(&p.chave) == Some(ativa))
            .map(|p| {
                let prazo = cerebro::depois(p.aviso.desde_mono, 0)
                    .max(self.ativa_desde)
                    .max(self.presente_desde)
                    + VISTO_PELO_FOCO_MS;
                (p, prazo)
            })
            .collect()
    }

    fn prazo_visto_pelo_foco(&self) -> Option<u64> {
        self.vistas_pelo_foco().into_iter().map(|(_, p)| p).min()
    }

    /// O pronto e o erro das sessões cujo terminal está em foco há
    /// [`VISTO_PELO_FOCO_MS`] saem: o Renan já viu. O "esperando você" fica
    /// (só um evento da sessão ou o clique o tiram).
    fn ver_pelo_foco(&mut self, agora_ms: u64) {
        for (p, prazo) in self.vistas_pelo_foco() {
            if agora_ms >= prazo {
                self.ver(&p.chave, agora_ms);
                info!(
                    "o {} da sessão {} saiu: o terminal dela ficou em foco",
                    p.aviso.tipo.nome(),
                    p.sid8
                );
            }
        }
    }

    // --- a soneca (decisão 0053) --------------------------------------------

    /// O botão direito: começa a soneca de 30 min (o bocejo e o selo "zZ")
    /// ou, se o pet já cochila, acorda (o despertar).
    pub fn alternar_soneca(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        let nome = if self.soneca(agora_ms).is_some() {
            self.soneca_ate = None;
            info!("soneca: acordou");
            DESPERTAR
        } else {
            self.soneca_ate = Some(agora_ms + SONECA_MS);
            info!("soneca de {} min", SONECA_MS / 60_000);
            // Na soneca, nenhum voo (decisão 0075).
            self.voltar_do_voo(agora_ms);
            BOCEJO
        };
        self.anotar(
            agora_ms,
            intencoes::Tipo::Reacao {
                nome: nome.to_owned(),
                motivo: "soneca",
                sid8: None,
                nivel: None,
            },
        );
        self.tocar(Some(&mut *ov), nome, agora_ms);
        self.desenhar(ov, agora_ms, false);
    }

    /// Até quando o pet cochila, se cochila.
    pub fn soneca(&self, agora_ms: u64) -> Option<u64> {
        self.soneca_ate.filter(|&ate| agora_ms < ate)
    }

    /// Uma reação do cérebro: na soneca, só as pequenas (o pulinho e o que
    /// for maior viram o aceno discreto).
    pub fn reagir(&mut self, ov: Option<&mut dyn Overlay>, reacao: &str, agora_ms: u64) -> bool {
        let reacao = if self.soneca(agora_ms).is_some() && reacao != crate::cerebro::TCHAU {
            crate::cerebro::ACENO
        } else {
            reacao
        };
        self.tocar(ov, reacao, agora_ms)
    }

    /// O pet está sendo arrastado.
    pub fn arrastando(&self) -> bool {
        self.arraste.arrastando()
    }

    // --- desktop -------------------------------------------------------------

    /// Um evento do desktop (decisão 0043): o monitor em foco, a janela
    /// ativa, a presença. Devolve se o que o `/v1/estado` mostra mudou.
    pub fn evento_desktop(
        &mut self,
        ov: Option<&mut dyn Overlay>,
        evento: &EventoDesktop,
        agora: Agora,
    ) -> bool {
        self.acertar_relogio(agora);
        if let EventoDesktop::MonitorEmFoco(nome) = evento {
            self.seguir.foco(nome.clone(), agora.mono_ms);
        }
        if let EventoDesktop::JanelaFechou(janela) = evento {
            self.identidades.fechou(janela);
        }
        match evento {
            EventoDesktop::Compartilhando(compartilhando) => {
                self.compartilhamento(*compartilhando, agora.mono_ms);
            }
            // A fonte caiu: o fim do compartilhamento pode se perder no meio.
            // O sinal desliga ali, e a discrição segura (decisão 0081).
            EventoDesktop::Ligado(false) => self.compartilhamento(false, agora.mono_ms),
            _ => {}
        }
        let protetor_antes = self.desktop.protetor_ativo();
        let ativa_antes = self.desktop.janela_ativa.clone();
        let ligado_antes = self.desktop.ligado;
        let ocioso_antes = self.desktop.ocioso;
        let mudou = self.desktop.aplicar(evento, agora.parede_ms);
        if self.desktop.ocioso != Some(true) {
            // A volta conta só para quem ficou longe de verdade: sem mexer
            // há 60 s ou mais, não de volta ao terminal da sessão que espera
            // (lá o Renan já vê o diálogo) e não com o diálogo já visto. Um
            // voo, na hora ou assim que o pet puder aparecer (decisões 0075 e
            // 0090).
            let longe_desde = self.ausente_desde.take();
            if ocioso_antes == Some(true)
                && self.desktop.ocioso == Some(false)
                && !self.olhando_a_espera()
                && self.chamando.as_ref().is_some_and(|c| c.vista_ms.is_none())
                && longe_desde.is_some_and(|d| {
                    agora.mono_ms + OCIOSO_MS >= d + escalada::PARADO_PARA_CHAMAR_MS
                })
            {
                // O Renan voltou com o aviso de pé: o pet acorda (o teto da
                // escalada pode ter soltado a base e deixado ele dormir com o
                // selo; decisão 0090), e o voo da volta mostra o aviso.
                self.acordar(agora.mono_ms, false);
                let ctx = self.contexto_da_escalada(agora.mono_ms);
                if let Some(c) = self.chamando.as_mut() {
                    let passos = c.escalada.voltou(agora.mono_ms, ctx);
                    self.aplicar_escalada(passos, "voltou", agora.mono_ms);
                }
            }
        } else if ocioso_antes != Some(true) {
            self.ausente_desde = Some(agora.mono_ms);
        }
        // Desde quando a janela ativa está ativa (o pronto visto pelo foco):
        // a fonte que volta não sabe desde quando.
        if self.desktop.janela_ativa != ativa_antes
            || (self.desktop.ligado == Some(true) && ligado_antes != Some(true))
        {
            self.ativa_desde = agora.mono_ms;
        }
        // O Renan voltou ao teclado ou ao mouse (ou a conexão começou a
        // contar): o pronto visto pelo foco conta daqui (decisão 0062).
        if *evento == EventoDesktop::Ocioso(false) {
            self.presente_desde = agora.mono_ms;
        }
        // O foco que o clique pediu chegou: o aviso sai (decisão 0057). O
        // socket2 e o foreign-toplevel contam a ativação.
        let ativou = match evento {
            EventoDesktop::JanelaAtiva {
                janela: Some(janela),
                ..
            }
            | EventoDesktop::JanelaInicial { janela, .. } => Some(janela),
            _ => None,
        };
        if let Some(janela) = ativou {
            let (vistos, esperando): (Vec<Focando>, Vec<Focando>) =
                std::mem::take(&mut self.focando)
                    .into_iter()
                    .partition(|f| &f.janela == janela);
            self.focando = esperando;
            for focando in vistos {
                info!("clique: a janela {} ficou ativa", janela.0);
                self.ver(&focando.chave, agora.mono_ms);
            }
        }
        // A proteção de tela do Omarchy abriu ou fechou: o pet sai e volta
        // (decisão 0053).
        if self.desktop.protetor_ativo() != protetor_antes {
            info!(
                "proteção de tela {}",
                if protetor_antes {
                    "fechou: o pet volta"
                } else {
                    "abriu: o pet se esconde"
                }
            );
            if let Some(ov) = ov {
                self.aplicar_visibilidade(ov, agora.mono_ms);
            }
        }
        mudou
    }

    /// O que os eventos do desktop contaram.
    pub fn desktop(&self) -> &EstadoDesktop {
        &self.desktop
    }

    // --- desenho -------------------------------------------------------------

    /// Desenha o quadro de agora (se a janela está viva e algo mudou) e
    /// marca a próxima troca. Com um quadro em voo o desenho espera por ele e
    /// nenhum prazo é marcado: é o [`EventoOverlay::Redesenhar`] que chama de
    /// novo, já com o quadro do instante em que chegar. Com a tela apagada ele
    /// não chega, e o pet fica sem commit nenhum até ela acender (decisão
    /// 0018). `forcar` faz o commit mesmo com um quadro em voo.
    pub fn desenhar(&mut self, ov: &mut dyn Overlay, agora_ms: u64, forcar: bool) {
        // O voo da escalada leva a célula (decisão 0084).
        self.andar_voo(agora_ms);
        let Some(palco) = self.palco else {
            return;
        };
        let Some(pet) = self.pet.as_ref() else {
            return;
        };
        if !matches!(ov.fase(), Fase::Viva { .. }) {
            return;
        }
        let (mut cena, mut proxima) = pet.cena(&palco, agora_ms);
        // O poof da viagem (decisão 0051): a nuvem em volta do corpo, e o pet
        // some na saída e aparece na chegada.
        let poof = match self.seguir.fase() {
            Some(FaseViagem::Poof { inicio_ms }) => Some((inicio_ms, false)),
            Some(FaseViagem::Entrando { inicio_ms }) => Some((inicio_ms, true)),
            _ => None,
        };
        if let Some((inicio_ms, entrando)) = poof {
            match poof::passo(inicio_ms, agora_ms) {
                Some((passo, prazo)) => {
                    if !poof::mostra_o_pet(passo, entrando) {
                        cena.clear();
                    }
                    if let Some(corpo) = pet.toque_no_palco(&palco) {
                        cena.extend(poof::nuvem(corpo, palco.d, passo, entrando));
                    }
                    proxima = Some(proxima.map_or(prazo, |p| p.min(prazo)));
                }
                None if !entrando => cena.clear(),
                None => {}
            }
        }
        // Os selos ao lado do corpo (decisão 0083), parados; o do aviso
        // pulsa na L4, uma troca por segundo. No voo, só o "!!".
        if poof.is_none()
            && self.voo.is_none()
            && let Some(corpo) = pet.toque_no_palco(&palco)
        {
            let fileira = self.fileira(agora_ms);
            if !fileira.vazia() {
                cena.extend(selos::elementos(
                    &fileira,
                    corpo,
                    palco.area,
                    balao::dt(palco.d),
                ));
            }
            if let Some(prazo) = self.proxima_troca_do_pulso(agora_ms) {
                proxima = Some(proxima.map_or(prazo, |p| p.min(prazo)));
            }
        }
        // O "!!" do voo em cima da cabeça, piscando, andando com o corpo.
        if let Some(voo) = self.voo
            && voo.exclamacoes_acesas(agora_ms)
            && let Some(corpo) = pet.toque_no_palco(&palco)
        {
            let (w, h) = selos::tamanho_das_exclamacoes(palco.d);
            let x = (corpo.x + corpo.w / 2 - w / 2)
                .min(palco.area.direita() - w)
                .max(palco.area.x);
            let y = (corpo.y - h - 2 * palco.d).max(palco.area.y);
            cena.extend(selos::exclamacoes(x, y, palco.d));
        }
        // O selo "zZ" da soneca, parado, enquanto ela durar.
        if let Some(ate) = self.soneca(agora_ms)
            && let Some(corpo) = pet.toque_no_palco(&palco)
            && poof.is_none()
        {
            cena.extend(balao::selo_zz(corpo, palco.area, balao::dt(palco.d)));
            proxima = Some(proxima.map_or(ate, |p| p.min(ate)));
        }
        // O coração da risadinha do clique que leva ao terminal.
        if let Some(ate) = self.coracao_ate.filter(|&ate| agora_ms < ate)
            && let Some(corpo) = pet.toque_no_palco(&palco)
            && poof.is_none()
        {
            cena.extend(balao::coracao(corpo, palco.area, balao::dt(palco.d)));
            proxima = Some(proxima.map_or(ate, |p| p.min(ate)));
        }
        // O balão em cima do corpo, até o prazo dele.
        if let Some(balao) = self.balao.as_ref().filter(|b| agora_ms < b.ate_ms)
            && let Some(corpo) = pet.toque_no_palco(&palco)
        {
            cena.extend(balao::elementos(
                &balao.linhas,
                corpo,
                palco.area,
                balao::dt(palco.d),
            ));
            proxima = Some(proxima.map_or(balao.ate_ms, |p| p.min(balao.ate_ms)));
        }
        // O confete da festa (decisão 0085), no fim da cena: os passos que
        // venceram andam todos; acaba quando o último pedaço sai da tela, no
        // prazo, ou numa janela que não cobre o monitor (o palco transitório
        // é do M8).
        if let Some(efeito) = self.efeito.as_mut() {
            let devidos = agora_ms.saturating_sub(efeito.inicio_ms) / PASSO_CONFETE_MS;
            while efeito.passos < devidos && !efeito.confete.acabou() {
                efeito.confete.passo();
                efeito.passos += 1;
            }
        }
        if self
            .efeito
            .as_ref()
            .is_some_and(|e| e.confete.acabou() || agora_ms >= e.fim_ms)
            || !ov.capacidades().tela_inteira
        {
            self.efeito = None;
        }
        if let Some(efeito) = self.efeito.as_ref() {
            cena.extend(efeito.confete.elementos());
            let prazo = efeito.inicio_ms + (efeito.passos + 1) * PASSO_CONFETE_MS;
            proxima = Some(proxima.map_or(prazo, |p| p.min(prazo)));
        }
        if self.estresse.as_ref().is_some_and(|e| agora_ms >= e.fim_ms) {
            self.estresse = None;
            info!("debug: estresse acabou");
        }
        if let Some(estresse) = self.estresse.as_mut() {
            estresse.avancar(agora_ms);
            cena.extend(estresse.chuva.elementos());
            let prazo = estresse.proximo_prazo();
            proxima = Some(proxima.map_or(prazo, |p| p.min(prazo)));
        }
        // No voo, os quadros andam na grade de 34 ms dele: os passos, o
        // pisca e o resto (as asas, o balão) entram no passo seguinte, e nunca
        // saem dois quadros a menos de 34 ms (até 30 por segundo).
        if let Some(voo) = self.voo {
            if let Some(p) = voo.proxima(agora_ms) {
                proxima = Some(proxima.map_or(p, |q| q.min(p)));
            }
            proxima = proxima.map(|p| voo.na_grade(p));
        } else if let Some(efeito) = self.efeito.as_ref() {
            // O mesmo com o confete: os quadros na grade dele.
            proxima = proxima.map(|p| na_grade(efeito.inicio_ms, PASSO_CONFETE_MS, p));
        }
        // Arrastando, a área de toque é o palco inteiro: o arraste continua
        // até numa área de trabalho vazia, onde o compositor pode perder a
        // pegada implícita. Ao soltar, volta ao corpo. No voo, ela anda com o
        // corpo.
        let toque = if self.arraste.arrastando() {
            Some(Ret::novo(0, 0, palco.tela.0, palco.tela.1))
        } else {
            pet.toque_no_palco(&palco)
        };
        match ov.desenhar(&cena, pet.skin(), toque, forcar) {
            Ok(Desenho::Enviado { retangulos, area }) => {
                self.commits.contar(agora_ms);
                if self.voo.is_some() {
                    self.voo_quadros = self.voo_quadros.saturating_add(1);
                }
                if self.estresse.is_none() {
                    depurar!("quadro enviado: {retangulos} retângulo(s) de dano, {area} px");
                }
            }
            Ok(Desenho::SoEstado) => self.commits.contar(agora_ms),
            // O quadro em voo chama `desenhar` de novo quando for mostrado.
            Ok(Desenho::Adiado) => return,
            Ok(Desenho::SemMudanca) => {}
            Err(e) => aviso!("desenho: {e}"),
        }
        // Sem nada que mude sozinho (a pose do sono profundo, parada), nenhum
        // prazo: nenhum commit até um evento.
        self.proximo_quadro = proxima;
    }

    /// A próxima troca de quadro da animação, se houver uma marcada.
    pub fn prazo_da_animacao(&self) -> Option<u64> {
        self.proximo_quadro
    }

    /// O prazo da animação venceu: desenha o quadro de agora.
    pub fn vencer_animacao(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        if self.proximo_quadro.is_some_and(|p| p <= agora_ms) {
            self.proximo_quadro = None;
            self.desenhar(ov, agora_ms, false);
        }
    }

    /// Os prazos do Motor que vencem com a conexão: o arraste (segurar e o
    /// fail-safe), a viagem para o monitor em foco e a animação.
    pub fn vencer(&mut self, punho: &mut dyn Punho, agora_ms: u64) {
        let gesto = self.arraste.vencer(agora_ms);
        self.aplicar_gesto(punho, gesto, agora_ms);
        self.vencer_viagem(punho.janela(), agora_ms);
        if self.balao.as_ref().is_some_and(|b| agora_ms >= b.ate_ms) {
            self.balao = None;
            self.desenhar(punho.janela(), agora_ms, false);
        }
        if self.soneca_ate.is_some_and(|ate| agora_ms >= ate) {
            self.soneca_ate = None;
            info!("soneca acabou");
            self.desenhar(punho.janela(), agora_ms, false);
        }
        if self.coracao_ate.is_some_and(|ate| agora_ms >= ate) {
            self.coracao_ate = None;
            self.desenhar(punho.janela(), agora_ms, false);
        }
        // O desktop não contou que a janela do clique ficou ativa: o aviso
        // fica, e o balão diz. Só o do clique mais novo: um clique depois
        // desse ainda espera a confirmação dele.
        let (vencidos, esperando): (Vec<Focando>, Vec<Focando>) = std::mem::take(&mut self.focando)
            .into_iter()
            .partition(|f| agora_ms >= f.ate_ms);
        self.focando = esperando;
        for focando in &vencidos {
            info!(
                "clique: a janela {} não ficou ativa em {} ms",
                focando.janela.0, CONFIRMAR_FOCO_MS
            );
        }
        if self.focando.is_empty()
            && let Some(focando) = vencidos.last()
        {
            self.balao_sem_foco(
                punho.janela(),
                &focando.chave,
                focando.proj.as_deref(),
                focando.tipo,
                "não consegui focar a janela dela",
                agora_ms,
            );
        }
        self.vencer_animacao(punho.janela(), agora_ms);
    }

    /// Os mesmos prazos de [`Self::vencer`], sem conexão (o compositor caiu
    /// ou a conexão está no backoff, com o socket2 ainda contando o foco):
    /// nada a desenhar, mas nenhum prazo vencido pode ficar armado, senão o
    /// laço acorda de novo na hora, sem fim, a 100% de CPU (decisão 0059).
    pub fn vencer_sem_conexao(&mut self, agora_ms: u64) {
        if self.arraste.vencer(agora_ms) != Gesto::Nada {
            self.arraste.cancelar();
        }
        // Sem janela, não há para onde viajar: o prazo de conferir o foco
        // sai, e a camada nova (com output NULL) nasce no monitor em foco.
        self.seguir.decidir(agora_ms, None, false);
        if self.balao.as_ref().is_some_and(|b| agora_ms >= b.ate_ms) {
            self.balao = None;
        }
        if self.soneca_ate.is_some_and(|ate| agora_ms >= ate) {
            self.soneca_ate = None;
            info!("soneca acabou");
        }
        if self.coracao_ate.is_some_and(|ate| agora_ms >= ate) {
            self.coracao_ate = None;
        }
        self.focando.retain(|f| agora_ms < f.ate_ms);
        if self.proximo_quadro.is_some_and(|p| p <= agora_ms) {
            self.proximo_quadro = None;
        }
    }

    /// O próximo prazo do Motor (cérebro, animação, arraste ou viagem).
    pub fn proximo_prazo(&self) -> Option<u64> {
        [
            self.prazo_do_cerebro(),
            self.proximo_quadro,
            self.arraste.prazo(),
            self.seguir.prazo(),
            self.balao.as_ref().map(|b| b.ate_ms),
            self.soneca_ate,
            self.coracao_ate,
            self.focando.iter().map(|f| f.ate_ms).min(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    // --- reações -------------------------------------------------------------

    /// Toca uma reação uma vez e volta à pose (o cérebro e o `tocar`).
    /// `false` sem pet (sem janela ou sem personagem) ou quando a skin não
    /// sabe tocar a reação. Com o pet escondido a animação corre no relógio
    /// sem desenhar nada.
    pub fn tocar(&mut self, ov: Option<&mut dyn Overlay>, reacao: &str, agora_ms: u64) -> bool {
        let Some(pet) = self.pet.as_mut() else {
            return false;
        };
        if !pet.tocar(reacao, agora_ms) {
            return false;
        }
        if let Some(ov) = ov {
            self.desenhar(ov, agora_ms, false);
        }
        true
    }

    /// `tocar` do `/v1/comando`: a tag que a skin toca para a reação e se
    /// ela apareceu (decisão 0033). Escondido, não toca.
    pub fn tocar_comando(
        &mut self,
        ov: Option<&mut dyn Overlay>,
        reacao: &str,
        agora_ms: u64,
    ) -> Tocou {
        let Some(skin) = self.skin.clone() else {
            return Tocou::SemPersonagem;
        };
        let Some(tag) = animador::tag_da_reacao(&skin, reacao)
            .and_then(|i| skin.tags.get(i))
            .map(|t| t.nome.clone())
        else {
            return Tocou::Desconhecida {
                skin: skin.id.clone(),
            };
        };
        if !self.visivel {
            return Tocou::ForaDaTela {
                tag,
                motivo: "o pet está escondido (bin/pet mostrar)",
            };
        }
        let Some(ov) = ov else {
            return Tocou::ForaDaTela {
                tag,
                motivo: "sem compositor",
            };
        };
        if self.tocar(Some(ov), reacao, agora_ms) {
            Tocou::NaTela { tag }
        } else {
            Tocou::ForaDaTela {
                tag,
                motivo: "a janela do pet ainda não tem o personagem",
            }
        }
    }

    /// Confete pela tela inteira por `segundos`, a `fps` quadros por
    /// segundo; depois volta ao repouso. O confete anda na grade de arte do
    /// pet (múltiplos de D a partir do canto da célula).
    pub fn estresse(&mut self, ov: &mut dyn Overlay, fps: u32, segundos: u32, agora_ms: u64) {
        let Some(palco) = self.palco else {
            aviso!("debug: estresse pedido com o pet fora da tela");
            return;
        };
        // Numa janela pequena (M8) o confete pela tela inteira pede o palco
        // transitório, que ainda não existe.
        if !ov.capacidades().tela_inteira {
            aviso!(
                "debug: estresse pede uma janela do tamanho do monitor (o palco transitório chega no M8)"
            );
            return;
        }
        let grade = Grade {
            x: palco.x,
            y: palco.y,
            d: palco.d,
        };
        self.estresse = Some(Estresse {
            chuva: Chuva::nova(CONFETES, palco.tela, grade, LADO_CONFETE, SEMENTE_CONFETE),
            fps: fps.max(1) as u64,
            inicio_ms: agora_ms,
            passos: 0,
            fim_ms: agora_ms + segundos as u64 * 1000,
        });
        info!("debug: estresse com {CONFETES} confetes a {fps} fps por {segundos} s");
        self.desenhar(ov, agora_ms, false);
    }

    /// O ponto (x, y) do palco cai no corpo do pet (o clique e o arraste do
    /// M4 começam aqui, com o [`crate::plataforma::EventoPonteiro`] já no
    /// palco). `false` sem pet ou sem palco.
    pub fn acerta_o_pet(&self, x: i32, y: i32) -> bool {
        match (&self.pet, &self.palco) {
            (Some(pet), Some(palco)) => pet.acerta(palco, x, y),
            _ => false,
        }
    }

    /// Fim do processo: esconde o pet (quadro transparente) e larga a
    /// janela, esperando o sistema confirmar se havia uma.
    pub fn encerrar(&mut self, ov: &mut dyn Overlay, agora_ms: u64) {
        let tinha = ov.fase() != Fase::Ausente;
        let passo = passo_de_visibilidade(ov.fase(), false);
        self.aplicar_passo(ov, passo, agora_ms);
        self.proximo_quadro = None;
        ov.encerrar(tinha);
    }

    // --- o que vai para fora -------------------------------------------------

    /// O painel do `/v1/estado`. Sem conexão (sem compositor), o painel
    /// vazio, só com o que os eventos do desktop contaram.
    pub fn painel(&mut self, punho: Option<&dyn Punho>, agora_ms: u64) -> Painel {
        let intencoes = self.intencoes.painel(agora_ms);
        let fotografia = self.painel_da_tela(agora_ms);
        let desenho = self.painel_do_desenho(agora_ms);
        let Some(punho) = punho else {
            return Painel {
                desktop: self.desktop.painel(Default::default(), Default::default()),
                intencoes,
                fotografia,
                ..Painel::default()
            };
        };
        let ov = punho.ver_janela();
        let desktop = punho.ver_desktop();
        let painel_desktop = self.desktop.painel(desktop.capacidades(), desktop.info());
        let info = ov.info();
        // Pelo relógio: com a tela apagada o último quadro do estresse pode
        // nunca ser desenhado, mas o estresse acabou do mesmo jeito.
        let estresse = self.estresse.as_ref().is_some_and(|e| agora_ms < e.fim_ms);
        let (d, sprite_disp) = match (&self.palco, &self.pet) {
            (Some(palco), Some(pet)) if info.visivel => (Some(palco.d), pet.sprite_disp(palco)),
            _ => (None, None),
        };
        Painel {
            monitor: info.monitor,
            escala: info.escala,
            d,
            sprite_disp,
            regiao_entrada: info.regiao,
            visivel: info.visivel,
            estresse,
            commits_por_min: self.commits.por_minuto(agora_ms),
            commits_total: self.commits.total,
            shm_bytes: info.shm_bytes,
            reacao: self
                .pet
                .as_ref()
                .and_then(|pet| pet.reacao(agora_ms))
                .map(str::to_owned),
            arrastando: self.arraste.arrastando(),
            viagem: self.seguir.fase().map(FaseViagem::nome),
            balao: self.balao(agora_ms).map(|b| b.linhas.clone()),
            soneca_restante_s: self
                .soneca(agora_ms)
                .map(|ate| (ate - agora_ms).div_ceil(1000)),
            desktop: painel_desktop,
            focando: self.focando.last().map(|f| f.janela.0.clone()),
            intencoes,
            fotografia,
            desenho: if info.visivel {
                desenho
            } else {
                PainelDesenho::default()
            },
        }
    }

    /// O que a janela desenha agora, pelo estado do Motor (decisão 0086).
    fn painel_do_desenho(&self, agora_ms: u64) -> PainelDesenho {
        let Some(pet) = self.pet.as_ref().filter(|_| self.palco.is_some()) else {
            return PainelDesenho::default();
        };
        let base = pet.base();
        let fileira = self.fileira(agora_ms);
        // O voo pelo relógio: o que acabou sem quadros (a tela apagada) sai
        // no próximo desenho, e a fileira volta nele.
        let voando = self
            .voo
            .and_then(|v| v.fase(agora_ms).map(|fase| (v.motivo, fase)));
        let selos =
            (voando.is_none() && self.seguir.fase().is_none() && !fileira.vazia()).then(|| {
                PainelFileira {
                    aviso: fileira.aviso.map(|a| match a {
                        selos::Aviso::Normal => "normal",
                        selos::Aviso::Aceso => "aceso",
                    }),
                    pulso: self.pulso_desde.is_some() && self.chamando.is_some(),
                    mais: fileira.mais,
                    corrente: fileira.corrente,
                    bandeiras: fileira.bandeiras.len(),
                }
            });
        PainelDesenho {
            base: Some(base.estado.clone()),
            ritmo: Some(base.ritmo.nome()),
            selos,
            voo: voando.map(|(motivo, fase)| PainelVoo {
                fase: fase.nome(),
                motivo,
            }),
            confete: self.confete_na_tela(agora_ms),
        }
    }

    /// O sprite como está na tela agora, para a checagem de nitidez.
    pub fn quadro_esperado(&self, ov: &dyn Overlay) -> Option<QuadroEsperado> {
        let ultimo = ov.ultimo_quadro()?;
        let pet = self.pet.as_ref()?;
        let palco = self.palco.as_ref()?;
        let sprite = ultimo
            .cena
            .iter()
            .find(|e| matches!(e, Elemento::Sprite { .. }))?;
        let Elemento::Sprite { x, y, d, .. } = *sprite else {
            return None;
        };
        let area = pet.sprite_disp(palco)?;
        Some(QuadroEsperado {
            monitor: ultimo.monitor,
            area,
            grade: (x, y),
            d,
            seq: ultimo.seq,
            idade_ms: ultimo.idade_ms,
            rgba: cena::rgba_do_sprite(pet.skin(), sprite, area),
        })
    }
}

/// A célula lá em cima no voo da escalada: o corpo no meio da área útil, com
/// o topo dele logo abaixo do "!!" (a margem da borda, o "!!" e dois pixels de
/// arte), preso para o corpo ficar na área.
fn alvo_do_voo(palco: &Palco, pet: &Pet) -> (i32, i32) {
    let d = palco.d;
    let toque = pet.skin().ancoras.toque(false);
    let (_, altura) = selos::tamanho_das_exclamacoes(d);
    let margem = crate::geometria::para_dispositivo(crate::geometria::MARGEM_LOGICA, palco.escala);
    let x = palco.area.x + palco.area.w / 2 - (toque.x * d + toque.w * d / 2);
    let y = palco.area.y + margem + altura + 2 * d - toque.y * d;
    pet.prender(palco, x, y)
}

#[cfg(test)]
mod testes;
