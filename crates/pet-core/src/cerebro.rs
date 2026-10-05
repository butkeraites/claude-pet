//! Cérebro (M3, decisões 0020 e 0032; M5, decisões 0072 em diante): eventos
//! do Claude Code → reações.
//!
//! Puro: o relógio vem de fora ([`Agora`]) e nada aqui faz I/O. O daemon
//! entrega cada evento validado ([`crate::evento`]) com a hora em que ele
//! chegou, chama [`Cerebro::tique`] no [`Cerebro::proximo_prazo`] e toca as
//! [`Reacao`]s que voltam.
//!
//! **Sessões** por `sid`. Só contam as de origem permitida
//! (`sessoes.origens`, padrão `["cli"]`): `claude -p`, SDK e IDE ficam de
//! fora. Eventos de teste (`teste: true`, do `bin/pet testar`) vivem num
//! mundo à parte, nunca se misturam com sessões reais e somem 60 s depois
//! do último evento. `SessionEnd` sempre larga a sessão e o turno dela; o
//! tchau (`bye`) só vem quando o processo está saindo (não num `/clear` nem
//! numa retomada, que seguem com outro `sid`) e não sobra nenhuma sessão.
//!
//! **Turnos** por `turno` (`prompt_id`). O `UserPromptSubmit` abre o turno
//! (t0); as ferramentas da thread principal contam como trabalho (Edit,
//! Write, MultiEdit, NotebookEdit, Bash) ou outras, com os arquivos únicos
//! (`arq`, no máximo [`ARQUIVOS_POR_TURNO`] lembrados) e a soma do tempo de
//! ferramenta (`dur`). Ferramentas de um subagente contam para o turno em
//! que ele nasceu, mas não mudam o estado da sessão. Evento de um turno que
//! ninguém abriu (o pet reiniciou no meio) abre um turno implícito.
//!
//! **A origem do turno** (decisão 0073): o 2.1.288 não manda o `source`, e
//! uma notificação de tarefa ou um tique de laço chegam como um prompt
//! qualquer. [`classificar`] decide pelo `src` (se um Claude Code futuro
//! mandar), pelo `orig` que o hook calcula (decisão 0072), pelos agendamentos
//! do último Stop (`crn`) com a evidência do Motor (o Renan longe do teclado,
//! ou outra janela certa em foco) e, com o hook antigo, pela corrente aberta.
//! O turno de máquina fora de uma corrente fica no teto T1, discreto, sem
//! pronto; o T0 dele não reage. Ele nunca resolve o pronto de antes.
//!
//! **A corrente** (decisão 0073): um Stop com agente em voo (`bgt` com
//! [`TAREFAS_DE_AGENTE`]) abre a corrente da sessão; enquanto ela está
//! aberta, todo Stop da sessão é dela: com agente em voo o turno entra (sem
//! festa, sem pronto), sem agente em voo ela fecha com uma festa só, pela
//! soma dos turnos e do trabalho dos agentes que veio depois (pelo `aid`). O
//! agente que acorda (um `SubagentStart` de um `aid` já visto) é trabalho em
//! segundo plano, não continuação da thread principal. A corrente expira
//! [`VIDA_CORRENTE_MS`] depois do último evento dela, sem festa.
//!
//! Os hooks são async e chegam fora de ordem. Além do turno aberto, cada
//! sessão guarda dois turnos que ainda podem receber eventos:
//!
//! - **Stop** inicia uma acomodação de 0,8 s: um PostToolUse atrasado (com
//!   `ts` anterior ao Stop) ainda conta, e só um evento de trabalho da
//!   thread principal com `ts` **posterior** ao Stop a cancela. Um prompt
//!   novo encerra a acomodação na hora. Dedupe por `(sid, turno)`.
//! - **Trocado:** um turno novo começou sem o Stop do anterior. Ou foi Esc
//!   (o Stop nunca vem), ou os dois hooks chegaram trocados (um prompt na
//!   fila entra logo depois do Stop). O anterior espera 0,8 s: se o Stop
//!   dele chega, comemora; senão fecha sem festa. Eventos atrasados dele
//!   contam nele, sem mexer no turno novo.
//! - **Comemorado:** o último turno fechado por um Stop. Um Stop hook de
//!   outro plugin pode segurar o Claude, e aí a continuação chega segundos
//!   depois da festa, com o mesmo `prompt_id`. Um evento de trabalho da
//!   thread principal com `ts` posterior ao Stop reabre o turno, e o Stop
//!   seguinte (com `sha`) só reage se o nível subir. Um prompt novo encerra
//!   essa chance.
//!
//! O estado da sessão só muda com evento aplicado a um turno vivo: evento
//! ignorado, atrasado ou do turno trocado não mexe nele.
//!
//! **Nível**: T0 (nenhuma ferramenta de trabalho, nenhum subagente, nenhum
//! arquivo editado) → `nod`, o aceno discreto; o resto → T1, `done_small`,
//! o pulinho. Cada turno guarda os componentes inteiros em [`RegistroTurno`]
//! (o `/v1/estado.turnos`).
//!
//! **Avisos** (M4, decisão 0057): cada sessão tem um espaço de aviso, o que
//! ela pede ao Renan — esperando você (uma permissão, uma pergunta, o
//! plano), erro (o turno acabou num erro da API) ou pronto (o Claude
//! terminou o turno). O clique no pet vai ao mais urgente
//! ([`Cerebro::pendencias`]); visto ([`Cerebro::ver`]), o aviso sai. Mudar de
//! estado resolve o aviso de antes, e entrar em "esperando você" ou em erro
//! abre um; o pronto abre quando a acomodação do Stop termina (a festa) e
//! um prompt digitado o resolve. O `idle_prompt` nunca abre nem resolve
//! aviso, e um evento atrasado (mais velho que o estado de agora) não mexe
//! nele. O pronto e o erro somem sozinhos em [`VIDA_AVISO_MS`]; o "esperando
//! você" só sai com um evento da própria sessão ou visto.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use crate::config::{Config, ModoCelebracao, Pesos};
use crate::evento::{self, Evento, ORIG_NOTIFICACAO};

/// Acomodação depois de um Stop, e espera pelo Stop atrasado de um turno
/// trocado.
pub const ACOMODACAO_MS: u64 = 800;
/// O `ts` do hook só vale se estiver a até isto da hora de chegada.
pub const JANELA_TS_MS: u64 = 6 * 60 * 60 * 1000;
/// Uma sessão de teste some depois disto sem eventos.
pub const VIDA_TESTE_MS: u64 = 60_000;
/// Uma sessão real some depois disto sem eventos (o `SessionEnd` de um
/// processo morto nunca chega).
pub const VIDA_SESSAO_MS: u64 = 12 * 60 * 60 * 1000;
/// Uma corrente de agentes sem evento nenhum por isto expira, sem festa
/// (decisão 0073).
pub const VIDA_CORRENTE_MS: u64 = 12 * 60 * 60 * 1000;
/// Turnos fechados no `/v1/estado.turnos`.
pub const TURNOS_GUARDADOS: usize = 20;
/// O pronto e o erro somem sozinhos depois disto (decisão 0057).
pub const VIDA_AVISO_MS: u64 = 2 * 60 * 60 * 1000;
/// Trabalhando, pensando ou compactando sem evento da sessão por isto, a
/// sessão volta a parada (o turno continua aberto; decisão 0076).
pub const ATIVA_SEM_EVENTO_MS: u64 = 5 * 60 * 1000;
/// O erro e o cansado ficam isto na tela (o aviso de erro fica, como no M4;
/// decisão 0076).
pub const ERRO_NA_TELA_MS: u64 = 60_000;
/// O erro do `StopFailure` que deixa o Zeca cansado: o limite de uso.
pub const ERRO_DE_LIMITE: &str = "rate_limit";
/// Um gatilho atrasado de um diálogo (até isto antes do aviso) ainda refina
/// o tipo da espera (decisão 0075).
pub const JANELA_DO_DIALOGO_MS: u64 = 5_000;
/// Arquivos diferentes lembrados por turno. Passou disso, cada arquivo novo
/// só soma num contador: um processo local mandando `arq` sempre novo não
/// cresce a memória.
pub const ARQUIVOS_POR_TURNO: usize = 1024;
/// Turnos fechados lembrados por sessão, para o dedupe do Stop.
const FECHADOS_POR_SESSAO: usize = 32;
/// Turnos fechados que esperam o Motor tirar ([`Cerebro::tirar_turnos_fechados`]).
const FECHADOS_A_TIRAR: usize = 64;
/// Subagentes lembrados por sessão (para atribuir ferramentas ao turno).
const AGENTES_POR_SESSAO: usize = 32;
/// Turnos guardados um a um numa corrente; passou disso, somam num só.
const MEMBROS_POR_CORRENTE: usize = 64;
/// Agentes em voo lembrados por corrente.
const AGENTES_POR_CORRENTE: usize = 64;
/// Teto de sessões acompanhadas de cada tipo (real ou teste); acima disso
/// a mais parada sai.
const MAX_SESSOES: usize = 64;
/// Teto de motivos distintos em `ignorados` (cada origem desconhecida é um
/// motivo): passou disso, conta em `outros`.
const MAX_MOTIVOS: usize = 32;
/// Motivos do `SessionEnd` em que o processo continua com outro `sid`
/// (`/clear`, retomada): sem tchau.
const FIM_SEM_TCHAU: [&str; 2] = ["clear", "resume"];

/// Ferramentas que contam como trabalho de verdade (decisão 0003).
pub const FERRAMENTAS_DE_TRABALHO: [&str; 5] =
    ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash"];
/// Ferramentas que lançam um subagente: o tempo delas é a vida dele, com o
/// pensar junto, e fica fora do `min_ativos` (as ferramentas do subagente já
/// contam pelo `aid`; decisão 0074).
pub const FERRAMENTAS_DE_LANCAMENTO: [&str; 2] = ["Agent", "Task"];
/// O intervalo do T3 no modo `sempre_grande` (decisão 0074).
pub const INTERVALO_T3_SEMPRE_GRANDE_MS: u64 = 2 * 60 * 1000;

/// Os tipos de tarefa em segundo plano (os rótulos normalizados do `bgt`)
/// que são trabalho de agente: só eles abrem ou estendem uma corrente
/// (decisão 0073). `shell`, `monitor`, `mcp_task` e `outro` nunca seguram uma
/// festa; `dream`, `auto_mode_scan` e `memory_import` são ignorados.
pub const TAREFAS_DE_AGENTE: [&str; 4] = ["subagent", "workflow", "teammate", "cloud_session"];

/// Aceno discreto (T0).
pub const ACENO: &str = "nod";
/// Pulinho (T1).
pub const PULINHO: &str = "done_small";
/// Voo curto com confete (T2).
pub const VOO_CURTO: &str = "done_medium";
/// Voo grande atravessando a tela (T3).
pub const VOO_GRANDE: &str = "done_big";
/// Tchau: a última sessão acabou.
pub const TCHAU: &str = "bye";

/// O relógio, injetado: parede (ms desde 1970, o mesmo do `ts` dos hooks)
/// para horas de evento e de reação; monotônico (ms) para os prazos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agora {
    pub parede_ms: u64,
    pub mono_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigCerebro {
    /// Origens aceitas (`$CLAUDE_CODE_ENTRYPOINT`); evento sem origem não
    /// conta.
    pub origens: Vec<String>,
    pub modo: ModoCelebracao,
    /// Os pesos e os limites da pontuação (decisão 0074).
    pub pesos: Pesos,
    /// O T3 no máximo a cada tanto (0: sem limite).
    pub intervalo_t3_ms: u64,
}

impl Default for ConfigCerebro {
    fn default() -> Self {
        ConfigCerebro {
            origens: vec!["cli".into()],
            modo: ModoCelebracao::Proporcional,
            pesos: Pesos::default(),
            intervalo_t3_ms: 10 * 60 * 1000,
        }
    }
}

impl ConfigCerebro {
    pub fn de(config: &Config) -> Self {
        ConfigCerebro {
            origens: config.sessoes_origens.clone(),
            modo: config.celebracao_modo,
            pesos: config.pontuacao,
            intervalo_t3_ms: (config.intervalo_t3_min * 60_000.0).round() as u64,
        }
    }
}

/// Nível da festa de um turno (decisões 0003 e 0074): T0 o aceno, T1 o
/// pulinho, T2 o voo curto, T3 o voo grande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Nivel {
    T0,
    T1,
    T2,
    T3,
}

impl Nivel {
    /// A reação do nível.
    pub fn reacao(self) -> &'static str {
        match self {
            Nivel::T0 => ACENO,
            Nivel::T1 => PULINHO,
            Nivel::T2 => VOO_CURTO,
            Nivel::T3 => VOO_GRANDE,
        }
    }
}

/// A pontuação de um turno (ou de uma corrente) e a parte de cada
/// componente, com duas casas (decisão 0074).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Pontuacao {
    /// O tempo de ferramenta, em minutos (nunca o tempo de pensar).
    pub min_ativos: f64,
    pub total: f64,
    pub minutos: f64,
    pub trabalho: f64,
    pub outras: f64,
    pub arquivos: f64,
    pub subagentes: f64,
}

fn duas_casas(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn eh_falso(b: &bool) -> bool {
    !*b
}

/// Uma reação para o pet tocar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reacao {
    /// `nod`, `done_small`, `done_medium`, `done_big` ou `bye`.
    pub nome: &'static str,
    /// Os 8 primeiros caracteres do `sid`.
    pub sid8: String,
    pub proj: Option<String>,
    /// Hora da reação (ms desde 1970).
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nivel: Option<Nivel>,
    pub teste: bool,
    /// Uma festa discreta (o turno de máquina, decisão 0073): sem balão.
    #[serde(skip_serializing_if = "eh_falso")]
    pub discreta: bool,
}

/// Como um turno acabou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fim {
    /// Stop e acomodação: o turno é classificado e comemora.
    Stop,
    /// Ferramenta interrompida (Esc): sem festa.
    Interrompido,
    /// `idle_prompt` com o turno aberto e sem Stop: sem festa.
    Ocioso,
    /// Erro de API (StopFailure): sem festa.
    Falhou,
    /// Um turno novo começou e o Stop deste não chegou na espera (Esc no
    /// meio da resposta): sem festa.
    Substituido,
    /// A sessão acabou.
    SessaoEncerrada,
    /// A corrente de agentes ficou sem evento por [`VIDA_CORRENTE_MS`]: sem
    /// festa.
    CorrenteExpirou,
}

/// De onde veio o prompt que abriu o turno (decisão 0073).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrigemTurno {
    /// O Renan digitou (ou não há prova do contrário).
    Digitado,
    /// O Claude Code acordou a sessão com o aviso de uma tarefa em segundo
    /// plano (`orig = notificacao`, ou um prompt sem marca com a corrente
    /// aberta, no hook antigo).
    Notificacao,
    /// Um laço ou um agendamento (`loop_wakeup`, `schedule_wakeup`,
    /// `poll_event`, ou um prompt comum com agendamento pendente que o Renan
    /// não digitou).
    Tique,
    /// `src = system` (um Claude Code que mande o `source`).
    Sistema,
}

impl OrigemTurno {
    /// Foi a máquina que começou o turno.
    pub fn maquina(self) -> bool {
        self != OrigemTurno::Digitado
    }
}

/// O que o Motor sabe da hora de um prompt comum (decisão 0073): prova de
/// que o Renan não o digitou.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Evidencia {
    /// O Renan estava longe do teclado e do mouse (o `ext_idle_notifier_v1`).
    pub ausente: bool,
    /// A janela da sessão é certa, e o anel diz que outra estava ativa na
    /// hora do prompt.
    pub outra_janela: bool,
}

/// A origem de um prompt (decisão 0073): `src` se vier; `orig =
/// notificacao`; com agendamento pendente (`crons`) e prova de que não foi
/// digitado, tique; sem `orig` (o hook de antes do M5) e com a corrente
/// aberta, notificação; senão, digitado.
pub fn classificar(
    src: Option<&str>,
    orig: Option<&str>,
    corrente_aberta: bool,
    crons: Option<u64>,
    evidencia: Evidencia,
) -> OrigemTurno {
    match src {
        Some("user") => return OrigemTurno::Digitado,
        Some("system") => return OrigemTurno::Sistema,
        Some("loop_wakeup" | "schedule_wakeup" | "poll_event") => return OrigemTurno::Tique,
        _ => {}
    }
    match orig {
        Some(ORIG_NOTIFICACAO) => OrigemTurno::Notificacao,
        Some(_) => {
            let agendada = crons.is_some_and(|n| n > 0);
            if agendada && (evidencia.ausente || evidencia.outra_janela) {
                OrigemTurno::Tique
            } else {
                OrigemTurno::Digitado
            }
        }
        None if corrente_aberta => OrigemTurno::Notificacao,
        None => OrigemTurno::Digitado,
    }
}

/// A corrente de um turno no registro: se ficou aberta (o turno entrou nela)
/// ou se fechou nele, e quantos turnos ela teve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ResumoCorrente {
    pub aberta: bool,
    pub turnos: u32,
}

/// Os componentes de um turno fechado: o que o M5 pontua.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegistroTurno {
    pub sid8: String,
    /// Os 8 primeiros caracteres do `prompt_id`.
    pub turno8: Option<String>,
    pub proj: Option<String>,
    pub teste: bool,
    /// Aberto por um evento qualquer, sem o `UserPromptSubmit`.
    pub implicito: bool,
    /// `source` do `UserPromptSubmit` (`user`, `system`, …), se veio.
    pub src: Option<String>,
    /// De onde veio o prompt (decisão 0073).
    pub origem: OrigemTurno,
    pub t0_ms: u64,
    pub fim_ms: u64,
    /// Relógio do turno (fim − t0): só para calibrar, nunca pontua.
    pub relogio_ms: u64,
    /// Ferramentas de trabalho (Edit, Write, MultiEdit, NotebookEdit, Bash).
    pub trabalho: u32,
    pub outras: u32,
    /// Arquivos diferentes editados.
    pub arquivos: u32,
    pub subagentes: u32,
    /// Ferramentas que falharam (também contadas em trabalho ou outras).
    pub falhas: u32,
    /// Ferramentas de subagentes (também contadas em trabalho ou outras).
    pub de_agentes: u32,
    /// Soma do `duration_ms` das ferramentas.
    pub dur_ms: u64,
    /// Tarefas em segundo plano no Stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<u64>,
    /// O Stop veio com `stop_hook_active`.
    pub sha: bool,
    /// Quantas vezes o turno reabriu depois de um Stop (um Stop hook de
    /// outro plugin segurou o Claude). O registro é um só por turno.
    pub continuacoes: u32,
    pub fim: Fim,
    /// A pontuação do trabalho (o do turno, ou o da corrente que fechou
    /// nele), num fim por Stop (decisão 0074).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pontuacao: Option<Pontuacao>,
    /// O nível pela pontuação, antes dos tetos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nivel_calculado: Option<Nivel>,
    /// O nível da festa.
    pub nivel: Option<Nivel>,
    /// A última reação tocada por este turno.
    pub reacao: Option<&'static str>,
    /// A corrente: o turno entrou numa aberta, ou ela fechou nele (com a
    /// soma de todos nos componentes acima).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrente: Option<ResumoCorrente>,
    /// O que mexeu no nível: `maquina` (teto T1, discreto), `modo` (o
    /// `celebracao.modo`), `intervalo_t3` (outro T3 há pouco).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub teto: Vec<&'static str>,
    /// (teste, sid, turno) inteiros, para trocar o registro de um turno que
    /// reabriu; nunca vai para o `/v1/estado`.
    #[serde(skip)]
    chave: (bool, String, Option<String>),
}

/// Estado de uma sessão para o `/v1/estado.sessoes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoSessao {
    Parada,
    Pensando,
    Trabalhando,
    Esperando,
    Compactando,
    Erro,
    /// O turno acabou no limite de uso (`StopFailure` com `rate_limit`;
    /// decisão 0076).
    Cansado,
}

/// O que uma sessão pede ao Renan (decisão 0057), do menos ao mais urgente:
/// o clique no pet vai ao mais urgente primeiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoAviso {
    /// O Claude terminou o turno (o Stop, depois da acomodação).
    Pronto,
    /// O turno acabou num erro da API (o StopFailure).
    Erro,
    /// O Claude precisa de você: uma permissão, uma pergunta, o plano.
    Esperando,
}

impl TipoAviso {
    /// Em palavras, para o balão.
    pub fn nome(self) -> &'static str {
        match self {
            TipoAviso::Pronto => "pronto",
            TipoAviso::Erro => "erro",
            TipoAviso::Esperando => "esperando você",
        }
    }
}

/// O que a sessão espera do Renan (decisão 0075), do menos ao mais forte: um
/// gatilho novo do mesmo diálogo só sobe o tipo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoEspera {
    /// Uma permissão (`PermissionRequest`, `permission_prompt`, …).
    Permissao,
    /// Um formulário de um servidor MCP (`elicitation_dialog`).
    Elicitacao,
    /// O plano para aprovar (`ExitPlanMode`).
    Plano,
    /// Uma pergunta (`AskUserQuestion`).
    Pergunta,
}

/// O tipo de espera que um evento dispara, se dispara (decisão 0075). O
/// `PermissionRequest` da pergunta e do plano é a pergunta e o plano.
pub fn espera_do_evento(ev: &Evento) -> Option<TipoEspera> {
    match ev.e.as_str() {
        "PreToolUse" | "PermissionRequest" => Some(match ev.tool.as_deref() {
            Some("AskUserQuestion") => TipoEspera::Pergunta,
            Some("ExitPlanMode") => TipoEspera::Plano,
            _ => TipoEspera::Permissao,
        }),
        "Notification" => match ev.nt.as_deref() {
            Some("elicitation_dialog" | "elicitation_url_dialog") => Some(TipoEspera::Elicitacao),
            Some("idle_prompt") | None => None,
            Some(_) => Some(TipoEspera::Permissao),
        },
        _ => None,
    }
}

/// O aviso de uma sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Aviso {
    pub tipo: TipoAviso,
    /// O que ela espera, num aviso de espera (decisão 0075).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub espera: Option<TipoEspera>,
    /// Desde quando (ms desde 1970).
    pub desde_ms: u64,
    /// Desde quando no relógio monotônico (os prazos).
    #[serde(skip)]
    pub desde_mono: u64,
}

/// Uma sessão com aviso, para o clique (decisão 0057).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pendencia {
    /// (teste, sid) inteiros, como a chave das sessões.
    pub chave: (bool, String),
    pub sid8: String,
    pub proj: Option<String>,
    pub aviso: Aviso,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Contadores {
    pub eventos: u64,
    pub turnos: u64,
    pub ferramentas: u64,
    pub reacoes: u64,
}

/// Uma sessão no `/v1/estado.sessoes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResumoSessao {
    pub sid8: String,
    pub proj: Option<String>,
    pub ent: Option<String>,
    pub teste: bool,
    pub estado: EstadoSessao,
    /// Desde quando a sessão está nesse estado (ms desde 1970): o "há
    /// quanto tempo" do balão das sessões (M4).
    pub estado_desde_ms: u64,
    pub turno_aberto: bool,
    pub acomodando: bool,
    /// Hora do último evento (ms desde 1970).
    pub ultimo_evento_ms: u64,
    pub contadores: Contadores,
    /// O que a sessão pede ao Renan, se pede (decisão 0057).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aviso: Option<Aviso>,
    /// A corrente de agentes aberta, se há uma (o selo "…"; decisão 0073).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrente: Option<ResumoCorrente>,
    /// Quantos agendamentos o último Stop listou (`crn`), se o hook disse.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agendamentos: Option<u64>,
    /// A janela do terminal da sessão, como o Motor a casou (decisão 0055);
    /// o cérebro não sabe de janelas e deixa vazio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub janela: Option<crate::motor::janelas::ResumoJanela>,
    /// (teste, sid) inteiros, para o Motor casar a janela; nunca vai para o
    /// `/v1/estado`.
    #[serde(skip)]
    pub chave: (bool, String),
}

/// O que o cérebro publica no `/v1/estado`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Resumo {
    pub sessoes: Vec<ResumoSessao>,
    pub ultima_reacao: Option<Reacao>,
    /// Do mais novo ao mais velho.
    pub turnos: Vec<RegistroTurno>,
    /// Eventos que não contaram, por motivo (`origem:sdk-cli`, `sem_sid`,
    /// `stop_repetido`, …).
    pub ignorados: BTreeMap<String, u64>,
    pub origens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StopPendente {
    /// Hora do Stop (parede).
    ts_ms: u64,
    /// Fim da acomodação (monotônico).
    prazo_mono: u64,
    sha: bool,
    bg: Option<u64>,
    /// Os ids das tarefas de agente em voo no Stop (decisão 0073).
    agentes: Vec<String>,
}

impl StopPendente {
    fn do_evento(ev: &Evento, ts_ms: u64, prazo_mono: u64) -> StopPendente {
        StopPendente {
            ts_ms,
            prazo_mono,
            sha: ev.sha,
            bg: ev.bg,
            agentes: agentes_em_voo(ev),
        }
    }
}

/// As tarefas de agente em voo num Stop: os ids do `bgi` cujo tipo no `bgt`
/// é de [`TAREFAS_DE_AGENTE`] (as listas andam alinhadas; decisão 0019).
fn agentes_em_voo(ev: &Evento) -> Vec<String> {
    ev.bgt
        .iter()
        .zip(&ev.bgi)
        .filter(|(tipo, _)| TAREFAS_DE_AGENTE.contains(&tipo.as_str()))
        .map(|(_, id)| id.clone())
        .take(AGENTES_POR_CORRENTE)
        .collect()
}

/// A festa de um turno que fechou por um Stop: um turno que reabre a leva
/// junto, para só reagir de novo se o nível subir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Festa {
    nivel: Nivel,
    reacao: Option<&'static str>,
}

/// O trabalho de um turno, ou de uma corrente inteira.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Componentes {
    trabalho: u32,
    outras: u32,
    subagentes: u32,
    falhas: u32,
    de_agentes: u32,
    /// Até [`ARQUIVOS_POR_TURNO`].
    arquivos: BTreeSet<String>,
    /// Arquivos novos que chegaram com o conjunto cheio.
    arquivos_a_mais: u32,
    dur_ms: u64,
    /// O tempo das ferramentas que lançam subagente (fora do `min_ativos`).
    dur_lancamento_ms: u64,
}

impl Componentes {
    /// A pontuação do trabalho com `pesos` (decisão 0074).
    fn pontuar(&self, pesos: &Pesos) -> Pontuacao {
        let ativo_ms = self.dur_ms.saturating_sub(self.dur_lancamento_ms);
        let min_ativos = ativo_ms as f64 / 60_000.0;
        let minutos = pesos.por_minuto * min_ativos;
        let trabalho = pesos.por_ferramenta_de_trabalho * f64::from(self.trabalho);
        let outras = pesos.por_outra_ferramenta * f64::from(self.outras);
        let arquivos = pesos.por_arquivo * f64::from(self.arquivos());
        let subagentes = pesos.por_subagente * f64::from(self.subagentes);
        let total = (minutos + trabalho + outras + arquivos + subagentes).min(pesos.teto);
        Pontuacao {
            min_ativos: duas_casas(min_ativos),
            total: duas_casas(total),
            minutos: duas_casas(minutos),
            trabalho: duas_casas(trabalho),
            outras: duas_casas(outras),
            arquivos: duas_casas(arquivos),
            subagentes: duas_casas(subagentes),
        }
    }

    /// O nível pela pontuação: T0 sem trabalho nenhum; senão pelos limites.
    fn nivel_pela(&self, pontuacao: &Pontuacao, pesos: &Pesos) -> Nivel {
        if self.nivel() == Nivel::T0 {
            Nivel::T0
        } else if pontuacao.total >= pesos.t3 {
            Nivel::T3
        } else if pontuacao.total >= pesos.t2 {
            Nivel::T2
        } else {
            Nivel::T1
        }
    }

    fn arquivos(&self) -> u32 {
        u32::try_from(self.arquivos.len())
            .unwrap_or(u32::MAX)
            .saturating_add(self.arquivos_a_mais)
    }

    /// T0 sem ferramenta de trabalho, sem subagente e sem arquivo editado.
    fn nivel(&self) -> Nivel {
        if self.trabalho == 0 && self.subagentes == 0 && self.arquivos() == 0 {
            Nivel::T0
        } else {
            Nivel::T1
        }
    }

    /// Alguma ferramenta ou subagente já contou.
    fn andou(&self) -> bool {
        self.trabalho > 0 || self.outras > 0 || self.subagentes > 0
    }

    fn guardar_arquivo(&mut self, arq: &str) {
        if self.arquivos.contains(arq) {
            return;
        }
        if self.arquivos.len() < ARQUIVOS_POR_TURNO {
            self.arquivos.insert(arq.to_owned());
        } else {
            self.arquivos_a_mais = self.arquivos_a_mais.saturating_add(1);
        }
    }

    fn contar_ferramenta(&mut self, ev: &Evento, falhou: bool) {
        let trabalho = ev
            .tool
            .as_deref()
            .is_some_and(|t| FERRAMENTAS_DE_TRABALHO.contains(&t));
        if trabalho {
            self.trabalho = self.trabalho.saturating_add(1);
        } else {
            self.outras = self.outras.saturating_add(1);
        }
        if falhou {
            self.falhas = self.falhas.saturating_add(1);
        }
        if ev.agente {
            self.de_agentes = self.de_agentes.saturating_add(1);
        }
        if !falhou && let Some(arq) = &ev.arq {
            self.guardar_arquivo(arq);
        }
        let dur = ev.dur.unwrap_or(0);
        self.dur_ms = self.dur_ms.saturating_add(dur);
        if ev
            .tool
            .as_deref()
            .is_some_and(|t| FERRAMENTAS_DE_LANCAMENTO.contains(&t))
        {
            self.dur_lancamento_ms = self.dur_lancamento_ms.saturating_add(dur);
        }
    }

    /// Soma outro trabalho a este (a corrente juntando os turnos).
    fn somar(&mut self, outro: &Componentes) {
        self.trabalho = self.trabalho.saturating_add(outro.trabalho);
        self.outras = self.outras.saturating_add(outro.outras);
        self.subagentes = self.subagentes.saturating_add(outro.subagentes);
        self.falhas = self.falhas.saturating_add(outro.falhas);
        self.de_agentes = self.de_agentes.saturating_add(outro.de_agentes);
        self.dur_ms = self.dur_ms.saturating_add(outro.dur_ms);
        self.dur_lancamento_ms = self
            .dur_lancamento_ms
            .saturating_add(outro.dur_lancamento_ms);
        for arq in &outro.arquivos {
            self.guardar_arquivo(arq);
        }
        self.arquivos_a_mais = self.arquivos_a_mais.saturating_add(outro.arquivos_a_mais);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Turno {
    id: Option<String>,
    t0_ms: u64,
    implicito: bool,
    src: Option<String>,
    origem: OrigemTurno,
    comp: Componentes,
    stop: Option<StopPendente>,
    /// Já fechou por um Stop e reabriu: a festa de então.
    festa: Option<Festa>,
    continuacoes: u32,
}

impl Turno {
    fn novo(
        id: Option<String>,
        t0_ms: u64,
        implicito: bool,
        src: Option<String>,
        origem: OrigemTurno,
    ) -> Turno {
        Turno {
            id,
            t0_ms,
            implicito,
            src,
            origem,
            comp: Componentes::default(),
            stop: None,
            festa: None,
            continuacoes: 0,
        }
    }
}

/// A corrente de agentes de uma sessão (decisão 0073).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Corrente {
    /// O t0 do primeiro turno.
    t0_ms: u64,
    /// Nasceu de um turno de máquina sem corrente: a festa dela tem o teto
    /// de máquina.
    raiz_maquina: bool,
    /// Os turnos que entraram, cada um com o trabalho dele (um turno que
    /// reabre e fecha de novo troca a entrada dele).
    membros: Vec<(Option<String>, Componentes)>,
    /// Os turnos que passaram de [`MEMBROS_POR_CORRENTE`], somados.
    excedente: Componentes,
    /// O trabalho dos agentes depois que o turno deles fechou.
    fundo: Componentes,
    turnos: u32,
    /// As tarefas de agente em voo, pelo último Stop.
    agentes: BTreeSet<String>,
    /// O último evento da corrente (monotônico), para a expiração.
    ultimo_mono: u64,
}

impl Corrente {
    fn nova(t0_ms: u64, raiz_maquina: bool, agora_mono: u64) -> Corrente {
        Corrente {
            t0_ms,
            raiz_maquina,
            membros: Vec::new(),
            excedente: Componentes::default(),
            fundo: Componentes::default(),
            turnos: 0,
            agentes: BTreeSet::new(),
            ultimo_mono: agora_mono,
        }
    }

    fn guardar_membro(&mut self, id: Option<String>, comp: Componentes) {
        if id.is_some()
            && let Some(entrada) = self.membros.iter_mut().find(|(m, _)| *m == id)
        {
            entrada.1 = comp;
            return;
        }
        self.turnos = self.turnos.saturating_add(1);
        if self.membros.len() < MEMBROS_POR_CORRENTE {
            self.membros.push((id, comp));
        } else {
            self.excedente.somar(&comp);
        }
    }

    fn tem_turno(&self, id: &str) -> bool {
        self.membros.iter().any(|(m, _)| m.as_deref() == Some(id))
    }

    fn total(&self) -> Componentes {
        let mut total = self.excedente.clone();
        for (_, comp) in &self.membros {
            total.somar(comp);
        }
        total.somar(&self.fundo);
        total
    }
}

/// O nível com os tetos (decisões 0073 e 0074): o turno de máquina segura
/// em T1 (e a festa fica discreta), o modo `discreta` também, e o
/// `sempre_grande` leva todo nível acima do T0 ao T3. Devolve o nível e o que
/// mexeu nele.
fn nivel_com_teto(
    calculado: Nivel,
    maquina: bool,
    modo: ModoCelebracao,
) -> (Nivel, Vec<&'static str>) {
    let mut nivel = calculado;
    let mut teto = Vec::new();
    if maquina {
        nivel = nivel.min(Nivel::T1);
        teto.push("maquina");
    }
    match modo {
        ModoCelebracao::Discreta if nivel > Nivel::T1 => {
            nivel = Nivel::T1;
            teto.push("modo");
        }
        ModoCelebracao::SempreGrande if !maquina && nivel > Nivel::T0 && nivel < Nivel::T3 => {
            nivel = Nivel::T3;
            teto.push("modo");
        }
        _ => {}
    }
    (nivel, teto)
}

/// A reação de um nível, segundo o modo de celebração; o T0 de máquina não
/// reage (decisão 0073), e com a celebração desligada nada reage.
fn reacao_do_nivel(nivel: Nivel, modo: ModoCelebracao, maquina: bool) -> Option<&'static str> {
    match (nivel, modo) {
        (_, ModoCelebracao::Desligada) => None,
        (Nivel::T0, _) if maquina => None,
        (nivel, _) => Some(nivel.reacao()),
    }
}

/// Um turno que acabou: o registro e, se comemorou, a reação.
struct Fechamento {
    registro: RegistroTurno,
    reacao: Option<Reacao>,
    /// O turno tinha reaberto: o registro troca o que ele já tinha.
    substitui: bool,
    /// O nível da festa de antes, num turno que reabriu.
    nivel_antes: Option<Nivel>,
    /// O fim merece o pronto (decisão 0057): um Stop que fecha um turno
    /// digitado, ou a corrente que nasceu de um.
    pronto: bool,
}

/// Um turno trocado por um turno novo sem ter visto o próprio Stop.
#[derive(Debug, Clone)]
struct Trocado {
    turno: Turno,
    /// Hora (parede) do evento que abriu o turno novo: o fim deste, se o
    /// Stop não vier.
    troca_ms: u64,
    /// Fim da espera pelo Stop (monotônico).
    prazo_mono: u64,
}

/// O último turno fechado por um Stop, que uma continuação pode reabrir.
#[derive(Debug, Clone)]
struct Comemorado {
    turno: Turno,
    /// Hora (parede) do Stop.
    stop_ms: u64,
}

/// De que turno é um evento.
enum Alvo {
    /// O turno aberto (ou o comemorado, que acabou de reabrir).
    Aberto,
    /// O turno trocado, à espera do Stop.
    Trocado,
    /// Um turno que já fechou.
    Fechado,
    /// Nenhum: o evento abre um turno.
    Novo,
}

#[derive(Debug, Clone)]
struct Sessao {
    sid: String,
    teste: bool,
    proj: Option<String>,
    ent: Option<String>,
    estado: EstadoSessao,
    /// Hora (parede) do evento que pôs a sessão no estado de agora.
    estado_desde: u64,
    /// O mesmo, no relógio monotônico (os prazos do estado).
    estado_desde_mono: u64,
    turno: Option<Turno>,
    trocado: Option<Trocado>,
    comemorado: Option<Comemorado>,
    /// A corrente de agentes aberta (decisão 0073).
    corrente: Option<Corrente>,
    /// Os agendamentos do último Stop (`crn`), se o hook disse.
    agendamentos: Option<u64>,
    /// Turnos já fechados (o `prompt_id`), mais novo no fim.
    fechados: VecDeque<String>,
    /// Subagente → turno em que nasceu.
    agentes: VecDeque<(String, Option<String>)>,
    ultimo_mono: u64,
    /// Hora do último evento (parede), para o `/v1/estado`.
    ultimo_parede: u64,
    contadores: Contadores,
    /// O espaço de aviso da sessão (decisão 0057).
    aviso: Option<Aviso>,
}

impl Sessao {
    fn nova(sid: String, teste: bool, agora: Agora) -> Sessao {
        Sessao {
            sid,
            teste,
            proj: None,
            ent: None,
            estado: EstadoSessao::Parada,
            estado_desde: agora.parede_ms,
            estado_desde_mono: agora.mono_ms,
            turno: None,
            trocado: None,
            comemorado: None,
            corrente: None,
            agendamentos: None,
            fechados: VecDeque::new(),
            agentes: VecDeque::new(),
            ultimo_mono: agora.mono_ms,
            ultimo_parede: agora.parede_ms,
            contadores: Contadores::default(),
            aviso: None,
        }
    }

    fn fechado(&self, turno: &str) -> bool {
        self.fechados.iter().any(|t| t == turno)
    }

    fn lembrar_fechado(&mut self, turno: &str) {
        if self.fechado(turno) {
            return;
        }
        if self.fechados.len() == FECHADOS_POR_SESSAO {
            self.fechados.pop_front();
        }
        self.fechados.push_back(turno.to_owned());
    }

    /// De que turno é um evento da thread principal com `id` e hora `t`. Um
    /// evento de trabalho (`continua`) com hora depois do Stop do turno
    /// comemorado reabre esse turno.
    fn alvo(&mut self, id: Option<&str>, t: u64, continua: bool) -> Alvo {
        if let Some(id) = id
            && self
                .trocado
                .as_ref()
                .is_some_and(|x| x.turno.id.as_deref() == Some(id))
        {
            return Alvo::Trocado;
        }
        if let Some(aberto) = &self.turno
            && (id.is_none() || aberto.id.as_deref() == id)
        {
            return Alvo::Aberto;
        }
        let Some(id) = id else {
            return Alvo::Novo;
        };
        if continua
            && self.turno.is_none()
            && self
                .comemorado
                .as_ref()
                .is_some_and(|c| c.turno.id.as_deref() == Some(id) && t > c.stop_ms)
        {
            self.reabrir();
            return Alvo::Aberto;
        }
        if self.fechado(id) {
            Alvo::Fechado
        } else {
            Alvo::Novo
        }
    }

    /// Uma continuação: o turno comemorado volta a ser o aberto, com os
    /// contadores e a festa de antes.
    fn reabrir(&mut self) {
        let Some(Comemorado { mut turno, .. }) = self.comemorado.take() else {
            return;
        };
        if let Some(id) = &turno.id {
            self.fechados.retain(|f| f != id);
        }
        turno.continuacoes = turno.continuacoes.saturating_add(1);
        self.turno = Some(turno);
    }

    /// Abre um turno implícito (o evento é de um turno que ninguém abriu; o
    /// `UserPromptSubmit` o assume em seguida): o aberto acaba
    /// ([`Self::encerrar_aberto`]) e o comemorado não reabre mais.
    fn abrir(
        &mut self,
        id: Option<&str>,
        t: u64,
        agora: Agora,
        cfg: &ConfigCerebro,
        fechamentos: &mut Vec<Fechamento>,
    ) -> &mut Turno {
        self.encerrar_aberto(t, agora, cfg, fechamentos);
        self.comemorado = None;
        self.turno.insert(Turno::novo(
            id.map(str::to_owned),
            t,
            true,
            None,
            OrigemTurno::Digitado,
        ))
    }

    /// Um turno novo vai começar: o aberto acaba. Com Stop pendente comemora
    /// na hora (fim = hora do Stop); sem Stop vira o trocado e espera o Stop
    /// atrasado por [`ACOMODACAO_MS`] (um trocado de antes fecha sem festa).
    fn encerrar_aberto(
        &mut self,
        t: u64,
        agora: Agora,
        cfg: &ConfigCerebro,
        fechamentos: &mut Vec<Fechamento>,
    ) {
        let Some(aberto) = self.turno.take() else {
            return;
        };
        if let Some(stop) = &aberto.stop {
            let fim_ms = stop.ts_ms;
            fechamentos.push(self.fechar_turno(aberto, Fim::Stop, fim_ms, agora, cfg, true));
            return;
        }
        if let Some(velho) = self.trocado.take() {
            fechamentos.push(self.fechar_trocado(velho, agora, cfg));
        }
        self.trocado = Some(Trocado {
            turno: aberto,
            troca_ms: t,
            prazo_mono: agora.mono_ms + ACOMODACAO_MS,
        });
    }

    /// O Stop do trocado não chegou na espera: fecha sem festa, com a hora
    /// da troca.
    fn fechar_trocado(
        &mut self,
        trocado: Trocado,
        agora: Agora,
        cfg: &ConfigCerebro,
    ) -> Fechamento {
        self.fechar_turno(
            trocado.turno,
            Fim::Substituido,
            trocado.troca_ms,
            agora,
            cfg,
            false,
        )
    }

    /// Fecha o turno aberto.
    fn fechar(
        &mut self,
        fim: Fim,
        fim_ms: u64,
        agora: Agora,
        cfg: &ConfigCerebro,
    ) -> Option<Fechamento> {
        let turno = self.turno.take()?;
        Some(self.fechar_turno(turno, fim, fim_ms, agora, cfg, true))
    }

    /// Fecha um turno. Só `Fim::Stop` comemora: na primeira vez, pelo nível
    /// e pelo modo; num turno que reabriu, só se o nível subiu, e o registro
    /// troca o de antes. Com a corrente aberta, ou com agente em voo no Stop,
    /// o Stop é da corrente (decisão 0073): com agente em voo, o turno entra
    /// nela sem festa; sem agente em voo, ela fecha aqui com a soma. Fechado
    /// por um Stop, o turno fica comemorado se `reabrivel` (o trocado não: já
    /// há um turno novo depois dele).
    fn fechar_turno(
        &mut self,
        mut turno: Turno,
        fim: Fim,
        fim_ms: u64,
        agora: Agora,
        cfg: &ConfigCerebro,
        reabrivel: bool,
    ) -> Fechamento {
        if let Some(id) = &turno.id {
            self.lembrar_fechado(id);
        }
        let stop = turno.stop.take();
        let mut comp = turno.comp.clone();
        let mut t0_ms = turno.t0_ms;
        let mut maquina = turno.origem.maquina();
        let mut corrente = None;
        let mut fim_da_corrente = false;
        if fim == Fim::Stop {
            let em_voo = stop.as_ref().map(|s| s.agentes.clone()).unwrap_or_default();
            if self.corrente.is_some() || !em_voo.is_empty() {
                let c = self
                    .corrente
                    .get_or_insert_with(|| Corrente::nova(turno.t0_ms, maquina, agora.mono_ms));
                c.ultimo_mono = agora.mono_ms;
                c.guardar_membro(turno.id.clone(), turno.comp.clone());
                c.agentes = em_voo.iter().cloned().collect();
                if em_voo.is_empty() {
                    let c = self.corrente.take().expect("a corrente de agora");
                    comp = c.total();
                    t0_ms = c.t0_ms;
                    maquina = c.raiz_maquina;
                    corrente = Some(ResumoCorrente {
                        aberta: false,
                        turnos: c.turnos,
                    });
                    fim_da_corrente = true;
                } else {
                    corrente = Some(ResumoCorrente {
                        aberta: true,
                        turnos: c.turnos,
                    });
                }
            }
        }
        // Com a corrente aberta, o Stop não festeja (nem o pronto).
        let festeja = fim == Fim::Stop && corrente.is_none_or(|c| !c.aberta);
        let pontuacao = (fim == Fim::Stop).then(|| comp.pontuar(&cfg.pesos));
        let nivel_calculado = pontuacao
            .as_ref()
            .filter(|_| festeja)
            .map(|p| comp.nivel_pela(p, &cfg.pesos));
        let (nivel_agora, teto) = match nivel_calculado {
            Some(calculado) => {
                let (nivel, teto) = nivel_com_teto(calculado, maquina, cfg.modo);
                (Some(nivel), teto)
            }
            None => (None, Vec::new()),
        };
        let antes = turno.festa;
        let subiu = match (nivel_agora, antes) {
            (Some(_), None) => true,
            (Some(nivel), Some(festa)) => nivel > festa.nivel,
            (None, _) => false,
        };
        let nome = nivel_agora
            .filter(|_| subiu)
            .and_then(|nivel| reacao_do_nivel(nivel, cfg.modo, maquina));
        let reacao = nome.map(|nome| Reacao {
            nome,
            sid8: evento::curto(&self.sid),
            proj: self.proj.clone(),
            ts: agora.parede_ms,
            nivel: nivel_agora,
            teste: self.teste,
            discreta: maquina,
        });
        let nivel = nivel_agora.max(antes.map(|festa| festa.nivel));
        let reacao_do_turno = nome.or(antes.and_then(|festa| festa.reacao));
        let reaberto = turno.continuacoes > 0;
        if !reaberto {
            self.contadores.turnos += 1;
        }
        if reacao.is_some() {
            self.contadores.reacoes += 1;
        }
        let registro = RegistroTurno {
            sid8: evento::curto(&self.sid),
            turno8: turno.id.as_deref().map(evento::curto),
            proj: self.proj.clone(),
            teste: self.teste,
            implicito: turno.implicito,
            src: turno.src.clone(),
            origem: turno.origem,
            t0_ms,
            fim_ms,
            relogio_ms: fim_ms.saturating_sub(t0_ms),
            trabalho: comp.trabalho,
            outras: comp.outras,
            arquivos: comp.arquivos(),
            subagentes: comp.subagentes,
            falhas: comp.falhas,
            de_agentes: comp.de_agentes,
            dur_ms: comp.dur_ms,
            bg: stop.as_ref().and_then(|s| s.bg),
            sha: stop.as_ref().is_some_and(|s| s.sha),
            continuacoes: turno.continuacoes,
            fim,
            pontuacao,
            nivel_calculado,
            nivel,
            reacao: reacao_do_turno,
            corrente,
            teto,
            chave: (self.teste, self.sid.clone(), turno.id.clone()),
        };
        if fim == Fim::Stop && reabrivel {
            turno.festa = nivel.map(|nivel| Festa {
                nivel,
                reacao: reacao_do_turno,
            });
            self.comemorado = Some(Comemorado {
                turno,
                stop_ms: fim_ms,
            });
        }
        Fechamento {
            registro,
            reacao,
            substitui: reaberto,
            nivel_antes: antes.map(|festa| festa.nivel),
            // O fim do pedido do Renan: um turno digitado que não ficou numa
            // corrente aberta, ou a corrente que nasceu de um.
            pronto: festeja && !maquina && (fim_da_corrente || corrente.is_none()),
        }
    }

    /// A corrente fecha sem festa (expirou, ou a sessão acabou): o registro
    /// dela, com a soma.
    fn fechar_corrente(&mut self, fim: Fim, fim_ms: u64) -> Option<Fechamento> {
        let c = self.corrente.take()?;
        let comp = c.total();
        Some(Fechamento {
            registro: RegistroTurno {
                sid8: evento::curto(&self.sid),
                turno8: None,
                proj: self.proj.clone(),
                teste: self.teste,
                implicito: false,
                src: None,
                origem: if c.raiz_maquina {
                    OrigemTurno::Notificacao
                } else {
                    OrigemTurno::Digitado
                },
                t0_ms: c.t0_ms,
                fim_ms,
                relogio_ms: fim_ms.saturating_sub(c.t0_ms),
                trabalho: comp.trabalho,
                outras: comp.outras,
                arquivos: comp.arquivos(),
                subagentes: comp.subagentes,
                falhas: comp.falhas,
                de_agentes: comp.de_agentes,
                dur_ms: comp.dur_ms,
                bg: None,
                sha: false,
                continuacoes: 0,
                fim,
                pontuacao: None,
                nivel_calculado: None,
                nivel: None,
                reacao: None,
                corrente: Some(ResumoCorrente {
                    aberta: false,
                    turnos: c.turnos,
                }),
                teto: Vec::new(),
                chave: (self.teste, self.sid.clone(), None),
            },
            reacao: None,
            substitui: false,
            nivel_antes: None,
            pronto: false,
        })
    }

    /// Turno de uma ferramenta de subagente: o turno em que ele nasceu, se
    /// ainda estiver aberto ou trocado; senão o do próprio evento; senão o
    /// aberto.
    fn turno_do_agente(&mut self, aid: Option<&str>, id: Option<&str>) -> Option<&mut Turno> {
        let alvo = self.nascimento(aid).or_else(|| id.map(str::to_owned));
        let Some(alvo) = alvo else {
            return self.turno.as_mut();
        };
        let eh_dele = |turno: &Turno| turno.id.as_deref() == Some(alvo.as_str());
        if self.turno.as_ref().is_some_and(eh_dele) {
            return self.turno.as_mut();
        }
        self.trocado
            .as_mut()
            .map(|x| &mut x.turno)
            .filter(|turno| eh_dele(turno))
    }

    /// O turno em que o subagente `aid` nasceu, se ele é conhecido.
    fn nascimento(&self, aid: Option<&str>) -> Option<String> {
        let aid = aid?;
        self.agentes
            .iter()
            .find(|(a, _)| a == aid)
            .and_then(|(_, t)| t.clone())
    }

    /// O subagente `aid` já foi visto (o `SubagentStart` dele ou o `bgi` de
    /// um Stop).
    fn conhece_agente(&self, aid: &str) -> bool {
        self.agentes.iter().any(|(a, _)| a == aid)
            || self
                .corrente
                .as_ref()
                .is_some_and(|c| c.agentes.contains(aid))
    }

    /// A corrente aberta, se o trabalho do agente `aid` (do turno `id`) é
    /// dela: o agente está em voo nela, ou nasceu num turno dela.
    fn corrente_do_agente(&mut self, aid: Option<&str>, id: Option<&str>) -> Option<&mut Corrente> {
        let nascimento = self.nascimento(aid);
        let c = self.corrente.as_mut()?;
        let dela = aid.is_some_and(|a| c.agentes.contains(a))
            || nascimento.as_deref().is_some_and(|t| c.tem_turno(t))
            || id.is_some_and(|t| c.tem_turno(t));
        dela.then_some(c)
    }

    fn lembrar_agente(&mut self, aid: &str, turno: Option<String>) {
        if self.agentes.iter().any(|(a, _)| a == aid) {
            return;
        }
        if self.agentes.len() == AGENTES_POR_SESSAO {
            self.agentes.pop_front();
        }
        self.agentes.push_back((aid.to_owned(), turno));
    }

    /// Quando o estado de agora volta sozinho a parado (decisão 0076):
    /// trabalhando, pensando e compactando 5 min depois do último evento da
    /// sessão; erro e cansado 60 s depois de entrar.
    fn prazo_do_estado(&self) -> Option<u64> {
        match self.estado {
            EstadoSessao::Pensando | EstadoSessao::Trabalhando | EstadoSessao::Compactando => {
                Some(self.ultimo_mono + ATIVA_SEM_EVENTO_MS)
            }
            EstadoSessao::Erro | EstadoSessao::Cansado => {
                Some(self.estado_desde_mono + ERRO_NA_TELA_MS)
            }
            EstadoSessao::Parada | EstadoSessao::Esperando => None,
        }
    }

    fn resumo_da_corrente(&self) -> Option<ResumoCorrente> {
        self.corrente.as_ref().map(|c| ResumoCorrente {
            aberta: true,
            turnos: c.turnos,
        })
    }
}

/// O cérebro: sessões, turnos e a última reação.
#[derive(Debug, Clone)]
pub struct Cerebro {
    config: ConfigCerebro,
    /// (teste, sid) → sessão: eventos de teste nunca tocam sessões reais.
    sessoes: BTreeMap<(bool, String), Sessao>,
    turnos: VecDeque<RegistroTurno>,
    /// Os turnos fechados desde que o Motor tirou da última vez (as
    /// intenções; decisão 0077).
    fechados_a_tirar: VecDeque<RegistroTurno>,
    ultima_reacao: Option<Reacao>,
    ignorados: BTreeMap<String, u64>,
    /// O último T3 de cada mundo (o real, o de teste), no relógio
    /// monotônico (decisão 0074).
    ultimo_t3: [Option<u64>; 2],
}

/// Hora do evento: o `ts` do hook quando plausível (até 6 h da chegada);
/// senão a hora de chegada.
pub fn hora_do_evento(ts: Option<u64>, recebido_ms: u64) -> u64 {
    match ts {
        Some(ts) if ts.abs_diff(recebido_ms) <= JANELA_TS_MS => ts,
        _ => recebido_ms,
    }
}

/// Quando um aviso some sozinho: o pronto e o erro em [`VIDA_AVISO_MS`]; o
/// "esperando você", nunca (só um evento da sessão ou visto).
fn prazo_do_aviso(aviso: Aviso) -> Option<u64> {
    (aviso.tipo != TipoAviso::Esperando).then_some(aviso.desde_mono + VIDA_AVISO_MS)
}

/// Eventos que mostram o turno andando: cancelam a acomodação quando vêm
/// depois do Stop e reabrem o turno comemorado.
fn continua_o_turno(e: &str) -> bool {
    matches!(
        e,
        "PreToolUse"
            | "PostToolUse"
            | "PostToolUseFailure"
            | "PermissionRequest"
            | "SubagentStart"
            | "PreCompact"
            | "PostCompact"
    )
}

impl Cerebro {
    pub fn novo(config: ConfigCerebro) -> Cerebro {
        Cerebro {
            config,
            sessoes: BTreeMap::new(),
            turnos: VecDeque::new(),
            fechados_a_tirar: VecDeque::new(),
            ultima_reacao: None,
            ignorados: BTreeMap::new(),
            ultimo_t3: [None; 2],
        }
    }

    pub fn config(&self) -> &ConfigCerebro {
        &self.config
    }

    /// Troca a configuração: o config relido a cada aprovação de personagem
    /// (decisão 0029) vale também aqui (decisão 0030). Uma sessão de origem
    /// que deixou de contar sai na hora, sem reação: os eventos dela seriam
    /// ignorados dali em diante e ela só sumiria em 12 h. As outras seguem
    /// como estavam.
    pub fn reconfigurar(&mut self, config: ConfigCerebro) {
        self.sessoes.retain(|_, sessao| {
            let origem = sessao.ent.as_deref().unwrap_or("desconhecida");
            config.origens.iter().any(|o| o == origem)
        });
        self.config = config;
    }

    fn ignorar(&mut self, motivo: impl Into<String>) {
        let mut motivo = motivo.into();
        if self.ignorados.len() >= MAX_MOTIVOS && !self.ignorados.contains_key(&motivo) {
            motivo = "outros".into();
        }
        *self.ignorados.entry(motivo).or_default() += 1;
    }

    /// Guarda os turnos fechados e devolve as reações deles. O registro de
    /// um turno que reabriu troca o de antes: um registro por turno.
    fn registrar(&mut self, fechamentos: Vec<Fechamento>, reacoes: &mut Vec<Reacao>, agora: Agora) {
        for mut f in fechamentos {
            self.intervalo_do_t3(&mut f, agora);
            if f.substitui {
                self.turnos.retain(|r| r.chave != f.registro.chave);
            }
            if self.turnos.len() == TURNOS_GUARDADOS {
                self.turnos.pop_front();
            }
            if self.fechados_a_tirar.len() == FECHADOS_A_TIRAR {
                self.fechados_a_tirar.pop_front();
            }
            self.fechados_a_tirar.push_back(f.registro.clone());
            self.turnos.push_back(f.registro);
            if let Some(reacao) = f.reacao {
                self.reagir(reacao, reacoes);
            }
        }
    }

    /// O T3 no máximo a cada `intervalo_t3_ms` (2 min no `sempre_grande`),
    /// em cada mundo (o real e o de teste; decisão 0074): cedo demais, o
    /// voo grande vira o voo curto, e a festa de um turno que reabriu só toca
    /// se ainda subir.
    fn intervalo_do_t3(&mut self, f: &mut Fechamento, agora: Agora) {
        let Some(reacao) = f.reacao.as_mut() else {
            return;
        };
        if reacao.nivel != Some(Nivel::T3) {
            return;
        }
        let mundo = usize::from(f.registro.teste);
        let intervalo = if self.config.modo == ModoCelebracao::SempreGrande {
            INTERVALO_T3_SEMPRE_GRANDE_MS
        } else {
            self.config.intervalo_t3_ms
        };
        let cedo = self.ultimo_t3[mundo].is_some_and(|ultimo| agora.mono_ms < ultimo + intervalo);
        if !cedo {
            self.ultimo_t3[mundo] = Some(agora.mono_ms);
            return;
        }
        reacao.nivel = Some(Nivel::T2);
        reacao.nome = VOO_CURTO;
        f.registro.teto.push("intervalo_t3");
        let ainda_sobe = f.nivel_antes.is_none_or(|antes| Nivel::T2 > antes);
        let reacao_do_turno = if ainda_sobe {
            Some(VOO_CURTO)
        } else {
            f.reacao = None;
            f.registro.reacao
        };
        f.registro.nivel = f.nivel_antes.max(Some(Nivel::T2));
        f.registro.reacao = reacao_do_turno;
        // A festa guardada para uma continuação é a que tocou.
        let (teste, sid, turno) = &f.registro.chave;
        if let Some(sessao) = self.sessoes.get_mut(&(*teste, sid.clone()))
            && let Some(c) = sessao.comemorado.as_mut()
            && c.turno.id == *turno
            && let Some(festa) = c.turno.festa.as_mut()
        {
            festa.nivel = f.registro.nivel.unwrap_or(Nivel::T2);
            festa.reacao = reacao_do_turno;
        }
    }

    fn reagir(&mut self, reacao: Reacao, reacoes: &mut Vec<Reacao>) {
        self.ultima_reacao = Some(reacao.clone());
        reacoes.push(reacao);
    }

    /// Um evento validado, com a hora em que chegou ao daemon (`agora`: o
    /// relógio da chegada, não o do processamento, para um laço atrasado não
    /// vencer uma acomodação que o próprio evento cancelaria). Devolve as
    /// reações para tocar agora (inclusive as de prazos que venceram antes
    /// dele).
    pub fn receber(&mut self, ev: &Evento, recebido_ms: u64, agora: Agora) -> Vec<Reacao> {
        self.receber_com(ev, recebido_ms, agora, Evidencia::default())
    }

    /// [`Self::receber`], com o que o Motor sabe da hora de um prompt
    /// (decisão 0073): a prova de que o Renan não o digitou, para separar o
    /// tique de um laço.
    pub fn receber_com(
        &mut self,
        ev: &Evento,
        recebido_ms: u64,
        agora: Agora,
        evidencia: Evidencia,
    ) -> Vec<Reacao> {
        let mut reacoes = self.tique(agora);
        let origem = ev.ent.as_deref().unwrap_or("desconhecida");
        if !self.config.origens.iter().any(|o| o == origem) {
            self.ignorar(format!("origem:{origem}"));
            return reacoes;
        }
        let Some(sid) = ev.sid.clone() else {
            self.ignorar("sem_sid");
            return reacoes;
        };
        let t = hora_do_evento(ev.ts, recebido_ms);
        let chave = (ev.teste, sid);
        if ev.e == "SessionEnd" {
            let sai = !ev
                .reason
                .as_deref()
                .is_some_and(|r| FIM_SEM_TCHAU.contains(&r));
            self.encerrar(&chave, t, sai, agora, &mut reacoes);
            return reacoes;
        }
        if !self.sessoes.contains_key(&chave) {
            self.abrir_vaga(ev.teste);
        }
        let cfg = self.config.clone();
        let mut fechamentos = Vec::new();
        let mut ignorado = None;
        let sessao = self
            .sessoes
            .entry(chave.clone())
            .or_insert_with(|| Sessao::nova(chave.1.clone(), chave.0, agora));
        sessao.ultimo_mono = agora.mono_ms;
        sessao.ultimo_parede = t;
        sessao.contadores.eventos += 1;
        if ev.proj.is_some() {
            sessao.proj.clone_from(&ev.proj);
        }
        if ev.ent.is_some() {
            sessao.ent.clone_from(&ev.ent);
        }
        let id = ev.turno.as_deref();
        let estado_antes = sessao.estado;
        let desde_antes = sessao.estado_desde;

        // Um evento de trabalho da thread principal: o SubagentStart vem com
        // o `agent_id` do subagente que nasce, mas quem o lança é a thread
        // principal. Depois do Stop, ele mostra que o turno continua. O de um
        // agente já visto é ele acordando em segundo plano (o shell dele
        // acabou; decisão 0073): não é a thread principal.
        let agente_conhecido =
            ev.e == "SubagentStart" && ev.aid.as_deref().is_some_and(|a| sessao.conhece_agente(a));
        let continua = (!ev.agente || (ev.e == "SubagentStart" && !agente_conhecido))
            && continua_o_turno(&ev.e);
        if continua
            && let Some(turno) = sessao.turno.as_mut()
            && let Some(stop) = turno.stop.as_ref()
            && t > stop.ts_ms
            && (id.is_none() || id == turno.id.as_deref())
        {
            turno.stop = None;
        }

        match ev.e.as_str() {
            "SessionStart" => {
                if sessao.turno.is_none() {
                    sessao.estado = EstadoSessao::Parada;
                }
            }
            "UserPromptSubmit" => {
                // Sem `prompt_id` não há como casar: sempre um turno novo.
                let alvo = match id {
                    None => Alvo::Novo,
                    Some(_) => sessao.alvo(id, t, false),
                };
                match alvo {
                    Alvo::Novo => {
                        // O aberto acaba antes (o Stop pendente comemora na
                        // hora e pode abrir ou fechar a corrente), e só então
                        // a origem do prompt novo é decidida (decisão 0073).
                        let antes = fechamentos.len();
                        sessao.encerrar_aberto(t, agora, &cfg, &mut fechamentos);
                        for f in &fechamentos[antes..] {
                            if f.pronto {
                                sessao.aviso = Some(Aviso {
                                    tipo: TipoAviso::Pronto,
                                    espera: None,
                                    desde_ms: f.registro.fim_ms,
                                    desde_mono: agora.mono_ms,
                                });
                            }
                        }
                        sessao.comemorado = None;
                        let origem = classificar(
                            ev.src.as_deref(),
                            ev.orig.as_deref(),
                            sessao.corrente.is_some(),
                            sessao.agendamentos,
                            evidencia,
                        );
                        sessao.turno = Some(Turno::novo(
                            id.map(str::to_owned),
                            t,
                            false,
                            ev.src.clone(),
                            origem,
                        ));
                        sessao.estado = EstadoSessao::Pensando;
                    }
                    // Aberto por um evento que chegou antes do prompt (hooks
                    // async fora de ordem): é o turno dele.
                    Alvo::Aberto => {
                        let origem = classificar(
                            ev.src.as_deref(),
                            ev.orig.as_deref(),
                            sessao.corrente.is_some(),
                            sessao.agendamentos,
                            evidencia,
                        );
                        match sessao.turno.as_mut() {
                            Some(turno) if turno.implicito => {
                                turno.implicito = false;
                                turno.t0_ms = turno.t0_ms.min(t);
                                turno.src.clone_from(&ev.src);
                                turno.origem = origem;
                                if !turno.comp.andou() && turno.stop.is_none() {
                                    sessao.estado = EstadoSessao::Pensando;
                                }
                            }
                            _ => ignorado = Some("prompt_repetido"),
                        }
                    }
                    Alvo::Trocado => ignorado = Some("prompt_repetido"),
                    Alvo::Fechado => ignorado = Some("turno_fechado"),
                }
            }
            "PostToolUse" | "PostToolUseFailure" => {
                sessao.contadores.ferramentas += 1;
                let falhou = ev.e == "PostToolUseFailure";
                if ev.agente {
                    // A ferramenta de um subagente conta no turno em que ele
                    // nasceu; fechado o turno, na corrente dele (decisão
                    // 0073).
                    if let Some(turno) = sessao.turno_do_agente(ev.aid.as_deref(), id) {
                        turno.comp.contar_ferramenta(ev, falhou);
                    } else if let Some(c) = sessao.corrente_do_agente(ev.aid.as_deref(), id) {
                        c.fundo.contar_ferramenta(ev, falhou);
                        c.ultimo_mono = agora.mono_ms;
                    } else {
                        ignorado = Some("ferramenta_de_agente_fora_do_turno");
                    }
                } else {
                    let interrompeu = falhou && ev.intr;
                    match sessao.alvo(id, t, continua) {
                        Alvo::Trocado => {
                            if let Some(trocado) = sessao.trocado.as_mut() {
                                trocado.turno.comp.contar_ferramenta(ev, falhou);
                            }
                            if interrompeu && let Some(trocado) = sessao.trocado.take() {
                                let turno = trocado.turno;
                                fechamentos.push(sessao.fechar_turno(
                                    turno,
                                    Fim::Interrompido,
                                    t,
                                    agora,
                                    &cfg,
                                    false,
                                ));
                            }
                        }
                        Alvo::Fechado => ignorado = Some("turno_fechado"),
                        alvo => {
                            if matches!(alvo, Alvo::Novo) {
                                sessao.abrir(id, t, agora, &cfg, &mut fechamentos);
                            }
                            if let Some(turno) = sessao.turno.as_mut() {
                                turno.comp.contar_ferramenta(ev, falhou);
                                // Atrasado (antes do Stop pendente): conta,
                                // mas a sessão segue parada.
                                if turno.stop.is_none() {
                                    sessao.estado = EstadoSessao::Trabalhando;
                                }
                            }
                            if interrompeu {
                                fechamentos.extend(sessao.fechar(
                                    Fim::Interrompido,
                                    t,
                                    agora,
                                    &cfg,
                                ));
                                sessao.estado = EstadoSessao::Parada;
                            }
                        }
                    }
                }
            }
            // Um agente já visto acordou em segundo plano: só a corrente dele
            // anda (decisão 0073).
            "SubagentStart" if agente_conhecido => {
                if let Some(c) = sessao.corrente_do_agente(ev.aid.as_deref(), id) {
                    c.ultimo_mono = agora.mono_ms;
                }
            }
            "SubagentStart" => match sessao.alvo(id, t, continua) {
                Alvo::Trocado => {
                    let nascimento = sessao.trocado.as_mut().map(|trocado| {
                        trocado.turno.comp.subagentes =
                            trocado.turno.comp.subagentes.saturating_add(1);
                        trocado.turno.id.clone()
                    });
                    if let (Some(nascimento), Some(aid)) = (nascimento, &ev.aid) {
                        sessao.lembrar_agente(aid, nascimento);
                    }
                }
                Alvo::Fechado => ignorado = Some("turno_fechado"),
                alvo => {
                    if matches!(alvo, Alvo::Novo) {
                        sessao.abrir(id, t, agora, &cfg, &mut fechamentos);
                    }
                    let nascimento = sessao.turno.as_mut().map(|turno| {
                        turno.comp.subagentes = turno.comp.subagentes.saturating_add(1);
                        turno.id.clone()
                    });
                    if let (Some(nascimento), Some(aid)) = (nascimento, &ev.aid) {
                        sessao.lembrar_agente(aid, nascimento);
                    }
                }
            },
            "PreToolUse" | "PermissionRequest" => {
                if !ev.agente {
                    match sessao.alvo(id, t, continua) {
                        Alvo::Fechado => ignorado = Some("turno_fechado"),
                        Alvo::Trocado => {}
                        alvo => {
                            if matches!(alvo, Alvo::Novo) {
                                sessao.abrir(id, t, agora, &cfg, &mut fechamentos);
                            }
                            if sessao.turno.as_ref().is_some_and(|t| t.stop.is_none()) {
                                sessao.estado = EstadoSessao::Esperando;
                            }
                        }
                    }
                }
            }
            "Notification" => match ev.nt.as_deref() {
                Some("idle_prompt") => {
                    if sessao.turno.as_ref().is_some_and(|t| t.stop.is_none()) {
                        fechamentos.extend(sessao.fechar(Fim::Ocioso, t, agora, &cfg));
                    }
                    if sessao.turno.is_none() {
                        sessao.estado = EstadoSessao::Parada;
                    }
                }
                Some(_) => sessao.estado = EstadoSessao::Esperando,
                None => ignorado = Some("notificacao_sem_tipo"),
            },
            "PreCompact" => sessao.estado = EstadoSessao::Compactando,
            "PostCompact" => {
                sessao.estado = if sessao.turno.is_some() {
                    EstadoSessao::Trabalhando
                } else {
                    EstadoSessao::Parada
                };
            }
            "Stop" => match sessao.alvo(id, t, false) {
                // O Stop atrasado do turno trocado: comemora na hora (o turno
                // novo já começou). Se o novo é de máquina, ele não resolve o
                // pronto deste (decisão 0073).
                Alvo::Trocado => {
                    if ev.crn.is_some() {
                        sessao.agendamentos = ev.crn;
                    }
                    if let Some(trocado) = sessao.trocado.take() {
                        let mut turno = trocado.turno;
                        turno.stop = Some(StopPendente::do_evento(ev, t, agora.mono_ms));
                        let f = sessao.fechar_turno(turno, Fim::Stop, t, agora, &cfg, false);
                        let novo_de_maquina =
                            sessao.turno.as_ref().is_some_and(|n| n.origem.maquina());
                        if f.pronto && novo_de_maquina {
                            sessao.aviso = Some(Aviso {
                                tipo: TipoAviso::Pronto,
                                espera: None,
                                desde_ms: t,
                                desde_mono: agora.mono_ms,
                            });
                        }
                        fechamentos.push(f);
                    }
                }
                Alvo::Fechado => ignorado = Some("stop_repetido"),
                alvo => {
                    if matches!(alvo, Alvo::Novo) {
                        sessao.abrir(id, t, agora, &cfg, &mut fechamentos);
                    }
                    if let Some(turno) = sessao.turno.as_mut() {
                        if turno.stop.is_some() {
                            ignorado = Some("stop_repetido");
                        } else {
                            turno.stop = Some(StopPendente::do_evento(
                                ev,
                                t,
                                agora.mono_ms + ACOMODACAO_MS,
                            ));
                            sessao.estado = EstadoSessao::Parada;
                            if ev.crn.is_some() {
                                sessao.agendamentos = ev.crn;
                            }
                        }
                    }
                }
            },
            "StopFailure" => {
                match sessao.alvo(id, t, false) {
                    Alvo::Trocado => {
                        if let Some(trocado) = sessao.trocado.take() {
                            let turno = trocado.turno;
                            fechamentos.push(sessao.fechar_turno(
                                turno,
                                Fim::Falhou,
                                t,
                                agora,
                                &cfg,
                                false,
                            ));
                        }
                    }
                    Alvo::Aberto => fechamentos.extend(sessao.fechar(Fim::Falhou, t, agora, &cfg)),
                    Alvo::Fechado | Alvo::Novo => {}
                }
                sessao.estado = if ev.err.as_deref() == Some(ERRO_DE_LIMITE) {
                    EstadoSessao::Cansado
                } else {
                    EstadoSessao::Erro
                };
            }
            _ => ignorado = Some("evento_desconhecido"),
        }
        if sessao.estado != estado_antes {
            sessao.estado_desde = t;
            sessao.estado_desde_mono = agora.mono_ms;
            // Os avisos (decisão 0057): mudar de estado resolve o de antes, e
            // entrar em "esperando você" ou em erro abre um. O `idle_prompt`
            // (repete a cada ~60 s) nunca abre nem resolve; um evento
            // atrasado, mais velho que o estado de antes (a permissão que ele
            // pede já foi respondida), não mexe no aviso.
            let ocioso = ev.e == "Notification" && ev.nt.as_deref() == Some("idle_prompt");
            if !ocioso && t >= desde_antes {
                let tipo = match sessao.estado {
                    EstadoSessao::Esperando => Some(TipoAviso::Esperando),
                    EstadoSessao::Erro | EstadoSessao::Cansado => Some(TipoAviso::Erro),
                    _ => None,
                };
                // Um turno de máquina não resolve o pronto de antes: o Renan
                // não viu nada (decisão 0073).
                let de_maquina = sessao.turno.as_ref().is_some_and(|t| t.origem.maquina());
                let guarda_o_pronto = de_maquina
                    && tipo.is_none()
                    && sessao.aviso.is_some_and(|a| a.tipo == TipoAviso::Pronto);
                if !guarda_o_pronto {
                    sessao.aviso = tipo.map(|tipo| Aviso {
                        tipo,
                        espera: (tipo == TipoAviso::Esperando)
                            .then(|| espera_do_evento(ev))
                            .flatten(),
                        desde_ms: t,
                        desde_mono: agora.mono_ms,
                    });
                }
            }
        }
        // Um gatilho do mesmo diálogo (a sessão ainda espera e nada andou):
        // só sobe o tipo, e o relógio do aviso fica (decisão 0075). Um
        // gatilho que chegou fora de ordem, até 5 s antes, também refina.
        if !ev.agente
            && sessao.estado == EstadoSessao::Esperando
            && let Some(nova) = espera_do_evento(ev)
            && let Some(aviso) = sessao.aviso.as_mut()
            && aviso.tipo == TipoAviso::Esperando
            && t + JANELA_DO_DIALOGO_MS >= aviso.desde_ms
        {
            aviso.espera = aviso.espera.max(Some(nova));
        }
        if let Some(motivo) = ignorado {
            self.ignorar(motivo);
        }
        self.registrar(fechamentos, &mut reacoes, agora);
        reacoes
    }

    /// Antes de acompanhar uma sessão nova: se já há [`MAX_SESSOES`] do
    /// mesmo tipo, a mais parada sai (sem tchau).
    fn abrir_vaga(&mut self, teste: bool) {
        let do_tipo = self.sessoes.keys().filter(|(t, _)| *t == teste).count();
        if do_tipo < MAX_SESSOES {
            return;
        }
        let mais_parada = self
            .sessoes
            .iter()
            .filter(|((t, _), _)| *t == teste)
            .min_by_key(|(_, s)| s.ultimo_mono)
            .map(|(chave, _)| chave.clone());
        if let Some(chave) = mais_parada {
            self.sessoes.remove(&chave);
            self.ignorar("sessao_descartada");
        }
    }

    /// `SessionEnd`: a sessão sai com os turnos dela, sem festa. Tchau só se
    /// o processo está saindo (`sai`: não é `/clear` nem retomada) e não
    /// sobrou nenhuma sessão do mesmo tipo.
    fn encerrar(
        &mut self,
        chave: &(bool, String),
        t: u64,
        sai: bool,
        agora: Agora,
        reacoes: &mut Vec<Reacao>,
    ) {
        let Some(mut sessao) = self.sessoes.remove(chave) else {
            self.ignorar("fim_de_sessao_desconhecida");
            return;
        };
        let cfg = self.config.clone();
        let mut fechamentos = Vec::new();
        if let Some(trocado) = sessao.trocado.take() {
            fechamentos.push(sessao.fechar_trocado(trocado, agora, &cfg));
        }
        fechamentos.extend(sessao.fechar(Fim::SessaoEncerrada, t, agora, &cfg));
        fechamentos.extend(sessao.fechar_corrente(Fim::SessaoEncerrada, t));
        self.registrar(fechamentos, reacoes, agora);
        let sobrou = self.sessoes.keys().any(|(teste, _)| *teste == chave.0);
        if sai && !sobrou && self.config.modo != ModoCelebracao::Desligada {
            let reacao = Reacao {
                nome: TCHAU,
                sid8: evento::curto(&sessao.sid),
                proj: sessao.proj.clone(),
                ts: agora.parede_ms,
                nivel: None,
                teste: chave.0,
                discreta: false,
            };
            self.reagir(reacao, reacoes);
        }
    }

    /// Prazos vencidos: trocados cujo Stop não veio (fecham sem festa),
    /// acomodações que terminaram (comemoram) e sessões que expiraram (somem
    /// caladas).
    pub fn tique(&mut self, agora: Agora) -> Vec<Reacao> {
        let mut reacoes = Vec::new();
        let cfg = self.config.clone();
        let mut fechamentos = Vec::new();
        for sessao in self.sessoes.values_mut() {
            if sessao
                .trocado
                .as_ref()
                .is_some_and(|x| agora.mono_ms >= x.prazo_mono)
                && let Some(trocado) = sessao.trocado.take()
            {
                fechamentos.push(sessao.fechar_trocado(trocado, agora, &cfg));
            }
            let vencida = sessao
                .turno
                .as_ref()
                .and_then(|t| t.stop.as_ref())
                .is_some_and(|s| agora.mono_ms >= s.prazo_mono);
            if vencida {
                let fim_ms = sessao
                    .turno
                    .as_ref()
                    .and_then(|t| t.stop.as_ref())
                    .map_or(agora.parede_ms, |s| s.ts_ms);
                if let Some(f) = sessao.fechar(Fim::Stop, fim_ms, agora, &cfg) {
                    // O Claude terminou e o turno é do Renan: o pronto, com ou
                    // sem festa (decisão 0057); não com a corrente aberta nem
                    // num turno de máquina (decisão 0073).
                    if f.pronto {
                        sessao.aviso = Some(Aviso {
                            tipo: TipoAviso::Pronto,
                            espera: None,
                            desde_ms: fim_ms,
                            desde_mono: agora.mono_ms,
                        });
                    }
                    fechamentos.push(f);
                }
            }
            if sessao
                .corrente
                .as_ref()
                .is_some_and(|c| agora.mono_ms >= c.ultimo_mono + VIDA_CORRENTE_MS)
            {
                fechamentos.extend(sessao.fechar_corrente(Fim::CorrenteExpirou, agora.parede_ms));
            }
            if sessao
                .aviso
                .and_then(prazo_do_aviso)
                .is_some_and(|prazo| agora.mono_ms >= prazo)
            {
                sessao.aviso = None;
            }
            if sessao.prazo_do_estado().is_some_and(|p| agora.mono_ms >= p) {
                sessao.estado = EstadoSessao::Parada;
                sessao.estado_desde = agora.parede_ms;
                sessao.estado_desde_mono = agora.mono_ms;
            }
        }
        self.registrar(fechamentos, &mut reacoes, agora);
        let antes = self.sessoes.len();
        self.sessoes.retain(|(teste, _), s| {
            let vida = if *teste {
                VIDA_TESTE_MS
            } else {
                VIDA_SESSAO_MS
            };
            agora.mono_ms.saturating_sub(s.ultimo_mono) < vida
        });
        for _ in self.sessoes.len()..antes {
            self.ignorar("sessao_expirada");
        }
        reacoes
    }

    /// Os turnos que fecharam desde a última vez, do mais velho ao mais novo
    /// (o Motor os anota nas intenções).
    pub fn tirar_turnos_fechados(&mut self) -> Vec<RegistroTurno> {
        self.fechados_a_tirar.drain(..).collect()
    }

    /// De onde veio o turno aberto `turno` da sessão `chave` (decisão
    /// 0073): o Motor só casa a janela da sessão num prompt digitado.
    pub fn origem_do_turno(
        &self,
        chave: &(bool, String),
        turno: Option<&str>,
    ) -> Option<OrigemTurno> {
        self.sessoes
            .get(chave)?
            .turno
            .as_ref()
            .filter(|t| turno.is_none() || t.id.as_deref() == turno)
            .map(|t| t.origem)
    }

    /// O cérebro acompanha a sessão (teste, sid).
    pub fn tem_sessao(&self, chave: &(bool, String)) -> bool {
        self.sessoes.contains_key(chave)
    }

    /// Quando chamar [`Self::tique`] de novo (relógio monotônico).
    pub fn proximo_prazo(&self) -> Option<u64> {
        self.sessoes
            .iter()
            .flat_map(|((teste, _), s)| {
                let vida = if *teste {
                    VIDA_TESTE_MS
                } else {
                    VIDA_SESSAO_MS
                };
                let acomodacao = s
                    .turno
                    .as_ref()
                    .and_then(|t| t.stop.as_ref())
                    .map(|p| p.prazo_mono);
                let trocado = s.trocado.as_ref().map(|x| x.prazo_mono);
                let aviso = s.aviso.and_then(prazo_do_aviso);
                let corrente = s
                    .corrente
                    .as_ref()
                    .map(|c| c.ultimo_mono + VIDA_CORRENTE_MS);
                [
                    Some(s.ultimo_mono + vida),
                    acomodacao,
                    trocado,
                    aviso,
                    corrente,
                    s.prazo_do_estado(),
                ]
            })
            .flatten()
            .min()
    }

    /// As sessões com aviso, da mais urgente para a menos (decisão 0057):
    /// esperando você > erro > pronto; no mesmo tipo, as reais antes das de
    /// teste e a que espera há mais tempo primeiro.
    pub fn pendencias(&self) -> Vec<Pendencia> {
        let mut lista: Vec<Pendencia> = self
            .sessoes
            .iter()
            .filter_map(|(chave, s)| {
                s.aviso.map(|aviso| Pendencia {
                    chave: chave.clone(),
                    sid8: evento::curto(&s.sid),
                    proj: s.proj.clone(),
                    aviso,
                })
            })
            .collect();
        lista.sort_by(|a, b| {
            b.aviso
                .tipo
                .cmp(&a.aviso.tipo)
                .then(a.chave.0.cmp(&b.chave.0))
                .then(a.aviso.desde_ms.cmp(&b.aviso.desde_ms))
                .then(a.chave.1.cmp(&b.chave.1))
        });
        lista
    }

    /// O Renan viu o aviso da sessão (o clique que focou o terminal dela, ou
    /// o terminal em foco): ele sai. Devolve o tipo, se havia um.
    pub fn ver(&mut self, chave: &(bool, String)) -> Option<TipoAviso> {
        self.sessoes
            .get_mut(chave)
            .and_then(|s| s.aviso.take())
            .map(|a| a.tipo)
    }

    pub fn resumo(&self) -> Resumo {
        Resumo {
            sessoes: self.resumo_das_sessoes(),
            ultima_reacao: self.ultima_reacao.clone(),
            turnos: self.turnos.iter().rev().cloned().collect(),
            ignorados: self.ignorados.clone(),
            origens: self.config.origens.clone(),
        }
    }

    /// Só as sessões do [`Self::resumo`] (o Motor olha a tela a cada passo).
    pub fn resumo_das_sessoes(&self) -> Vec<ResumoSessao> {
        self.sessoes
            .values()
            .map(|s| ResumoSessao {
                sid8: evento::curto(&s.sid),
                proj: s.proj.clone(),
                ent: s.ent.clone(),
                teste: s.teste,
                estado: s.estado,
                estado_desde_ms: s.estado_desde,
                turno_aberto: s.turno.is_some(),
                acomodando: s.turno.as_ref().is_some_and(|t| t.stop.is_some()),
                ultimo_evento_ms: s.ultimo_parede,
                contadores: s.contadores.clone(),
                aviso: s.aviso,
                corrente: s.resumo_da_corrente(),
                agendamentos: s.agendamentos,
                janela: None,
                chave: (s.teste, s.sid.clone()),
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Hora de parede do instante 0 dos testes.
    const BASE: u64 = 1_790_000_000_000;
    const SID: &str = "aaaaaaaa-1111-4222-8333-444444444444";

    fn em(ms: u64) -> Agora {
        Agora {
            parede_ms: BASE + ms,
            mono_ms: ms,
        }
    }

    /// Um evento do jeito que o `avisar.sh` manda (origem `cli`).
    fn ev(e: &str) -> Evento {
        Evento {
            e: e.into(),
            sid: Some(SID.into()),
            ent: Some("cli".into()),
            proj: Some("meu-projeto".into()),
            ..Evento::default()
        }
    }

    fn prompt(turno: &str) -> Evento {
        Evento {
            turno: Some(turno.into()),
            src: Some("user".into()),
            ..ev("UserPromptSubmit")
        }
    }

    fn ferramenta(turno: &str, tool: &str, arq: Option<&str>, dur: u64) -> Evento {
        Evento {
            turno: Some(turno.into()),
            tool: Some(tool.into()),
            arq: arq.map(str::to_owned),
            dur: Some(dur),
            ..ev("PostToolUse")
        }
    }

    fn stop(turno: &str) -> Evento {
        Evento {
            turno: Some(turno.into()),
            ..ev("Stop")
        }
    }

    /// Um passo do roteiro: um evento chegando em `ms` (com `ts` = hora de
    /// parede de `ts_ms`, ou sem `ts`), ou só o relógio andando até `ms`.
    enum Passo {
        Chega {
            ms: u64,
            ts_ms: Option<u64>,
            evento: Box<Evento>,
        },
        Ate(u64),
    }

    /// Reações com o instante (ms) em que saíram.
    type Saida = Vec<(u64, &'static str)>;

    use Passo::{Ate, Chega};

    /// Evento chegando em `ms` com `ts` do mesmo instante.
    fn chega(ms: u64, evento: Evento) -> Passo {
        Chega {
            ms,
            ts_ms: Some(ms),
            evento: Box::new(evento),
        }
    }

    /// Evento chegando em `ms` com `ts` de `ts_ms` (chegou atrasado).
    fn atrasado(ms: u64, ts_ms: u64, evento: Evento) -> Passo {
        Chega {
            ms,
            ts_ms: Some(ts_ms),
            evento: Box::new(evento),
        }
    }

    /// Roda o roteiro como o daemon: cada evento no instante dele, e o
    /// `tique` em todo prazo vencido no caminho. Devolve (ms, reação).
    fn rodar(cerebro: &mut Cerebro, roteiro: Vec<Passo>) -> Saida {
        let mut saida = Vec::new();
        let mut relogio = 0;
        let mut andar_ate =
            |cerebro: &mut Cerebro, ate: u64, saida: &mut Vec<(u64, &'static str)>| {
                while let Some(prazo) = cerebro.proximo_prazo().filter(|p| *p <= ate) {
                    let prazo = prazo.max(relogio);
                    for r in cerebro.tique(em(prazo)) {
                        saida.push((prazo, r.nome));
                    }
                    relogio = prazo;
                    if cerebro.proximo_prazo() == Some(prazo) {
                        break;
                    }
                }
                relogio = relogio.max(ate);
            };
        for passo in roteiro {
            match passo {
                Chega {
                    ms,
                    ts_ms,
                    mut evento,
                } => {
                    andar_ate(cerebro, ms, &mut saida);
                    evento.ts = ts_ms.map(|t| BASE + t);
                    for r in cerebro.receber(&evento, BASE + ms, em(ms)) {
                        saida.push((ms, r.nome));
                    }
                }
                Ate(ms) => andar_ate(cerebro, ms, &mut saida),
            }
        }
        saida
    }

    fn novo() -> Cerebro {
        Cerebro::novo(ConfigCerebro::default())
    }

    #[test]
    fn tabela_de_cenarios() {
        let edit = |t: &str, arq: &str| ferramenta(t, "Edit", Some(arq), 40);
        let casos: Vec<(&str, Vec<Passo>, Saida)> = vec![
            (
                "rapido: resposta sem ferramenta → aceno",
                vec![
                    chega(0, ev("SessionStart")),
                    chega(100, prompt("p1")),
                    chega(30_000, stop("p1")),
                    Ate(40_000),
                ],
                vec![(30_800, ACENO)],
            ),
            (
                "pequeno: 1 Edit + 1 Bash → pulinho",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, edit("p1", "aaaaaaaaaaaa")),
                    chega(2_000, ferramenta("p1", "Bash", None, 900)),
                    chega(5_000, stop("p1")),
                    Ate(10_000),
                ],
                vec![(5_800, PULINHO)],
            ),
            (
                "resposta longa só com leitura → aceno",
                vec![
                    chega(0, prompt("p1")),
                    chega(10_000, ferramenta("p1", "Read", None, 5)),
                    chega(20_000, ferramenta("p1", "Grep", None, 5)),
                    chega(90_000, stop("p1")),
                    Ate(100_000),
                ],
                vec![(90_800, ACENO)],
            ),
            (
                "Stop repetido não comemora duas vezes",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, stop("p1")),
                    chega(1_200, stop("p1")),
                    Ate(5_000),
                    chega(6_000, stop("p1")),
                    Ate(9_000),
                ],
                vec![(1_800, ACENO)],
            ),
            (
                "PostToolUse depois do Stop cancela a acomodação",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, stop("p1")),
                    chega(1_300, ferramenta("p1", "Bash", None, 50)),
                    Ate(5_000),
                    chega(
                        6_000,
                        Evento {
                            sha: true,
                            ..stop("p1")
                        },
                    ),
                    Ate(9_000),
                ],
                vec![(6_800, PULINHO)],
            ),
            (
                "PostToolUse atrasado (ts antes do Stop) conta e não cancela",
                vec![
                    chega(0, prompt("p1")),
                    chega(2_000, stop("p1")),
                    atrasado(2_050, 1_500, edit("p1", "bbbbbbbbbbbb")),
                    Ate(5_000),
                ],
                vec![(2_800, PULINHO)],
            ),
            (
                "prompt novo na acomodação comemora o anterior na hora",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, edit("p1", "cccccccccccc")),
                    chega(2_000, stop("p1")),
                    chega(2_100, prompt("p2")),
                    chega(9_000, stop("p2")),
                    Ate(12_000),
                ],
                vec![(2_100, PULINHO), (9_800, ACENO)],
            ),
            (
                "subagente conta para o turno",
                vec![
                    chega(0, prompt("p1")),
                    chega(
                        500,
                        Evento {
                            turno: Some("p1".into()),
                            agente: true,
                            aid: Some("ag1".into()),
                            ..ev("SubagentStart")
                        },
                    ),
                    chega(
                        900,
                        Evento {
                            agente: true,
                            aid: Some("ag1".into()),
                            ..ferramenta("p1", "Grep", None, 3)
                        },
                    ),
                    chega(3_000, stop("p1")),
                    Ate(5_000),
                ],
                vec![(3_800, PULINHO)],
            ),
            (
                "SubagentStart depois do Stop cancela a acomodação",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, stop("p1")),
                    chega(
                        1_400,
                        Evento {
                            turno: Some("p1".into()),
                            agente: true,
                            aid: Some("ag2".into()),
                            ..ev("SubagentStart")
                        },
                    ),
                    Ate(5_000),
                ],
                vec![],
            ),
            (
                "interrompido (Esc numa ferramenta) não comemora",
                vec![
                    chega(0, prompt("p1")),
                    chega(
                        1_000,
                        Evento {
                            turno: Some("p1".into()),
                            tool: Some("Bash".into()),
                            intr: true,
                            ..ev("PostToolUseFailure")
                        },
                    ),
                    Ate(5_000),
                ],
                vec![],
            ),
            (
                "erro de API fecha o turno sem festa",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, edit("p1", "dddddddddddd")),
                    chega(
                        2_000,
                        Evento {
                            turno: Some("p1".into()),
                            err: Some("rate_limit".into()),
                            ..ev("StopFailure")
                        },
                    ),
                    Ate(5_000),
                ],
                vec![],
            ),
            (
                "Stop sem prompt visto (pet reiniciou no meio) ainda comemora",
                vec![
                    chega(0, edit("p9", "eeeeeeeeeeee")),
                    chega(1_000, stop("p9")),
                    Ate(3_000),
                ],
                vec![(1_800, PULINHO)],
            ),
            (
                "tchau quando a última sessão acaba",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, stop("p1")),
                    Ate(2_000),
                    chega(3_000, ev("SessionEnd")),
                ],
                vec![(1_800, ACENO), (3_000, TCHAU)],
            ),
            (
                // Prompt na fila (ou aviso de tarefa) entra logo depois do
                // Stop, e os dois hooks async chegam trocados.
                "Stop que chega depois do prompt seguinte ainda comemora",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, edit("p1", "aaaaaaaaaaaa")),
                    chega(5_003, prompt("p2")),
                    atrasado(5_010, 5_000, stop("p1")),
                    chega(9_000, stop("p2")),
                    Ate(12_000),
                ],
                vec![(5_010, PULINHO), (9_800, ACENO)],
            ),
            (
                "Stop do turno trocado depois da espera não comemora",
                vec![
                    chega(0, prompt("p1")),
                    chega(1_000, edit("p1", "aaaaaaaaaaaa")),
                    chega(5_000, prompt("p2")),
                    atrasado(5_900, 4_990, stop("p1")),
                    Ate(9_000),
                ],
                vec![],
            ),
            (
                "evento atrasado do turno trocado conta nele e não fecha o novo",
                vec![
                    chega(0, prompt("p1")),
                    chega(2_000, prompt("p2")),
                    atrasado(2_050, 1_500, edit("p1", "bbbbbbbbbbbb")),
                    atrasado(2_100, 1_900, stop("p1")),
                    chega(5_000, stop("p2")),
                    Ate(8_000),
                ],
                vec![(2_100, PULINHO), (5_800, ACENO)],
            ),
            (
                // Um Stop hook de outro plugin segura o Claude: a continuação
                // chega segundos depois da festa, com o mesmo `prompt_id`.
                "stop-bloqueado: a continuação reabre o turno e o Stop final sobe de nível",
                vec![
                    chega(0, prompt("q1")),
                    chega(1_000, stop("q1")),
                    Ate(2_000),
                    chega(3_000, edit("q1", "cccccccccccc")),
                    chega(3_500, ferramenta("q1", "Bash", None, 900)),
                    chega(
                        6_000,
                        Evento {
                            sha: true,
                            ..stop("q1")
                        },
                    ),
                    Ate(8_000),
                ],
                vec![(1_800, ACENO), (6_800, PULINHO)],
            ),
            (
                "stop-bloqueado sem subir de nível não comemora de novo",
                vec![
                    chega(0, prompt("q1")),
                    chega(500, edit("q1", "dddddddddddd")),
                    chega(1_000, stop("q1")),
                    Ate(2_000),
                    chega(3_000, ferramenta("q1", "Bash", None, 50)),
                    chega(
                        5_000,
                        Evento {
                            sha: true,
                            ..stop("q1")
                        },
                    ),
                    Ate(7_000),
                ],
                vec![(1_800, PULINHO)],
            ),
            (
                "evento atrasado (ts antes do Stop) depois da festa não reabre",
                vec![
                    chega(0, prompt("q1")),
                    chega(1_000, stop("q1")),
                    Ate(2_000),
                    atrasado(2_500, 900, ferramenta("q1", "Bash", None, 5)),
                    Ate(6_000),
                ],
                vec![(1_800, ACENO)],
            ),
        ];
        for (nome, roteiro, esperado) in casos {
            let mut c = novo();
            assert_eq!(rodar(&mut c, roteiro), esperado, "{nome}");
        }
    }

    /// O registro do turno `turno` (há um só por turno).
    fn registro<'a>(r: &'a Resumo, turno: &str) -> &'a RegistroTurno {
        let achados: Vec<_> = r
            .turnos
            .iter()
            .filter(|t| t.turno8.as_deref() == Some(turno))
            .collect();
        assert_eq!(achados.len(), 1, "um registro por turno: {:?}", r.turnos);
        achados[0]
    }

    #[test]
    fn stop_atrasado_comemora_o_turno_trocado_com_os_componentes_dele() {
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, prompt("p1")),
                chega(2_000, prompt("p2")),
                atrasado(2_050, 1_500, edit_teste("p1")),
                atrasado(2_100, 1_900, stop("p1")),
                Ate(2_200),
            ],
        );
        assert_eq!(saida, vec![(2_100, PULINHO)]);
        let r = c.resumo();
        let p1 = registro(&r, "p1");
        assert_eq!(
            (p1.fim, p1.nivel, p1.trabalho),
            (Fim::Stop, Some(Nivel::T1), 1)
        );
        assert_eq!(p1.fim_ms, BASE + 1_900, "fim = hora do Stop");
        assert!(r.ignorados.is_empty(), "{:?}", r.ignorados);
        // O p2 segue aberto e pensando, intocado pelos eventos do p1.
        assert_eq!(r.sessoes[0].estado, EstadoSessao::Pensando);
        assert!(r.sessoes[0].turno_aberto);
        // Sem Stop, o trocado fecha sem festa depois da espera, com a hora do
        // prompt que o trocou.
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, prompt("p1")),
                chega(5_000, prompt("p2")),
                Ate(5_799),
            ],
        );
        assert!(saida.is_empty());
        assert!(c.resumo().turnos.is_empty(), "ainda esperando o Stop");
        assert_eq!(c.proximo_prazo(), Some(5_000 + ACOMODACAO_MS));
        rodar(&mut c, vec![Ate(5_800)]);
        let r = c.resumo();
        let p1 = registro(&r, "p1");
        assert_eq!(
            (p1.fim, p1.fim_ms, p1.reacao),
            (Fim::Substituido, BASE + 5_000, None)
        );
    }

    #[test]
    fn stop_bloqueado_reabre_e_fica_num_registro_so() {
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, prompt("q1")),
                chega(1_000, stop("q1")),
                Ate(2_000),
                chega(3_000, edit_teste("q1")),
            ],
        );
        assert_eq!(saida, vec![(1_800, ACENO)]);
        let r = c.resumo();
        assert!(r.sessoes[0].turno_aberto, "a continuação reabriu o turno");
        assert_eq!(r.sessoes[0].estado, EstadoSessao::Trabalhando);
        let saida = rodar(
            &mut c,
            vec![
                chega(
                    6_000,
                    Evento {
                        sha: true,
                        ..stop("q1")
                    },
                ),
                Ate(8_000),
            ],
        );
        assert_eq!(saida, vec![(6_800, PULINHO)]);
        let r = c.resumo();
        assert_eq!(r.turnos.len(), 1, "{:?}", r.turnos);
        let q1 = registro(&r, "q1");
        assert_eq!(
            (q1.fim, q1.nivel, q1.reacao, q1.sha, q1.continuacoes),
            (Fim::Stop, Some(Nivel::T1), Some(PULINHO), true, 1)
        );
        assert_eq!((q1.trabalho, q1.arquivos, q1.fim_ms), (1, 1, BASE + 6_000));
        assert!(r.ignorados.is_empty(), "{:?}", r.ignorados);
        let s = &r.sessoes[0];
        assert_eq!((s.contadores.turnos, s.contadores.reacoes), (1, 2));
        assert_eq!(s.estado, EstadoSessao::Parada);

        // Continuação que acaba sem Stop (Esc, `idle_prompt`): o registro
        // fica com a festa de antes.
        let mut c = novo();
        let ocioso = Evento {
            nt: Some("idle_prompt".into()),
            ..ev("Notification")
        };
        let saida = rodar(
            &mut c,
            vec![
                chega(0, prompt("q1")),
                chega(1_000, stop("q1")),
                Ate(2_000),
                chega(3_000, ferramenta("q1", "Read", None, 5)),
                chega(63_000, ocioso),
            ],
        );
        assert_eq!(saida, vec![(1_800, ACENO)]);
        let r = c.resumo();
        let q1 = registro(&r, "q1");
        assert_eq!(
            (q1.fim, q1.nivel, q1.reacao, q1.continuacoes, q1.outras),
            (Fim::Ocioso, Some(Nivel::T0), Some(ACENO), 1, 1)
        );
        assert_eq!(r.sessoes[0].estado, EstadoSessao::Parada);
        // Um prompt novo encerra a chance de continuação.
        let mut c = novo();
        rodar(
            &mut c,
            vec![
                chega(0, prompt("q1")),
                chega(1_000, stop("q1")),
                Ate(2_000),
                chega(3_000, prompt("q2")),
                chega(3_500, ferramenta("q1", "Bash", None, 5)),
            ],
        );
        let r = c.resumo();
        assert_eq!(r.ignorados["turno_fechado"], 1);
        assert_eq!(registro(&r, "q1").continuacoes, 0);
    }

    #[test]
    fn evento_ignorado_nao_mexe_no_estado() {
        let mut c = novo();
        let estado = |c: &Cerebro| {
            (
                c.resumo().sessoes[0].estado,
                c.resumo().sessoes[0].turno_aberto,
            )
        };
        rodar(
            &mut c,
            vec![chega(0, prompt("r1")), chega(1_000, stop("r1")), Ate(2_000)],
        );
        assert_eq!(estado(&c), (EstadoSessao::Parada, false));
        // Prompt atrasado de um turno já fechado.
        rodar(&mut c, vec![atrasado(2_500, 10, prompt("r1"))]);
        assert_eq!(estado(&c), (EstadoSessao::Parada, false));
        // Ferramenta e pergunta atrasadas (ts antes do Stop) depois da festa.
        let pergunta = Evento {
            turno: Some("r1".into()),
            tool: Some("AskUserQuestion".into()),
            ..ev("PreToolUse")
        };
        rodar(
            &mut c,
            vec![
                atrasado(2_600, 900, ferramenta("r1", "Bash", None, 5)),
                atrasado(2_700, 950, pergunta),
            ],
        );
        assert_eq!(estado(&c), (EstadoSessao::Parada, false));
        // Stop repetido também não.
        rodar(&mut c, vec![chega(2_800, stop("r1"))]);
        assert_eq!(estado(&c), (EstadoSessao::Parada, false));
        let r = c.resumo();
        assert_eq!(r.ignorados["turno_fechado"], 3);
        assert_eq!(r.ignorados["stop_repetido"], 1);
        // Ferramenta atrasada durante a acomodação: conta no turno, mas a
        // sessão continua parada (o Stop já disse que acabou).
        let mut c = novo();
        rodar(
            &mut c,
            vec![
                chega(0, prompt("r2")),
                chega(1_000, stop("r2")),
                atrasado(1_100, 800, ferramenta("r2", "Bash", None, 5)),
            ],
        );
        let r = c.resumo();
        assert_eq!(r.sessoes[0].estado, EstadoSessao::Parada);
        assert!(r.sessoes[0].acomodando);
        rodar(&mut c, vec![Ate(3_000)]);
        assert_eq!(registro(&c.resumo(), "r2").trabalho, 1);
    }

    #[test]
    fn arquivos_por_turno_tem_teto() {
        let mut c = novo();
        c.receber(&prompt("p1"), BASE, em(0));
        let total = ARQUIVOS_POR_TURNO + 500;
        for i in 0..total {
            let arq = format!("{i:012x}");
            c.receber(&ferramenta("p1", "Edit", Some(&arq), 1), BASE + 1, em(1));
        }
        // Repetido não conta de novo (antes do teto).
        c.receber(
            &ferramenta("p1", "Edit", Some(&format!("{:012x}", 0)), 1),
            BASE + 1,
            em(1),
        );
        let lembrados = c
            .sessoes
            .values()
            .next()
            .unwrap()
            .turno
            .as_ref()
            .unwrap()
            .comp
            .arquivos
            .len();
        assert_eq!(lembrados, ARQUIVOS_POR_TURNO, "memória limitada");
        c.receber(&stop("p1"), BASE + 2, em(2));
        assert_eq!(
            c.tique(em(2 + ACOMODACAO_MS))[0].nome,
            VOO_GRANDE,
            "1524 arquivos: a pontuação no teto"
        );
        let r = c.resumo();
        assert_eq!(registro(&r, "p1").arquivos as usize, total);
        assert_eq!(registro(&r, "p1").trabalho as usize, total + 1);
    }

    #[test]
    fn origem_sdk_cli_e_sem_origem_ignoradas() {
        let mut c = novo();
        let sdk = |e: Evento| Evento {
            ent: Some("sdk-cli".into()),
            ..e
        };
        let roteiro = vec![
            chega(0, sdk(prompt("p1"))),
            chega(1_000, sdk(stop("p1"))),
            chega(
                1_100,
                Evento {
                    ent: None,
                    ..stop("p2")
                },
            ),
            Ate(5_000),
        ];
        assert_eq!(rodar(&mut c, roteiro), vec![]);
        let r = c.resumo();
        assert!(r.sessoes.is_empty());
        assert_eq!(r.ignorados["origem:sdk-cli"], 2);
        assert_eq!(r.ignorados["origem:desconhecida"], 1);
        // Com sdk-cli na config, a mesma sessão conta.
        let mut c = Cerebro::novo(ConfigCerebro {
            origens: vec!["cli".into(), "sdk-cli".into()],
            ..ConfigCerebro::default()
        });
        let roteiro = vec![
            chega(0, sdk(prompt("p1"))),
            chega(1_000, sdk(stop("p1"))),
            Ate(5_000),
        ];
        assert_eq!(rodar(&mut c, roteiro), vec![(1_800, ACENO)]);
    }

    #[test]
    fn teste_expira_e_nao_se_mistura_com_o_real() {
        let mut c = novo();
        let t = |e: Evento| Evento { teste: true, ..e };
        let roteiro = vec![
            // Sessão real e de teste com o mesmo sid.
            chega(0, prompt("p1")),
            chega(100, t(prompt("p1"))),
            chega(200, t(edit_teste("p1"))),
            chega(300, t(stop("p1"))),
            Ate(2_000),
        ];
        assert_eq!(rodar(&mut c, roteiro), vec![(1_100, PULINHO)]);
        let r = c.resumo();
        assert_eq!(r.sessoes.len(), 2);
        let real = r.sessoes.iter().find(|s| !s.teste).unwrap();
        assert!(real.turno_aberto, "o turno real continua aberto");
        assert_eq!(real.contadores.ferramentas, 0);
        assert!(r.ultima_reacao.as_ref().unwrap().teste);
        // 60 s depois do último evento a de teste some; a real fica.
        rodar(&mut c, vec![Ate(300 + VIDA_TESTE_MS - 1)]);
        assert_eq!(c.resumo().sessoes.len(), 2);
        rodar(&mut c, vec![Ate(300 + VIDA_TESTE_MS)]);
        let r = c.resumo();
        assert_eq!(r.sessoes.len(), 1);
        assert!(!r.sessoes[0].teste);
        assert_eq!(r.ignorados["sessao_expirada"], 1);
    }

    fn edit_teste(turno: &str) -> Evento {
        ferramenta(turno, "Write", Some("ffffffffffff"), 10)
    }

    #[test]
    fn session_end_limpa_sessao_turno_e_acomodacao() {
        let mut c = novo();
        let outra = |e: Evento| Evento {
            sid: Some("bbbbbbbb-2222".into()),
            ..e
        };
        let roteiro = vec![
            chega(0, prompt("p1")),
            chega(100, outra(prompt("q1"))),
            chega(1_000, edit_teste("p1")),
            chega(2_000, stop("p1")),
            // Acaba antes da acomodação: sem festa, e ainda há outra sessão.
            chega(2_100, ev("SessionEnd")),
            Ate(5_000),
        ];
        assert_eq!(rodar(&mut c, roteiro), vec![]);
        let r = c.resumo();
        assert_eq!(r.sessoes.len(), 1);
        assert_eq!(r.sessoes[0].sid8, "bbbbbbbb");
        assert_eq!(r.turnos[0].fim, Fim::SessaoEncerrada);
        assert_eq!(r.turnos[0].trabalho, 1);
        // A última sessão acaba: tchau.
        let saida = rodar(&mut c, vec![chega(6_000, outra(ev("SessionEnd")))]);
        assert_eq!(saida, vec![(6_000, TCHAU)]);
        assert!(c.resumo().sessoes.is_empty());
        assert_eq!(c.proximo_prazo(), None);
    }

    #[test]
    fn clear_e_retomada_nao_dao_tchau() {
        let fim = |reason: &str| Evento {
            reason: Some(reason.into()),
            ..ev("SessionEnd")
        };
        for (reason, esperado) in [
            ("clear", vec![]),
            ("resume", vec![]),
            ("prompt_input_exit", vec![(1_000, TCHAU)]),
            ("logout", vec![(1_000, TCHAU)]),
            ("other", vec![(1_000, TCHAU)]),
        ] {
            let mut c = novo();
            let saida = rodar(
                &mut c,
                vec![chega(0, ev("SessionStart")), chega(1_000, fim(reason))],
            );
            assert_eq!(saida, esperado, "{reason}");
            assert!(
                c.resumo().sessoes.is_empty(),
                "{reason}: a sessão sempre sai"
            );
        }
        // /clear: o fim da velha e o começo da nova, em qualquer ordem.
        let nova = |e: Evento| Evento {
            sid: Some("dddddddd-nova".into()),
            ..e
        };
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, ev("SessionStart")),
                chega(1_000, nova(ev("SessionStart"))),
                chega(1_001, fim("clear")),
            ],
        );
        assert_eq!(saida, vec![]);
        assert_eq!(c.resumo().sessoes[0].sid8, "dddddddd");
    }

    #[test]
    fn motivos_ignorados_tem_teto() {
        let mut c = novo();
        for i in 0..100 {
            let ev = Evento {
                ent: Some(format!("origem-{i}")),
                ..stop("p1")
            };
            c.receber(&ev, BASE, em(0));
        }
        let r = c.resumo();
        assert_eq!(r.ignorados.len(), MAX_MOTIVOS + 1);
        assert_eq!(r.ignorados["outros"], 100 - MAX_MOTIVOS as u64);
    }

    #[test]
    fn idle_prompt_fecha_turno_sem_stop() {
        let mut c = novo();
        let ocioso = Evento {
            nt: Some("idle_prompt".into()),
            ..ev("Notification")
        };
        let roteiro = vec![
            chega(0, prompt("p1")),
            chega(1_000, edit_teste("p1")),
            // Esc no meio da resposta: nenhum Stop. Um minuto depois:
            chega(61_000, ocioso.clone()),
            Ate(70_000),
        ];
        assert_eq!(rodar(&mut c, roteiro), vec![]);
        let r = c.resumo();
        assert_eq!(r.turnos[0].fim, Fim::Ocioso);
        assert_eq!(r.sessoes[0].estado, EstadoSessao::Parada);
        assert!(!r.sessoes[0].turno_aberto);
        // Repetido a cada minuto não faz nada.
        assert_eq!(rodar(&mut c, vec![chega(121_000, ocioso)]), vec![]);
        assert_eq!(c.resumo().turnos.len(), 1);
    }

    #[test]
    fn prompt_sem_stop_no_anterior_substitui_sem_festa() {
        let mut c = novo();
        let roteiro = vec![
            chega(0, prompt("p1")),
            chega(1_000, edit_teste("p1")),
            chega(5_000, prompt("p2")),
            chega(6_000, stop("p2")),
            Ate(8_000),
        ];
        assert_eq!(rodar(&mut c, roteiro), vec![(6_800, ACENO)]);
        let r = c.resumo();
        assert_eq!(r.turnos[1].fim, Fim::Substituido);
        assert_eq!(r.turnos[0].fim, Fim::Stop);
    }

    #[test]
    fn componentes_do_turno_registrados() {
        let mut c = novo();
        let roteiro = vec![
            chega(0, prompt("p1")),
            chega(1_000, ferramenta("p1", "Edit", Some("aaaaaaaaaaaa"), 30)),
            chega(2_000, ferramenta("p1", "Edit", Some("aaaaaaaaaaaa"), 20)),
            chega(3_000, ferramenta("p1", "Write", Some("bbbbbbbbbbbb"), 10)),
            chega(4_000, ferramenta("p1", "Bash", None, 1_500)),
            chega(5_000, ferramenta("p1", "Read", None, 5)),
            chega(
                6_000,
                Evento {
                    turno: Some("p1".into()),
                    tool: Some("Bash".into()),
                    dur: Some(700),
                    ..ev("PostToolUseFailure")
                },
            ),
            chega(
                9_000,
                Evento {
                    bg: Some(2),
                    ..stop("p1")
                },
            ),
            Ate(12_000),
        ];
        assert_eq!(rodar(&mut c, roteiro), vec![(9_800, PULINHO)]);
        let r = c.resumo();
        let t = &r.turnos[0];
        assert_eq!(
            (t.trabalho, t.outras, t.arquivos, t.falhas, t.dur_ms),
            (5, 1, 2, 1, 2_265)
        );
        assert_eq!(
            (t.t0_ms, t.fim_ms, t.relogio_ms),
            (BASE, BASE + 9_000, 9_000)
        );
        assert_eq!(t.bg, Some(2));
        assert_eq!(t.nivel, Some(Nivel::T1));
        assert_eq!(t.reacao, Some(PULINHO));
        assert_eq!(t.turno8.as_deref(), Some("p1"));
        assert!(!t.implicito);
        let ultima = r.ultima_reacao.unwrap();
        assert_eq!(ultima.nome, PULINHO);
        assert_eq!(ultima.sid8, "aaaaaaaa");
        assert_eq!(ultima.proj.as_deref(), Some("meu-projeto"));
        assert_eq!(ultima.ts, BASE + 9_800);
        let s = &r.sessoes[0];
        assert_eq!(s.contadores.ferramentas, 6);
        assert_eq!(s.contadores.turnos, 1);
        assert_eq!(s.contadores.reacoes, 1);
    }

    #[test]
    fn ts_fora_da_janela_vale_a_chegada() {
        assert_eq!(hora_do_evento(Some(BASE + 5), BASE), BASE + 5);
        assert_eq!(
            hora_do_evento(Some(BASE - JANELA_TS_MS), BASE),
            BASE - JANELA_TS_MS
        );
        assert_eq!(hora_do_evento(Some(BASE + JANELA_TS_MS + 1), BASE), BASE);
        assert_eq!(hora_do_evento(Some(1), BASE), BASE);
        assert_eq!(hora_do_evento(None, BASE), BASE);
        // Um `ts` de 1970 (relógio do host maluco) não vale: conta a chegada,
        // que é depois do Stop, e o PostToolUse cancela a acomodação.
        let mut c = novo();
        rodar(
            &mut c,
            vec![chega(0, prompt("p1")), chega(1_000, stop("p1"))],
        );
        let maluco = Evento {
            ts: Some(1),
            ..ferramenta("p1", "Bash", None, 1)
        };
        assert!(c.receber(&maluco, BASE + 1_300, em(1_300)).is_empty());
        assert!(c.tique(em(5_000)).is_empty());
        assert!(c.resumo().sessoes[0].turno_aberto);
        // Sem `ts` também vale a chegada.
        let mut c = novo();
        let roteiro = vec![
            chega(0, prompt("p1")),
            chega(1_000, stop("p1")),
            Chega {
                ms: 1_300,
                ts_ms: None,
                evento: Box::new(ferramenta("p1", "Bash", None, 1)),
            },
            Ate(5_000),
        ];
        assert_eq!(rodar(&mut c, roteiro), vec![]);
    }

    #[test]
    fn modos_de_celebracao() {
        let roteiro = || {
            vec![
                chega(0, prompt("p1")),
                chega(1_000, edit_teste("p1")),
                chega(2_000, stop("p1")),
                Ate(3_000),
                chega(4_000, ev("SessionEnd")),
            ]
        };
        let com_modo = |modo| {
            Cerebro::novo(ConfigCerebro {
                modo,
                ..ConfigCerebro::default()
            })
        };
        // Discreta: o teto T1 (decisão 0074; no M3 ela só acenava).
        assert_eq!(
            rodar(&mut com_modo(ModoCelebracao::Discreta), roteiro()),
            vec![(2_800, PULINHO), (4_000, TCHAU)]
        );
        assert_eq!(
            rodar(&mut com_modo(ModoCelebracao::Desligada), roteiro()),
            vec![]
        );
        // Sempre grande: todo nível acima do T0 vira T3.
        let mut c = com_modo(ModoCelebracao::SempreGrande);
        assert_eq!(
            rodar(&mut c, roteiro()),
            vec![(2_800, VOO_GRANDE), (4_000, TCHAU)]
        );
        assert_eq!(registro(&c.resumo(), "p1").teto, vec!["modo"]);
        // Um turno de T2 no modo discreto fica no pulinho, com o teto anotado.
        let medio = || {
            let mut r = vec![chega(0, prompt("p1"))];
            for i in 0..12u64 {
                r.push(chega(
                    1_000 + i,
                    ferramenta("p1", "Edit", Some(&format!("{i:012x}")), 15_000),
                ));
            }
            r.push(chega(5_000, stop("p1")));
            r.push(Ate(6_000));
            r
        };
        let mut c = com_modo(ModoCelebracao::Discreta);
        assert_eq!(rodar(&mut c, medio()), vec![(5_800, PULINHO)]);
        let r = c.resumo();
        let p1 = registro(&r, "p1");
        assert_eq!(
            (p1.nivel_calculado, p1.nivel, p1.teto.clone()),
            (Some(Nivel::T2), Some(Nivel::T1), vec!["modo"])
        );
    }

    #[test]
    fn eventos_sem_sid_e_desconhecidos() {
        let mut c = novo();
        let sem_sid = Evento {
            sid: None,
            ..stop("p1")
        };
        rodar(
            &mut c,
            vec![chega(0, sem_sid), chega(10, ev("EventoDoFuturo"))],
        );
        let r = c.resumo();
        assert_eq!(r.ignorados["sem_sid"], 1);
        assert_eq!(r.ignorados["evento_desconhecido"], 1);
        assert_eq!(
            r.sessoes.len(),
            1,
            "o evento do futuro ainda marca a sessão"
        );
    }

    #[test]
    fn estado_da_sessao_acompanha_os_eventos() {
        let mut c = novo();
        let estado = |c: &Cerebro| c.resumo().sessoes[0].estado;
        rodar(&mut c, vec![chega(0, ev("SessionStart"))]);
        assert_eq!(estado(&c), EstadoSessao::Parada);
        rodar(&mut c, vec![chega(10, prompt("p1"))]);
        assert_eq!(estado(&c), EstadoSessao::Pensando);
        let pergunta = Evento {
            turno: Some("p1".into()),
            tool: Some("AskUserQuestion".into()),
            ..ev("PreToolUse")
        };
        rodar(&mut c, vec![chega(20, pergunta)]);
        assert_eq!(estado(&c), EstadoSessao::Esperando);
        rodar(&mut c, vec![chega(30, ferramenta("p1", "Bash", None, 1))]);
        assert_eq!(estado(&c), EstadoSessao::Trabalhando);
        // Ferramenta de subagente não muda o estado.
        rodar(&mut c, vec![chega(35, ev("PreCompact"))]);
        assert_eq!(estado(&c), EstadoSessao::Compactando);
        let de_agente = Evento {
            agente: true,
            aid: Some("x".into()),
            ..ferramenta("p1", "Read", None, 1)
        };
        rodar(&mut c, vec![chega(40, de_agente)]);
        assert_eq!(estado(&c), EstadoSessao::Compactando);
        rodar(&mut c, vec![chega(50, ev("PostCompact"))]);
        assert_eq!(estado(&c), EstadoSessao::Trabalhando);
        rodar(&mut c, vec![chega(60, stop("p1"))]);
        assert_eq!(estado(&c), EstadoSessao::Parada);
        assert!(c.resumo().sessoes[0].acomodando);
    }

    #[test]
    fn so_os_ultimos_20_turnos_ficam() {
        let mut c = novo();
        let mut roteiro = Vec::new();
        for i in 0..25u64 {
            let id = format!("p{i}");
            roteiro.push(chega(i * 10_000, prompt(&id)));
            roteiro.push(chega(i * 10_000 + 1_000, stop(&id)));
        }
        roteiro.push(Ate(300_000));
        assert_eq!(rodar(&mut c, roteiro).len(), 25);
        let r = c.resumo();
        assert_eq!(r.turnos.len(), TURNOS_GUARDADOS);
        assert_eq!(
            r.turnos[0].turno8.as_deref(),
            Some("p24"),
            "mais novo primeiro"
        );
    }

    #[test]
    fn prazos() {
        let mut c = novo();
        assert_eq!(c.proximo_prazo(), None);
        c.receber(&prompt("p1"), BASE, em(0));
        // Pensando sem evento volta a parada em 5 min (decisão 0076).
        assert_eq!(c.proximo_prazo(), Some(ATIVA_SEM_EVENTO_MS));
        c.receber(&stop("p1"), BASE + 1_000, em(1_000));
        assert_eq!(c.proximo_prazo(), Some(1_000 + ACOMODACAO_MS));
        assert!(c.tique(em(1_799)).is_empty());
        assert_eq!(c.tique(em(1_800))[0].nome, ACENO);
        // O pronto some em 2 h (decisão 0057); a sessão, em 12 h.
        assert_eq!(c.proximo_prazo(), Some(1_800 + VIDA_AVISO_MS));
        assert!(c.tique(em(1_800 + VIDA_AVISO_MS)).is_empty());
        assert!(c.pendencias().is_empty());
        assert_eq!(c.proximo_prazo(), Some(1_000 + VIDA_SESSAO_MS));
        // Real sem eventos por 12 h some calada.
        assert!(c.tique(em(1_000 + VIDA_SESSAO_MS)).is_empty());
        assert!(c.resumo().sessoes.is_empty());
    }

    #[test]
    fn reconfigurar_troca_origens_e_modo() {
        let mut c = Cerebro::novo(ConfigCerebro::default());
        let saida = rodar(
            &mut c,
            vec![chega(0, prompt("p1")), chega(10, stop("p1")), Ate(900)],
        );
        assert_eq!(saida, vec![(810, ACENO)]);
        assert_eq!(c.resumo().sessoes.len(), 1);

        // Só `sdk-cli` agora: a sessão `cli` sai sem reação e os eventos
        // dela passam a ser ignorados.
        c.reconfigurar(ConfigCerebro {
            origens: vec!["sdk-cli".into()],
            modo: ModoCelebracao::Proporcional,
            ..ConfigCerebro::default()
        });
        assert_eq!(c.config().origens, vec!["sdk-cli"]);
        assert!(c.resumo().sessoes.is_empty());
        let saida = rodar(
            &mut c,
            vec![
                chega(1000, prompt("p2")),
                chega(1010, stop("p2")),
                Ate(2000),
            ],
        );
        assert!(saida.is_empty(), "{saida:?}");
        assert_eq!(c.resumo().ignorados["origem:cli"], 2);

        // De volta ao `cli`, com a celebração desligada: conta, mas não reage.
        c.reconfigurar(ConfigCerebro {
            origens: vec!["cli".into()],
            modo: ModoCelebracao::Desligada,
            ..ConfigCerebro::default()
        });
        let saida = rodar(
            &mut c,
            vec![
                chega(3000, prompt("p3")),
                chega(3010, stop("p3")),
                Ate(4000),
            ],
        );
        assert!(saida.is_empty(), "{saida:?}");
        assert_eq!(c.resumo().sessoes.len(), 1);
    }

    // --- avisos (decisão 0057) ----------------------------------------------

    fn aviso_de(c: &Cerebro) -> Option<TipoAviso> {
        c.resumo()
            .sessoes
            .first()
            .and_then(|s| s.aviso)
            .map(|a| a.tipo)
    }

    fn permissao(turno: &str) -> Evento {
        Evento {
            turno: Some(turno.into()),
            tool: Some("Bash".into()),
            ..ev("PermissionRequest")
        }
    }

    fn notificacao(nt: &str) -> Evento {
        Evento {
            nt: Some(nt.into()),
            ..ev("Notification")
        }
    }

    fn falhou(turno: &str) -> Evento {
        Evento {
            turno: Some(turno.into()),
            ..ev("StopFailure")
        }
    }

    #[test]
    fn avisos_abrem_e_se_resolvem_com_o_estado() {
        let mut c = novo();
        rodar(
            &mut c,
            vec![chega(0, prompt("p1")), chega(1_000, permissao("p1"))],
        );
        assert_eq!(aviso_de(&c), Some(TipoAviso::Esperando));
        let desde = c.resumo().sessoes[0].aviso.unwrap().desde_ms;
        assert_eq!(desde, BASE + 1_000);
        // A notificação do mesmo diálogo só refina: o aviso é o mesmo.
        rodar(&mut c, vec![chega(1_200, notificacao("permission_prompt"))]);
        assert_eq!(c.resumo().sessoes[0].aviso.unwrap().desde_ms, desde);
        // A ferramenta rodou: resolvido.
        rodar(
            &mut c,
            vec![chega(5_000, ferramenta("p1", "Bash", None, 10))],
        );
        assert_eq!(aviso_de(&c), None);
        // O Stop: o pronto só quando a acomodação acaba (com a festa).
        let saida = rodar(&mut c, vec![chega(9_000, stop("p1")), Ate(9_799)]);
        assert!(saida.is_empty());
        assert_eq!(aviso_de(&c), None, "acomodando");
        assert_eq!(rodar(&mut c, vec![Ate(9_800)]), vec![(9_800, PULINHO)]);
        let aviso = c.resumo().sessoes[0].aviso.unwrap();
        assert_eq!(
            (aviso.tipo, aviso.desde_ms),
            (TipoAviso::Pronto, BASE + 9_000),
            "desde o Stop"
        );
        // Um prompt novo resolve.
        rodar(&mut c, vec![chega(80_000, prompt("p2"))]);
        assert_eq!(aviso_de(&c), None);
        // Erro da API: o erro; o prompt seguinte resolve.
        rodar(&mut c, vec![chega(90_000, falhou("p2"))]);
        assert_eq!(aviso_de(&c), Some(TipoAviso::Erro));
        rodar(&mut c, vec![chega(95_000, prompt("p3"))]);
        assert_eq!(aviso_de(&c), None);
        // O idle_prompt fecha o turno (sem Stop) e para a sessão, mas não
        // resolve o "esperando você".
        rodar(
            &mut c,
            vec![
                chega(96_000, permissao("p3")),
                chega(160_000, notificacao("idle_prompt")),
            ],
        );
        assert_eq!(c.resumo().sessoes[0].estado, EstadoSessao::Parada);
        assert_eq!(aviso_de(&c), Some(TipoAviso::Esperando));
        // O fim da sessão leva o aviso junto.
        rodar(&mut c, vec![chega(170_000, ev("SessionEnd"))]);
        assert!(c.pendencias().is_empty());
    }

    #[test]
    fn evento_atrasado_nao_mexe_no_aviso_e_a_continuacao_tira_o_pronto() {
        let mut c = novo();
        // Uma permissão que chega depois da ferramenta que ela liberou (hooks
        // async fora de ordem): o estado muda (M3), o aviso não abre.
        rodar(
            &mut c,
            vec![
                chega(0, prompt("p1")),
                chega(5_000, ferramenta("p1", "Bash", None, 10)),
                atrasado(5_100, 4_000, permissao("p1")),
            ],
        );
        assert_eq!(aviso_de(&c), None);
        // O pronto; um Stop hook de outro plugin segura o Claude e a
        // continuação reabre o turno: o pronto sai, e volta no Stop seguinte.
        rodar(
            &mut c,
            vec![
                chega(6_000, ferramenta("p1", "Edit", Some("aaaaaaaaaaaa"), 10)),
                chega(7_000, stop("p1")),
                Ate(8_000),
            ],
        );
        assert_eq!(aviso_de(&c), Some(TipoAviso::Pronto));
        rodar(
            &mut c,
            vec![chega(12_000, ferramenta("p1", "Bash", None, 10))],
        );
        assert_eq!(aviso_de(&c), None, "trabalhando de novo");
        rodar(&mut c, vec![chega(15_000, stop("p1")), Ate(16_000)]);
        assert_eq!(aviso_de(&c), Some(TipoAviso::Pronto));
        // Uma permissão atrasada (de antes do Stop) não tira o pronto.
        rodar(&mut c, vec![atrasado(16_500, 14_000, permissao("p1"))]);
        assert_eq!(aviso_de(&c), Some(TipoAviso::Pronto));
    }

    #[test]
    fn o_pronto_e_o_erro_somem_em_2_h_e_o_esperando_fica() {
        let mut c = novo();
        rodar(
            &mut c,
            vec![chega(0, prompt("p1")), chega(100, stop("p1")), Ate(900)],
        );
        assert_eq!(aviso_de(&c), Some(TipoAviso::Pronto));
        assert_eq!(c.proximo_prazo(), Some(900 + VIDA_AVISO_MS));
        rodar(&mut c, vec![Ate(900 + VIDA_AVISO_MS - 1)]);
        assert_eq!(aviso_de(&c), Some(TipoAviso::Pronto));
        rodar(&mut c, vec![Ate(900 + VIDA_AVISO_MS)]);
        assert_eq!(aviso_de(&c), None, "2 h depois");
        let mut c = novo();
        rodar(
            &mut c,
            vec![chega(0, prompt("p1")), chega(100, permissao("p1"))],
        );
        assert_eq!(c.proximo_prazo(), Some(100 + VIDA_SESSAO_MS));
        rodar(&mut c, vec![Ate(VIDA_AVISO_MS + 1_000)]);
        assert_eq!(aviso_de(&c), Some(TipoAviso::Esperando));
    }

    #[test]
    fn pendencias_do_mais_urgente_ao_menos_urgente() {
        let mut c = novo();
        let de = |sid: &str, e: Evento| Evento {
            sid: Some(sid.into()),
            ..e
        };
        let teste = |e: Evento| Evento { teste: true, ..e };
        rodar(
            &mut c,
            vec![
                chega(0, de("a", prompt("pa"))),
                chega(100, de("a", stop("pa"))),
                chega(1_000, de("b", prompt("pb"))),
                chega(1_100, de("b", stop("pb"))),
                chega(2_000, de("c", prompt("pc"))),
                chega(2_100, de("c", falhou("pc"))),
                chega(3_000, teste(de("t", prompt("pt")))),
                chega(3_100, teste(de("t", permissao("pt")))),
                chega(4_000, de("d", prompt("pd"))),
                chega(4_100, de("d", permissao("pd"))),
                Ate(10_000),
            ],
        );
        let ordem: Vec<(String, TipoAviso)> = c
            .pendencias()
            .into_iter()
            .map(|p| (p.chave.1, p.aviso.tipo))
            .collect();
        let esperado: Vec<(String, TipoAviso)> = [
            ("d", TipoAviso::Esperando),
            ("t", TipoAviso::Esperando),
            ("c", TipoAviso::Erro),
            ("a", TipoAviso::Pronto),
            ("b", TipoAviso::Pronto),
        ]
        .into_iter()
        .map(|(s, t)| (s.to_owned(), t))
        .collect();
        assert_eq!(
            ordem, esperado,
            "a real antes da de teste; a mais velha primeiro"
        );
        assert_eq!(c.pendencias()[0].proj.as_deref(), Some("meu-projeto"));
        assert_eq!(c.ver(&(false, "d".into())), Some(TipoAviso::Esperando));
        assert_eq!(c.ver(&(false, "d".into())), None, "visto uma vez só");
        assert_eq!(c.pendencias()[0].chave.1, "t");
        // Visto, o "esperando você" do mesmo diálogo não volta com a
        // notificação dele.
        rodar(
            &mut c,
            vec![chega(11_000, de("d", notificacao("permission_prompt")))],
        );
        assert!(c.pendencias().iter().all(|p| p.chave.1 != "d"));
    }

    fn dialogo(e: &str, turno: &str, tool: &str) -> Evento {
        Evento {
            turno: Some(turno.into()),
            tool: Some(tool.into()),
            ..ev(e)
        }
    }

    fn espera_de(c: &Cerebro) -> Option<(TipoAviso, Option<TipoEspera>, u64)> {
        c.pendencias()
            .first()
            .map(|p| (p.aviso.tipo, p.aviso.espera, p.aviso.desde_ms - BASE))
    }

    #[test]
    fn um_dialogo_e_um_aviso_so_com_o_tipo_refinado() {
        // Os três gatilhos de uma pergunta no 2.1.288 (decisão 0071): o
        // PreToolUse, o PermissionRequest 14 ms depois e a notificação 6 s
        // depois. Um aviso só, desde o primeiro, do tipo pergunta.
        let mut c = novo();
        rodar(
            &mut c,
            vec![
                chega(0, prompt("p1")),
                chega(2_205, dialogo("PreToolUse", "p1", "AskUserQuestion")),
                chega(2_219, dialogo("PermissionRequest", "p1", "AskUserQuestion")),
                chega(8_217, notificacao("permission_prompt")),
            ],
        );
        assert_eq!(
            espera_de(&c),
            Some((TipoAviso::Esperando, Some(TipoEspera::Pergunta), 2_205))
        );
        // Respondida: a sessão andou, o aviso sai.
        rodar(
            &mut c,
            vec![chega(18_964, ferramenta("p1", "AskUserQuestion", None, 0))],
        );
        assert_eq!(espera_de(&c), None);
        // O plano: plano, e a notificação não desce para permissão.
        rodar(
            &mut c,
            vec![
                chega(20_000, dialogo("PreToolUse", "p1", "ExitPlanMode")),
                chega(20_021, dialogo("PermissionRequest", "p1", "ExitPlanMode")),
                chega(26_000, notificacao("permission_prompt")),
            ],
        );
        assert_eq!(
            espera_de(&c),
            Some((TipoAviso::Esperando, Some(TipoEspera::Plano), 20_000))
        );
        rodar(
            &mut c,
            vec![chega(30_000, ferramenta("p1", "ExitPlanMode", None, 1))],
        );
        // A notificação chegou antes do PreToolUse (hooks async): a permissão
        // sobe para pergunta, no mesmo aviso.
        rodar(
            &mut c,
            vec![
                chega(40_000, notificacao("permission_prompt")),
                chega(40_030, dialogo("PreToolUse", "p1", "AskUserQuestion")),
            ],
        );
        assert_eq!(
            espera_de(&c),
            Some((TipoAviso::Esperando, Some(TipoEspera::Pergunta), 40_000))
        );
        rodar(
            &mut c,
            vec![chega(45_000, ferramenta("p1", "AskUserQuestion", None, 0))],
        );
        // Um gatilho que saiu até 5 s antes do aviso (fora de ordem) refina;
        // um mais velho que isso, não.
        rodar(
            &mut c,
            vec![
                chega(50_000, notificacao("elicitation_dialog")),
                atrasado(50_100, 46_000, dialogo("PreToolUse", "p1", "ExitPlanMode")),
            ],
        );
        assert_eq!(
            espera_de(&c),
            Some((TipoAviso::Esperando, Some(TipoEspera::Plano), 50_000))
        );
        rodar(
            &mut c,
            vec![
                chega(51_000, ferramenta("p1", "ExitPlanMode", None, 1)),
                chega(60_000, permissao("p1")),
                atrasado(
                    60_100,
                    54_000,
                    dialogo("PreToolUse", "p1", "AskUserQuestion"),
                ),
            ],
        );
        assert_eq!(
            espera_de(&c),
            Some((TipoAviso::Esperando, Some(TipoEspera::Permissao), 60_000))
        );
        // Um evento de subagente não refina (ele não abre diálogo do Renan).
        let do_agente = Evento {
            agente: true,
            aid: Some("x1".into()),
            ..dialogo("PermissionRequest", "p1", "AskUserQuestion")
        };
        rodar(&mut c, vec![chega(61_000, do_agente)]);
        assert_eq!(espera_de(&c).unwrap().1, Some(TipoEspera::Permissao));
        assert_eq!(
            serde_json::to_value(c.pendencias()[0].aviso).unwrap()["espera"],
            "permissao"
        );
    }

    #[test]
    fn o_tipo_de_espera_de_cada_gatilho() {
        let n = |nt: &str| espera_do_evento(&notificacao(nt));
        assert_eq!(n("permission_prompt"), Some(TipoEspera::Permissao));
        assert_eq!(n("worker_permission_prompt"), Some(TipoEspera::Permissao));
        assert_eq!(n("agent_needs_input"), Some(TipoEspera::Permissao));
        assert_eq!(n("elicitation_dialog"), Some(TipoEspera::Elicitacao));
        assert_eq!(n("elicitation_url_dialog"), Some(TipoEspera::Elicitacao));
        assert_eq!(n("idle_prompt"), None);
        assert_eq!(
            espera_do_evento(&dialogo("PermissionRequest", "p", "Bash")),
            Some(TipoEspera::Permissao)
        );
        assert_eq!(
            espera_do_evento(&dialogo("PermissionRequest", "p", "ExitPlanMode")),
            Some(TipoEspera::Plano)
        );
        assert_eq!(espera_do_evento(&prompt("p")), None);
        assert!(TipoEspera::Pergunta > TipoEspera::Plano);
        assert!(TipoEspera::Plano > TipoEspera::Elicitacao);
        assert!(TipoEspera::Elicitacao > TipoEspera::Permissao);
    }

    // --- correntes e turnos de máquina (decisão 0073) ------------------------

    /// O prompt com que o Claude Code acorda a sessão quando uma tarefa em
    /// segundo plano termina (o `orig` que o hook calcula; decisão 0072).
    fn aviso_de_tarefa(turno: &str) -> Evento {
        Evento {
            src: None,
            orig: Some(ORIG_NOTIFICACAO.into()),
            ..prompt(turno)
        }
    }

    /// Um prompt do hook do M5 (`orig = comum`), sem `src` (o 2.1.288).
    fn comum(turno: &str) -> Evento {
        Evento {
            src: None,
            orig: Some("comum".into()),
            ..prompt(turno)
        }
    }

    /// Um prompt do hook de antes do M5: sem `src` e sem `orig`.
    fn antigo(turno: &str) -> Evento {
        Evento {
            src: None,
            ..prompt(turno)
        }
    }

    /// Um Stop com as tarefas em segundo plano (tipo, id) e os agendamentos.
    fn stop_com(turno: &str, tarefas: &[(&str, &str)], crn: u64) -> Evento {
        Evento {
            bg: Some(tarefas.len() as u64),
            bgt: tarefas.iter().map(|(t, _)| (*t).to_owned()).collect(),
            bgi: tarefas.iter().map(|(_, i)| (*i).to_owned()).collect(),
            crn: Some(crn),
            ..stop(turno)
        }
    }

    fn nasce(turno: &str, aid: &str) -> Evento {
        Evento {
            turno: Some(turno.into()),
            agente: true,
            aid: Some(aid.into()),
            ..ev("SubagentStart")
        }
    }

    fn do_agente(turno: &str, aid: &str, tool: &str, dur: u64) -> Evento {
        Evento {
            agente: true,
            aid: Some(aid.into()),
            ..ferramenta(turno, tool, None, dur)
        }
    }

    #[test]
    fn a_origem_do_prompt() {
        use OrigemTurno::*;
        let longe = Evidencia {
            ausente: true,
            outra_janela: false,
        };
        let outra = Evidencia {
            ausente: false,
            outra_janela: true,
        };
        let nada = Evidencia::default();
        for (src, orig, corrente, crons, evidencia, esperado) in [
            // O `source`, se um Claude Code mandar, vale primeiro.
            (
                Some("user"),
                Some(ORIG_NOTIFICACAO),
                true,
                Some(1),
                longe,
                Digitado,
            ),
            (Some("system"), None, false, None, nada, Sistema),
            (Some("loop_wakeup"), None, false, None, nada, Tique),
            (Some("schedule_wakeup"), None, false, None, nada, Tique),
            (Some("poll_event"), None, false, None, nada, Tique),
            // O hook do M5: a notificação pela forma.
            (None, Some(ORIG_NOTIFICACAO), false, None, nada, Notificacao),
            (None, Some("comum"), true, None, nada, Digitado),
            // Tique: só com agendamento pendente e prova de que não foi
            // digitado.
            (None, Some("comum"), false, Some(1), longe, Tique),
            (None, Some("comum"), false, Some(2), outra, Tique),
            (None, Some("comum"), false, Some(1), nada, Digitado),
            (None, Some("comum"), false, Some(0), longe, Digitado),
            (None, Some("comum"), false, None, longe, Digitado),
            // O hook antigo: sem `orig`, a corrente aberta decide.
            (None, None, true, None, nada, Notificacao),
            (None, None, false, Some(3), longe, Digitado),
            // Um `orig` de um hook mais novo vale como comum.
            (None, Some("outra_forma"), false, None, nada, Digitado),
        ] {
            assert_eq!(
                classificar(src, orig, corrente, crons, evidencia),
                esperado,
                "{src:?} {orig:?} corrente={corrente} crons={crons:?} {evidencia:?}"
            );
        }
    }

    #[test]
    fn o_agente_em_segundo_plano_festeja_uma_vez_so_no_fim() {
        // A sequência real da pesquisa (decisão 0071): o pedido lança um
        // agente em segundo plano e acaba com ele em voo; o agente deixa um
        // shell rodando; a notificação acorda o turno principal com o shell
        // ainda em voo; o agente acorda quando o shell acaba e, no fim, outra
        // notificação. Uma festa só, no último Stop, pela soma.
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, comum("p3")),
                chega(2_199, nasce("p3", "x1")),
                chega(2_205, ferramenta("p3", "Agent", None, 6)),
                chega(3_624, stop_com("p3", &[("subagent", "x1")], 0)),
                chega(7_441, do_agente("p3", "x1", "Bash", 41)),
                chega(8_680, aviso_de_tarefa("p4")),
                chega(11_587, stop_com("p4", &[("shell", "x2")], 0)),
                chega(32_534, nasce("p4", "x1")),
                chega(33_347, aviso_de_tarefa("p5")),
                chega(34_682, stop_com("p5", &[], 0)),
                Ate(40_000),
            ],
        );
        // O Stop de p4 não tem agente em voo (o shell é do agente, mas o fio
        // não diz de quem é): a corrente fecha em p4, com a festa; p5, a
        // última notificação, é um turno de máquina sem corrente, e o T0
        // dele não reage.
        assert_eq!(saida, vec![(12_387, PULINHO)]);
        let r = c.resumo();
        assert!(r.ignorados.is_empty(), "{:?}", r.ignorados);
        let p3 = registro(&r, "p3");
        assert_eq!(
            (p3.fim, p3.nivel, p3.reacao, p3.corrente),
            (
                Fim::Stop,
                None,
                None,
                Some(ResumoCorrente {
                    aberta: true,
                    turnos: 1
                })
            ),
            "o pedido entra na corrente sem festa"
        );
        let p4 = registro(&r, "p4");
        assert_eq!(
            (p4.fim, p4.nivel, p4.reacao, p4.origem),
            (
                Fim::Stop,
                Some(Nivel::T1),
                Some(PULINHO),
                OrigemTurno::Notificacao
            )
        );
        assert_eq!(
            p4.corrente,
            Some(ResumoCorrente {
                aberta: false,
                turnos: 2
            })
        );
        // A soma: o Agent e o Bash do agente (depois do Stop do pedido), um
        // subagente (o que acordou não conta de novo), o t0 do pedido.
        assert_eq!(
            (
                p4.trabalho,
                p4.outras,
                p4.subagentes,
                p4.de_agentes,
                p4.dur_ms
            ),
            (1, 1, 1, 1, 47)
        );
        assert_eq!(p4.t0_ms, BASE, "o t0 do primeiro turno");
        assert!(
            p4.teto.is_empty(),
            "a corrente nasceu de um prompt digitado"
        );
        let p5 = registro(&r, "p5");
        assert_eq!(
            (p5.nivel, p5.reacao, p5.origem, p5.teto.clone()),
            (
                Some(Nivel::T0),
                None,
                OrigemTurno::Notificacao,
                vec!["maquina"]
            )
        );
        // O pronto é o do fim da corrente; a notificação de p5 não o tira
        // nem abre outro.
        let aviso = c.resumo().sessoes[0].aviso.unwrap();
        assert_eq!(
            (aviso.tipo, aviso.desde_ms),
            (TipoAviso::Pronto, BASE + 11_587)
        );
    }

    #[test]
    fn a_corrente_espera_o_ultimo_agente_e_soma_o_trabalho_dele() {
        // Dois agentes em segundo plano; o Stop do turno que volta com um
        // deles ainda em voo não festeja; o último fecha.
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, comum("p1")),
                chega(1_000, nasce("p1", "a1")),
                chega(1_100, nasce("p1", "a2")),
                chega(
                    2_000,
                    stop_com("p1", &[("subagent", "a1"), ("workflow", "w1")], 0),
                ),
                chega(5_000, do_agente("p1", "a1", "Edit", 30)),
                chega(6_000, do_agente("p1", "a2", "Write", 20)),
                chega(9_000, aviso_de_tarefa("p2")),
                chega(10_000, ferramenta("p2", "Edit", Some("aaaaaaaaaaa1"), 40)),
                chega(12_000, stop_com("p2", &[("workflow", "w1")], 0)),
                chega(60_000, aviso_de_tarefa("p3")),
                chega(62_000, stop_com("p3", &[("shell", "s9")], 0)),
                Ate(70_000),
            ],
        );
        assert_eq!(saida, vec![(62_800, PULINHO)], "uma festa, no fim");
        let r = c.resumo();
        let p3 = registro(&r, "p3");
        assert_eq!(
            p3.corrente,
            Some(ResumoCorrente {
                aberta: false,
                turnos: 3
            })
        );
        assert_eq!(
            (p3.trabalho, p3.subagentes, p3.de_agentes, p3.arquivos),
            (3, 2, 2, 1)
        );
        assert_eq!(
            registro(&r, "p2").corrente,
            Some(ResumoCorrente {
                aberta: true,
                turnos: 2
            })
        );
        assert!(c.resumo().sessoes[0].corrente.is_none(), "fechou");
    }

    #[test]
    fn shell_e_monitor_nunca_seguram_a_festa() {
        // Um servidor de desenvolvimento rodando em segundo plano: todo Stop
        // vem com `bg ≥ 1`, e as festas são as de sempre.
        let mut c = novo();
        let servidor = [("shell", "srv"), ("monitor", "mon"), ("dream", "d1")];
        let saida = rodar(
            &mut c,
            vec![
                chega(0, comum("p1")),
                chega(1_000, ferramenta("p1", "Bash", None, 14)),
                chega(2_000, stop_com("p1", &servidor, 0)),
                chega(10_000, comum("p2")),
                chega(11_000, stop_com("p2", &servidor, 0)),
                Ate(15_000),
            ],
        );
        assert_eq!(saida, vec![(2_800, PULINHO), (11_800, ACENO)]);
        assert!(c.resumo().sessoes[0].corrente.is_none());
    }

    #[test]
    fn turno_de_maquina_sem_corrente_e_discreto_e_nao_mexe_no_pronto() {
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                // O pedido do Renan, com o pronto.
                chega(0, comum("p1")),
                chega(1_000, ferramenta("p1", "Edit", Some("aaaaaaaaaaa1"), 30)),
                chega(2_000, stop_com("p1", &[("shell", "s1")], 0)),
                Ate(3_000),
                // O shell acaba: a notificação acorda a sessão, que edita.
                chega(30_000, aviso_de_tarefa("p2")),
                chega(31_000, ferramenta("p2", "Edit", Some("aaaaaaaaaaa2"), 30)),
                chega(31_500, ferramenta("p2", "Bash", None, 900)),
                chega(33_000, stop_com("p2", &[], 0)),
                Ate(34_000),
                // Outra notificação, sem trabalho: T0 de máquina, calado.
                chega(50_000, aviso_de_tarefa("p3")),
                chega(51_000, stop_com("p3", &[], 0)),
                Ate(60_000),
            ],
        );
        assert_eq!(saida, vec![(2_800, PULINHO), (33_800, PULINHO)]);
        let r = c.resumo();
        let p2 = registro(&r, "p2");
        assert_eq!(
            (p2.nivel, p2.origem, p2.teto.clone()),
            (Some(Nivel::T1), OrigemTurno::Notificacao, vec!["maquina"])
        );
        assert!(c.resumo().ultima_reacao.unwrap().discreta);
        // O pronto continua o de p1: as notificações não o tiraram nem
        // abriram outro.
        let aviso = c.resumo().sessoes[0].aviso.unwrap();
        assert_eq!(
            (aviso.tipo, aviso.desde_ms),
            (TipoAviso::Pronto, BASE + 2_000)
        );
        // Um prompt digitado resolve.
        rodar(&mut c, vec![chega(70_000, comum("p4"))]);
        assert_eq!(aviso_de(&c), None);
    }

    #[test]
    fn com_o_hook_antigo_o_prompt_com_corrente_aberta_e_continuacao() {
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, antigo("p1")),
                chega(500, nasce("p1", "a1")),
                chega(1_000, stop_com("p1", &[("subagent", "a1")], 0)),
                Ate(2_000),
                // A notificação, no hook antigo: um prompt sem marca.
                chega(9_000, antigo("p2")),
                chega(10_000, stop_com("p2", &[], 0)),
                Ate(11_000),
                // Sem corrente, um prompt sem marca é digitado.
                chega(20_000, antigo("p3")),
                chega(21_000, stop_com("p3", &[], 0)),
                Ate(22_000),
            ],
        );
        assert_eq!(saida, vec![(10_800, PULINHO), (21_800, ACENO)]);
        let r = c.resumo();
        assert_eq!(registro(&r, "p2").origem, OrigemTurno::Notificacao);
        assert_eq!(registro(&r, "p3").origem, OrigemTurno::Digitado);
    }

    #[test]
    fn o_agente_que_acorda_nao_reabre_nem_segura_a_acomodacao() {
        // O SubagentStart de um agente já visto, depois do Stop, é o agente
        // acordando em segundo plano: não cancela a acomodação nem reabre o
        // turno comemorado (o M3 o tomava pela thread principal).
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, comum("p1")),
                chega(500, nasce("p1", "a1")),
                chega(1_000, stop_com("p1", &[], 0)),
                chega(1_300, nasce("p1", "a1")),
                Ate(5_000),
                chega(6_000, nasce("p1", "a1")),
                Ate(8_000),
            ],
        );
        assert_eq!(saida, vec![(1_800, PULINHO)]);
        let r = c.resumo();
        assert_eq!(
            (
                registro(&r, "p1").continuacoes,
                registro(&r, "p1").subagentes
            ),
            (0, 1)
        );
        assert!(!r.sessoes[0].turno_aberto);
        // Um agente novo depois do Stop continua sendo a thread principal.
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, comum("p1")),
                chega(1_000, stop_com("p1", &[], 0)),
                chega(1_300, nasce("p1", "a9")),
                Ate(5_000),
            ],
        );
        assert!(saida.is_empty(), "{saida:?}");
    }

    #[test]
    fn a_corrente_expira_em_12_h_e_o_fim_da_sessao_a_fecha_sem_festa() {
        let mut c = novo();
        let abrir = || {
            vec![
                chega(0, comum("p1")),
                chega(1_000, stop_com("p1", &[("subagent", "a1")], 0)),
                Ate(2_000),
            ]
        };
        assert!(rodar(&mut c, abrir()).is_empty());
        // A sessão continua viva (o Renan usa o terminal, o idle_prompt), e
        // o agente nunca volta: a corrente expira 12 h depois do último
        // evento dela (o fechamento do Stop, na acomodação).
        let ocioso = Evento {
            nt: Some("idle_prompt".into()),
            ..ev("Notification")
        };
        let hora = 60 * 60 * 1000;
        let saida = rodar(
            &mut c,
            vec![
                chega(6 * hora, ocioso.clone()),
                chega(11 * hora, ocioso),
                Ate(1_800 + VIDA_CORRENTE_MS - 1),
            ],
        );
        assert!(saida.is_empty());
        assert!(c.resumo().sessoes[0].corrente.is_some(), "ainda aberta");
        assert_eq!(c.proximo_prazo(), Some(1_800 + VIDA_CORRENTE_MS));
        let saida = rodar(&mut c, vec![Ate(1_800 + VIDA_CORRENTE_MS)]);
        assert!(saida.is_empty());
        let r = c.resumo();
        assert_eq!(r.turnos[0].fim, Fim::CorrenteExpirou);
        assert_eq!(
            r.turnos[0].corrente,
            Some(ResumoCorrente {
                aberta: false,
                turnos: 1
            })
        );
        // O fim da sessão fecha a corrente: registro, sem festa nem tchau
        // de corrente (o tchau é da sessão).
        let mut c = novo();
        rodar(&mut c, abrir());
        let saida = rodar(&mut c, vec![chega(5_000, ev("SessionEnd"))]);
        assert_eq!(saida, vec![(5_000, TCHAU)]);
        let r = c.resumo();
        assert_eq!(
            (r.turnos[0].fim, r.turnos[0].reacao),
            (Fim::SessaoEncerrada, None)
        );
    }

    #[test]
    fn o_tique_do_laco_com_o_renan_longe_e_de_maquina() {
        let mut c = novo();
        let longe = Evidencia {
            ausente: true,
            outra_janela: false,
        };
        // O /loop agenda um cron: o Stop traz crn = 1.
        rodar(
            &mut c,
            vec![
                chega(0, comum("p1")),
                chega(1_000, ferramenta("p1", "CronCreate", None, 1)),
                chega(2_000, stop_com("p1", &[], 1)),
                Ate(3_000),
            ],
        );
        assert_eq!(c.resumo().sessoes[0].agendamentos, Some(1));
        // O tique chega com o Renan longe: de máquina, discreto, sem pronto.
        let tique = comum("p2");
        c.receber_com(&tique, BASE + 60_000, em(60_000), longe);
        c.receber(
            &ferramenta("p2", "Edit", Some("aaaaaaaaaaa1"), 20),
            BASE + 61_000,
            em(61_000),
        );
        c.receber(&stop_com("p2", &[], 1), BASE + 62_000, em(62_000));
        let saida = c.tique(em(62_800));
        assert_eq!(saida.len(), 1);
        assert!(saida[0].discreta);
        let r = c.resumo();
        assert_eq!(registro(&r, "p2").origem, OrigemTurno::Tique);
        // Com o Renan no teclado e na janela certa, é digitado.
        c.receber_com(
            &comum("p3"),
            BASE + 120_000,
            em(120_000),
            Evidencia::default(),
        );
        c.receber(&stop_com("p3", &[], 1), BASE + 121_000, em(121_000));
        let saida = c.tique(em(121_800));
        assert!(!saida[0].discreta);
        assert_eq!(registro(&c.resumo(), "p3").origem, OrigemTurno::Digitado);
    }

    #[test]
    fn a_corrente_que_nasce_de_maquina_fica_no_teto() {
        // Uma notificação sem corrente lança um agente: a corrente nasce de
        // máquina e a festa dela é discreta, sem pronto.
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, aviso_de_tarefa("p1")),
                chega(500, nasce("p1", "a1")),
                chega(1_000, stop_com("p1", &[("subagent", "a1")], 0)),
                chega(5_000, do_agente("p1", "a1", "Edit", 30)),
                chega(9_000, aviso_de_tarefa("p2")),
                chega(10_000, stop_com("p2", &[], 0)),
                Ate(12_000),
            ],
        );
        assert_eq!(saida, vec![(10_800, PULINHO)]);
        assert!(c.resumo().ultima_reacao.unwrap().discreta);
        assert_eq!(aviso_de(&c), None, "sem pronto");
    }

    // --- pontuação e níveis (decisão 0074) -----------------------------------

    /// `n` ferramentas `tool` de `dur` ms cada, em arquivos de `0` a `arqs-1`
    /// (se `arqs > 0`), no turno `turno`, a partir de `t`.
    fn varias(turno: &str, tool: &str, n: u32, dur: u64, arqs: u32, t: u64) -> Vec<Passo> {
        (0..n)
            .map(|i| {
                let arq =
                    (arqs > 0).then(|| format!("{:012x}", 0xc00000000000u64 + u64::from(i % arqs)));
                chega(
                    t + u64::from(i),
                    ferramenta(turno, tool, arq.as_deref(), dur),
                )
            })
            .collect()
    }

    fn turno_com(trabalho: Vec<Passo>) -> (Saida, RegistroTurno) {
        let mut c = novo();
        let mut roteiro = vec![chega(0, prompt("p1"))];
        roteiro.extend(trabalho);
        roteiro.push(chega(500_000, stop("p1")));
        roteiro.push(Ate(510_000));
        let saida = rodar(&mut c, roteiro);
        let r = c.resumo();
        (saida, registro(&r, "p1").clone())
    }

    #[test]
    fn exemplos_de_pontuacao() {
        // Os exemplos da decisão 0074.
        let (saida, r) = turno_com(vec![]);
        assert_eq!((saida, r.nivel), (vec![(500_800, ACENO)], Some(Nivel::T0)));
        let mut leitura = varias("p1", "Read", 15, 5, 0, 1_000);
        leitura.extend(varias("p1", "WebFetch", 2, 2_000, 0, 2_000));
        let (saida, r) = turno_com(leitura);
        assert_eq!(saida, vec![(500_800, ACENO)], "nada de trabalho: T0");
        assert_eq!(r.pontuacao.unwrap().total, 0.92, "pontua, mas é T0");
        let mut pequeno = varias("p1", "Edit", 1, 40, 1, 1_000);
        pequeno.extend(varias("p1", "Bash", 1, 900, 0, 2_000));
        let (saida, r) = turno_com(pequeno);
        assert_eq!(saida, vec![(500_800, PULINHO)]);
        let p = r.pontuacao.unwrap();
        assert_eq!(
            (p.total, p.min_ativos, p.trabalho, p.arquivos),
            (0.82, 0.02, 0.3, 0.5)
        );
        // 10 Edit em 5 arquivos, 8 Bash e 7 Read com 3 min de ferramenta.
        let mut medio = varias("p1", "Edit", 10, 0, 5, 1_000);
        medio.extend(varias("p1", "Bash", 8, 22_500, 0, 2_000));
        medio.extend(varias("p1", "Read", 7, 0, 0, 3_000));
        let (saida, r) = turno_com(medio);
        assert_eq!(saida, vec![(500_800, VOO_CURTO)]);
        assert_eq!(r.pontuacao.unwrap().total, 8.55);
        // 40 Edit em 15 arquivos, 30 Bash, 50 Read e 2 subagentes com 12 min.
        let mut grande = varias("p1", "Edit", 40, 0, 15, 1_000);
        grande.extend(varias("p1", "Bash", 30, 24_000, 0, 2_000));
        grande.extend(varias("p1", "Read", 50, 0, 0, 3_000));
        grande.push(chega(4_000, nasce("p1", "a1")));
        grande.push(chega(4_001, nasce("p1", "a2")));
        let (saida, r) = turno_com(grande);
        assert_eq!(saida, vec![(500_800, VOO_GRANDE)]);
        let p = r.pontuacao.unwrap();
        assert_eq!((p.total, p.min_ativos), (20.0, 12.0), "no teto de 20");
        assert_eq!(r.nivel_calculado, Some(Nivel::T3));
    }

    #[test]
    fn o_tempo_do_agent_fica_fora_dos_minutos() {
        // Um Agent em primeiro plano de 10 min (a vida do subagente, com o
        // pensar dele) não vira 10 min de trabalho; as ferramentas do
        // subagente contam pelo aid.
        let mut trabalho = vec![chega(1_000, nasce("p1", "a1"))];
        trabalho.push(chega(2_000, do_agente("p1", "a1", "Bash", 60_000)));
        trabalho.push(chega(600_000 - 1, ferramenta("p1", "Agent", None, 600_000)));
        let mut c = novo();
        let mut roteiro = vec![chega(0, prompt("p1"))];
        roteiro.extend(trabalho);
        roteiro.push(chega(600_100, stop("p1")));
        roteiro.push(Ate(610_000));
        rodar(&mut c, roteiro);
        let r = c.resumo();
        let p1 = registro(&r, "p1");
        assert_eq!(p1.dur_ms, 660_000, "o registro mostra todo o tempo");
        assert_eq!(p1.pontuacao.unwrap().min_ativos, 1.0, "só o Bash do agente");
    }

    #[test]
    fn o_t3_no_maximo_a_cada_10_min() {
        let grande = |turno: &str, t: u64| {
            let mut r = vec![chega(t, prompt(turno))];
            for i in 0..12u64 {
                r.push(chega(
                    t + 1 + i,
                    ferramenta(turno, "Edit", Some(&format!("{:012x}", t + i)), 60_000),
                ));
            }
            r.push(chega(t + 1_000, stop(turno)));
            r
        };
        let mut c = novo();
        let mut roteiro = grande("p1", 0);
        roteiro.extend(grande("p2", 60_000));
        roteiro.extend(grande("p3", 60_000 + 10 * 60 * 1000));
        roteiro.push(Ate(60_000 + 10 * 60 * 1000 + 5_000));
        assert_eq!(
            rodar(&mut c, roteiro),
            vec![
                (1_800, VOO_GRANDE),
                (61_800, VOO_CURTO),
                (60_000 + 10 * 60 * 1000 + 1_800, VOO_GRANDE)
            ]
        );
        let r = c.resumo();
        let p2 = registro(&r, "p2");
        assert_eq!(
            (p2.nivel_calculado, p2.nivel, p2.teto.clone()),
            (Some(Nivel::T3), Some(Nivel::T2), vec!["intervalo_t3"])
        );
        // Sem limite (intervalo 0): dois T3 seguidos.
        let mut c = Cerebro::novo(ConfigCerebro {
            intervalo_t3_ms: 0,
            ..ConfigCerebro::default()
        });
        let mut roteiro = grande("p1", 0);
        roteiro.extend(grande("p2", 60_000));
        roteiro.push(Ate(70_000));
        assert_eq!(
            rodar(&mut c, roteiro),
            vec![(1_800, VOO_GRANDE), (61_800, VOO_GRANDE)]
        );
    }

    #[test]
    fn a_continuacao_so_festeja_se_subir_tambem_em_t2_e_t3() {
        // O Stop com sha de um turno que já festejou em T1 sobe para T2 com
        // o trabalho da continuação; outra continuação que fica em T2 não
        // festeja de novo.
        let mut c = novo();
        let mut roteiro = vec![
            chega(0, prompt("q1")),
            chega(500, ferramenta("q1", "Edit", Some("aaaaaaaaaaa1"), 40)),
            chega(1_000, stop("q1")),
            Ate(2_000),
        ];
        for i in 0..9u64 {
            roteiro.push(chega(
                3_000 + i,
                ferramenta("q1", "Edit", Some(&format!("{:012x}", 0xd0 + i)), 20_000),
            ));
        }
        roteiro.push(chega(
            6_000,
            Evento {
                sha: true,
                ..stop("q1")
            },
        ));
        roteiro.push(Ate(7_000));
        roteiro.push(chega(8_000, ferramenta("q1", "Read", None, 5)));
        roteiro.push(chega(
            9_000,
            Evento {
                sha: true,
                ..stop("q1")
            },
        ));
        roteiro.push(Ate(10_000));
        assert_eq!(
            rodar(&mut c, roteiro),
            vec![(1_800, PULINHO), (6_800, VOO_CURTO)]
        );
        let r = c.resumo();
        let q1 = registro(&r, "q1");
        assert_eq!(
            (q1.nivel, q1.reacao, q1.continuacoes),
            (Some(Nivel::T2), Some(VOO_CURTO), 2)
        );
    }

    // --- prazos do estado e o cansado (decisão 0076) -------------------------

    #[test]
    fn trabalhando_sem_evento_volta_a_parado_em_5_min_com_o_turno_aberto() {
        let mut c = novo();
        let estado = |c: &Cerebro| {
            (
                c.resumo().sessoes[0].estado,
                c.resumo().sessoes[0].turno_aberto,
            )
        };
        rodar(
            &mut c,
            vec![
                chega(0, prompt("p1")),
                chega(1_000, ferramenta("p1", "Bash", None, 10)),
                Ate(1_000 + ATIVA_SEM_EVENTO_MS - 1),
            ],
        );
        assert_eq!(estado(&c), (EstadoSessao::Trabalhando, true));
        rodar(&mut c, vec![Ate(1_000 + ATIVA_SEM_EVENTO_MS)]);
        assert_eq!(estado(&c), (EstadoSessao::Parada, true), "o turno continua");
        // Um build de 10 min: o PostToolUse dele chega e a sessão volta a
        // trabalhar; o Stop festeja com tudo (10,3 pontos: T2).
        let saida = rodar(
            &mut c,
            vec![
                chega(601_000, ferramenta("p1", "Bash", None, 600_000)),
                chega(602_000, stop("p1")),
                Ate(603_000),
            ],
        );
        assert_eq!(saida, vec![(602_800, VOO_CURTO)]);
        // Compactando também tem prazo: o PostCompact pode não vir.
        let mut c = novo();
        rodar(
            &mut c,
            vec![chega(0, prompt("p1")), chega(1_000, ev("PreCompact"))],
        );
        assert_eq!(c.resumo().sessoes[0].estado, EstadoSessao::Compactando);
        rodar(&mut c, vec![Ate(1_000 + ATIVA_SEM_EVENTO_MS)]);
        assert_eq!(c.resumo().sessoes[0].estado, EstadoSessao::Parada);
        // Esperando você não volta sozinho.
        let mut c = novo();
        rodar(
            &mut c,
            vec![
                chega(0, prompt("p1")),
                chega(1_000, permissao("p1")),
                Ate(10 * ATIVA_SEM_EVENTO_MS),
            ],
        );
        assert_eq!(c.resumo().sessoes[0].estado, EstadoSessao::Esperando);
    }

    #[test]
    fn o_limite_de_uso_cansa_e_o_erro_fica_60_s_na_tela() {
        let falha = |turno: &str, err: &str| Evento {
            turno: Some(turno.into()),
            err: Some(err.into()),
            ..ev("StopFailure")
        };
        let mut c = novo();
        let saida = rodar(
            &mut c,
            vec![
                chega(0, prompt("p1")),
                chega(1_000, edit_teste("p1")),
                chega(2_000, falha("p1", ERRO_DE_LIMITE)),
                Ate(2_000 + ERRO_NA_TELA_MS - 1),
            ],
        );
        assert!(saida.is_empty(), "sem festa: {saida:?}");
        let s = &c.resumo().sessoes[0];
        assert_eq!(s.estado, EstadoSessao::Cansado);
        assert_eq!(s.aviso.unwrap().tipo, TipoAviso::Erro, "o aviso de erro");
        rodar(&mut c, vec![Ate(2_000 + ERRO_NA_TELA_MS)]);
        let s = &c.resumo().sessoes[0];
        assert_eq!(s.estado, EstadoSessao::Parada);
        assert_eq!(
            s.aviso.unwrap().tipo,
            TipoAviso::Erro,
            "o aviso fica (sai visto ou em 2 h)"
        );
        assert_eq!(registro(&c.resumo(), "p1").fim, Fim::Falhou);
        // Outro erro da API: erro, também 60 s.
        let mut c = novo();
        rodar(
            &mut c,
            vec![
                chega(0, prompt("p1")),
                chega(500, falha("p1", "overloaded")),
            ],
        );
        assert_eq!(c.resumo().sessoes[0].estado, EstadoSessao::Erro);
        assert_eq!(c.proximo_prazo(), Some(500 + ERRO_NA_TELA_MS));
    }
}
