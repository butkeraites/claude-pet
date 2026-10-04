//! Cérebro mínimo (M3, decisões 0020 e 0032): eventos do Claude Code →
//! reações.
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
//! o pulinho. T2/T3, correntes de tarefas em segundo plano, escalada e
//! presença são do M5; por isso cada turno guarda os componentes inteiros
//! em [`RegistroTurno`] (o `/v1/estado.turnos`), para a pontuação do M5.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use crate::config::{Config, ModoCelebracao};
use crate::evento::{self, Evento};

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
/// Turnos fechados no `/v1/estado.turnos`.
pub const TURNOS_GUARDADOS: usize = 20;
/// Arquivos diferentes lembrados por turno. Passou disso, cada arquivo novo
/// só soma num contador: um processo local mandando `arq` sempre novo não
/// cresce a memória.
pub const ARQUIVOS_POR_TURNO: usize = 1024;
/// Turnos fechados lembrados por sessão, para o dedupe do Stop.
const FECHADOS_POR_SESSAO: usize = 32;
/// Subagentes lembrados por sessão (para atribuir ferramentas ao turno).
const AGENTES_POR_SESSAO: usize = 32;
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

/// Aceno discreto (T0).
pub const ACENO: &str = "nod";
/// Pulinho (T1).
pub const PULINHO: &str = "done_small";
/// Tchau: a última sessão acabou.
pub const TCHAU: &str = "bye";

/// O relógio, injetado: parede (ms desde 1970, o mesmo do `ts` dos hooks)
/// para horas de evento e de reação; monotônico (ms) para os prazos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agora {
    pub parede_ms: u64,
    pub mono_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigCerebro {
    /// Origens aceitas (`$CLAUDE_CODE_ENTRYPOINT`); evento sem origem não
    /// conta.
    pub origens: Vec<String>,
    pub modo: ModoCelebracao,
}

impl Default for ConfigCerebro {
    fn default() -> Self {
        ConfigCerebro {
            origens: vec!["cli".into()],
            modo: ModoCelebracao::Proporcional,
        }
    }
}

impl ConfigCerebro {
    pub fn de(config: &Config) -> Self {
        ConfigCerebro {
            origens: config.sessoes_origens.clone(),
            modo: config.celebracao_modo,
        }
    }
}

/// Nível da festa de um turno (T0 < T1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Nivel {
    T0,
    T1,
}

/// Uma reação para o pet tocar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reacao {
    /// `nod`, `done_small` ou `bye`.
    pub nome: &'static str,
    /// Os 8 primeiros caracteres do `sid`.
    pub sid8: String,
    pub proj: Option<String>,
    /// Hora da reação (ms desde 1970).
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nivel: Option<Nivel>,
    pub teste: bool,
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
}

/// Os componentes de um turno fechado: o que o M5 vai pontuar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegistroTurno {
    pub sid8: String,
    /// Os 8 primeiros caracteres do `prompt_id`.
    pub turno8: Option<String>,
    pub proj: Option<String>,
    pub teste: bool,
    /// Aberto por um evento qualquer, sem o `UserPromptSubmit`.
    pub implicito: bool,
    /// `source` do `UserPromptSubmit` (`user`, `system`, …).
    pub src: Option<String>,
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
    pub nivel: Option<Nivel>,
    /// A última reação tocada por este turno.
    pub reacao: Option<&'static str>,
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
    pub turno_aberto: bool,
    pub acomodando: bool,
    /// Hora do último evento (ms desde 1970).
    pub ultimo_evento_ms: u64,
    pub contadores: Contadores,
}

/// O que o cérebro publica no `/v1/estado`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StopPendente {
    /// Hora do Stop (parede).
    ts_ms: u64,
    /// Fim da acomodação (monotônico).
    prazo_mono: u64,
    sha: bool,
    bg: Option<u64>,
}

/// A festa de um turno que fechou por um Stop: um turno que reabre a leva
/// junto, para só reagir de novo se o nível subir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Festa {
    nivel: Nivel,
    reacao: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Turno {
    id: Option<String>,
    t0_ms: u64,
    implicito: bool,
    src: Option<String>,
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
    stop: Option<StopPendente>,
    /// Já fechou por um Stop e reabriu: a festa de então.
    festa: Option<Festa>,
    continuacoes: u32,
}

impl Turno {
    fn novo(id: Option<String>, t0_ms: u64, implicito: bool, src: Option<String>) -> Turno {
        Turno {
            id,
            t0_ms,
            implicito,
            src,
            trabalho: 0,
            outras: 0,
            subagentes: 0,
            falhas: 0,
            de_agentes: 0,
            arquivos: BTreeSet::new(),
            arquivos_a_mais: 0,
            dur_ms: 0,
            stop: None,
            festa: None,
            continuacoes: 0,
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
        if !falhou
            && let Some(arq) = &ev.arq
            && !self.arquivos.contains(arq)
        {
            if self.arquivos.len() < ARQUIVOS_POR_TURNO {
                self.arquivos.insert(arq.clone());
            } else {
                self.arquivos_a_mais = self.arquivos_a_mais.saturating_add(1);
            }
        }
        self.dur_ms = self.dur_ms.saturating_add(ev.dur.unwrap_or(0));
    }
}

/// A reação de um nível, segundo o modo de celebração.
fn reacao_do_nivel(nivel: Nivel, modo: ModoCelebracao) -> Option<&'static str> {
    match (nivel, modo) {
        (_, ModoCelebracao::Desligada) => None,
        (_, ModoCelebracao::Discreta) | (Nivel::T0, _) => Some(ACENO),
        (Nivel::T1, _) => Some(PULINHO),
    }
}

/// Um turno que acabou: o registro e, se comemorou, a reação.
struct Fechamento {
    registro: RegistroTurno,
    reacao: Option<Reacao>,
    /// O turno tinha reaberto: o registro troca o que ele já tinha.
    substitui: bool,
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
    turno: Option<Turno>,
    trocado: Option<Trocado>,
    comemorado: Option<Comemorado>,
    /// Turnos já fechados (o `prompt_id`), mais novo no fim.
    fechados: VecDeque<String>,
    /// Subagente → turno em que nasceu.
    agentes: VecDeque<(String, Option<String>)>,
    ultimo_mono: u64,
    /// Hora do último evento (parede), para o `/v1/estado`.
    ultimo_parede: u64,
    contadores: Contadores,
}

impl Sessao {
    fn nova(sid: String, teste: bool, agora: Agora) -> Sessao {
        Sessao {
            sid,
            teste,
            proj: None,
            ent: None,
            estado: EstadoSessao::Parada,
            turno: None,
            trocado: None,
            comemorado: None,
            fechados: VecDeque::new(),
            agentes: VecDeque::new(),
            ultimo_mono: agora.mono_ms,
            ultimo_parede: agora.parede_ms,
            contadores: Contadores::default(),
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
        modo: ModoCelebracao,
        fechamentos: &mut Vec<Fechamento>,
    ) -> &mut Turno {
        self.encerrar_aberto(t, agora, modo, fechamentos);
        self.comemorado = None;
        self.turno
            .insert(Turno::novo(id.map(str::to_owned), t, true, None))
    }

    /// Um turno novo vai começar: o aberto acaba. Com Stop pendente comemora
    /// na hora (fim = hora do Stop); sem Stop vira o trocado e espera o Stop
    /// atrasado por [`ACOMODACAO_MS`] (um trocado de antes fecha sem festa).
    fn encerrar_aberto(
        &mut self,
        t: u64,
        agora: Agora,
        modo: ModoCelebracao,
        fechamentos: &mut Vec<Fechamento>,
    ) {
        let Some(aberto) = self.turno.take() else {
            return;
        };
        if let Some(stop) = aberto.stop {
            fechamentos.push(self.fechar_turno(aberto, Fim::Stop, stop.ts_ms, agora, modo, true));
            return;
        }
        if let Some(velho) = self.trocado.take() {
            fechamentos.push(self.fechar_trocado(velho, agora, modo));
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
        modo: ModoCelebracao,
    ) -> Fechamento {
        self.fechar_turno(
            trocado.turno,
            Fim::Substituido,
            trocado.troca_ms,
            agora,
            modo,
            false,
        )
    }

    /// Fecha o turno aberto.
    fn fechar(
        &mut self,
        fim: Fim,
        fim_ms: u64,
        agora: Agora,
        modo: ModoCelebracao,
    ) -> Option<Fechamento> {
        let turno = self.turno.take()?;
        Some(self.fechar_turno(turno, fim, fim_ms, agora, modo, true))
    }

    /// Fecha um turno. Só `Fim::Stop` comemora: na primeira vez, pelo nível
    /// e pelo modo; num turno que reabriu, só se o nível subiu, e o registro
    /// troca o de antes. Fechado por um Stop, o turno fica comemorado se
    /// `reabrivel` (o trocado não: já há um turno novo depois dele).
    fn fechar_turno(
        &mut self,
        mut turno: Turno,
        fim: Fim,
        fim_ms: u64,
        agora: Agora,
        modo: ModoCelebracao,
        reabrivel: bool,
    ) -> Fechamento {
        if let Some(id) = &turno.id {
            self.lembrar_fechado(id);
        }
        let nivel_agora = (fim == Fim::Stop).then(|| turno.nivel());
        let antes = turno.festa;
        let subiu = match (nivel_agora, antes) {
            (Some(_), None) => true,
            (Some(nivel), Some(festa)) => nivel > festa.nivel,
            (None, _) => false,
        };
        let nome = nivel_agora
            .filter(|_| subiu)
            .and_then(|nivel| reacao_do_nivel(nivel, modo));
        let reacao = nome.map(|nome| Reacao {
            nome,
            sid8: evento::curto(&self.sid),
            proj: self.proj.clone(),
            ts: agora.parede_ms,
            nivel: nivel_agora,
            teste: self.teste,
        });
        let nivel = nivel_agora.max(antes.map(|festa| festa.nivel));
        let reacao_do_turno = nome.or(antes.and_then(|festa| festa.reacao));
        if antes.is_none() {
            self.contadores.turnos += 1;
        }
        if reacao.is_some() {
            self.contadores.reacoes += 1;
        }
        let stop = turno.stop.take();
        let registro = RegistroTurno {
            sid8: evento::curto(&self.sid),
            turno8: turno.id.as_deref().map(evento::curto),
            proj: self.proj.clone(),
            teste: self.teste,
            implicito: turno.implicito,
            src: turno.src.clone(),
            t0_ms: turno.t0_ms,
            fim_ms,
            relogio_ms: fim_ms.saturating_sub(turno.t0_ms),
            trabalho: turno.trabalho,
            outras: turno.outras,
            arquivos: turno.arquivos(),
            subagentes: turno.subagentes,
            falhas: turno.falhas,
            de_agentes: turno.de_agentes,
            dur_ms: turno.dur_ms,
            bg: stop.and_then(|s| s.bg),
            sha: stop.is_some_and(|s| s.sha),
            continuacoes: turno.continuacoes,
            fim,
            nivel,
            reacao: reacao_do_turno,
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
            substitui: antes.is_some(),
        }
    }

    /// Turno de uma ferramenta de subagente: o turno em que ele nasceu, se
    /// ainda estiver aberto ou trocado; senão o do próprio evento; senão o
    /// aberto.
    fn turno_do_agente(&mut self, aid: Option<&str>, id: Option<&str>) -> Option<&mut Turno> {
        let nascimento = aid.and_then(|aid| {
            self.agentes
                .iter()
                .find(|(a, _)| a == aid)
                .map(|(_, t)| t.clone())
        });
        let alvo = match nascimento {
            Some(t) => t,
            None => id.map(str::to_owned),
        };
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

    fn lembrar_agente(&mut self, aid: &str, turno: Option<String>) {
        if self.agentes.iter().any(|(a, _)| a == aid) {
            return;
        }
        if self.agentes.len() == AGENTES_POR_SESSAO {
            self.agentes.pop_front();
        }
        self.agentes.push_back((aid.to_owned(), turno));
    }
}

/// O cérebro: sessões, turnos e a última reação.
#[derive(Debug, Clone)]
pub struct Cerebro {
    config: ConfigCerebro,
    /// (teste, sid) → sessão: eventos de teste nunca tocam sessões reais.
    sessoes: BTreeMap<(bool, String), Sessao>,
    turnos: VecDeque<RegistroTurno>,
    ultima_reacao: Option<Reacao>,
    ignorados: BTreeMap<String, u64>,
}

/// Hora do evento: o `ts` do hook quando plausível (até 6 h da chegada);
/// senão a hora de chegada.
pub fn hora_do_evento(ts: Option<u64>, recebido_ms: u64) -> u64 {
    match ts {
        Some(ts) if ts.abs_diff(recebido_ms) <= JANELA_TS_MS => ts,
        _ => recebido_ms,
    }
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
            ultima_reacao: None,
            ignorados: BTreeMap::new(),
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
    fn registrar(&mut self, fechamentos: Vec<Fechamento>, reacoes: &mut Vec<Reacao>) {
        for f in fechamentos {
            if f.substitui {
                self.turnos.retain(|r| r.chave != f.registro.chave);
            }
            if self.turnos.len() == TURNOS_GUARDADOS {
                self.turnos.pop_front();
            }
            self.turnos.push_back(f.registro);
            if let Some(reacao) = f.reacao {
                self.reagir(reacao, reacoes);
            }
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
        let modo = self.config.modo;
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

        // Um evento de trabalho da thread principal: o SubagentStart vem com
        // o `agent_id` do subagente que nasce, mas quem o lança é a thread
        // principal. Depois do Stop, ele mostra que o turno continua.
        let continua = (!ev.agente || ev.e == "SubagentStart") && continua_o_turno(&ev.e);
        if continua
            && let Some(turno) = sessao.turno.as_mut()
            && let Some(stop) = turno.stop
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
                        let turno = sessao.abrir(id, t, agora, modo, &mut fechamentos);
                        turno.implicito = false;
                        turno.src.clone_from(&ev.src);
                        sessao.estado = EstadoSessao::Pensando;
                    }
                    // Aberto por um evento que chegou antes do prompt (hooks
                    // async fora de ordem): é o turno dele.
                    Alvo::Aberto => match sessao.turno.as_mut() {
                        Some(turno) if turno.implicito => {
                            turno.implicito = false;
                            turno.t0_ms = turno.t0_ms.min(t);
                            turno.src.clone_from(&ev.src);
                            let andou =
                                turno.trabalho > 0 || turno.outras > 0 || turno.subagentes > 0;
                            if !andou && turno.stop.is_none() {
                                sessao.estado = EstadoSessao::Pensando;
                            }
                        }
                        _ => ignorado = Some("prompt_repetido"),
                    },
                    Alvo::Trocado => ignorado = Some("prompt_repetido"),
                    Alvo::Fechado => ignorado = Some("turno_fechado"),
                }
            }
            "PostToolUse" | "PostToolUseFailure" => {
                sessao.contadores.ferramentas += 1;
                let falhou = ev.e == "PostToolUseFailure";
                if ev.agente {
                    match sessao.turno_do_agente(ev.aid.as_deref(), id) {
                        Some(turno) => turno.contar_ferramenta(ev, falhou),
                        None => ignorado = Some("ferramenta_de_agente_fora_do_turno"),
                    }
                } else {
                    let interrompeu = falhou && ev.intr;
                    match sessao.alvo(id, t, continua) {
                        Alvo::Trocado => {
                            if let Some(trocado) = sessao.trocado.as_mut() {
                                trocado.turno.contar_ferramenta(ev, falhou);
                            }
                            if interrompeu && let Some(trocado) = sessao.trocado.take() {
                                let turno = trocado.turno;
                                fechamentos.push(sessao.fechar_turno(
                                    turno,
                                    Fim::Interrompido,
                                    t,
                                    agora,
                                    modo,
                                    false,
                                ));
                            }
                        }
                        Alvo::Fechado => ignorado = Some("turno_fechado"),
                        alvo => {
                            if matches!(alvo, Alvo::Novo) {
                                sessao.abrir(id, t, agora, modo, &mut fechamentos);
                            }
                            if let Some(turno) = sessao.turno.as_mut() {
                                turno.contar_ferramenta(ev, falhou);
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
                                    modo,
                                ));
                                sessao.estado = EstadoSessao::Parada;
                            }
                        }
                    }
                }
            }
            "SubagentStart" => match sessao.alvo(id, t, continua) {
                Alvo::Trocado => {
                    let nascimento = sessao.trocado.as_mut().map(|trocado| {
                        trocado.turno.subagentes = trocado.turno.subagentes.saturating_add(1);
                        trocado.turno.id.clone()
                    });
                    if let (Some(nascimento), Some(aid)) = (nascimento, &ev.aid) {
                        sessao.lembrar_agente(aid, nascimento);
                    }
                }
                Alvo::Fechado => ignorado = Some("turno_fechado"),
                alvo => {
                    if matches!(alvo, Alvo::Novo) {
                        sessao.abrir(id, t, agora, modo, &mut fechamentos);
                    }
                    let nascimento = sessao.turno.as_mut().map(|turno| {
                        turno.subagentes = turno.subagentes.saturating_add(1);
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
                                sessao.abrir(id, t, agora, modo, &mut fechamentos);
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
                        fechamentos.extend(sessao.fechar(Fim::Ocioso, t, agora, modo));
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
                // novo já começou).
                Alvo::Trocado => {
                    if let Some(trocado) = sessao.trocado.take() {
                        let mut turno = trocado.turno;
                        turno.stop = Some(StopPendente {
                            ts_ms: t,
                            prazo_mono: agora.mono_ms,
                            sha: ev.sha,
                            bg: ev.bg,
                        });
                        fechamentos.push(sessao.fechar_turno(
                            turno,
                            Fim::Stop,
                            t,
                            agora,
                            modo,
                            false,
                        ));
                    }
                }
                Alvo::Fechado => ignorado = Some("stop_repetido"),
                alvo => {
                    if matches!(alvo, Alvo::Novo) {
                        sessao.abrir(id, t, agora, modo, &mut fechamentos);
                    }
                    if let Some(turno) = sessao.turno.as_mut() {
                        if turno.stop.is_some() {
                            ignorado = Some("stop_repetido");
                        } else {
                            turno.stop = Some(StopPendente {
                                ts_ms: t,
                                prazo_mono: agora.mono_ms + ACOMODACAO_MS,
                                sha: ev.sha,
                                bg: ev.bg,
                            });
                            sessao.estado = EstadoSessao::Parada;
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
                                modo,
                                false,
                            ));
                        }
                    }
                    Alvo::Aberto => fechamentos.extend(sessao.fechar(Fim::Falhou, t, agora, modo)),
                    Alvo::Fechado | Alvo::Novo => {}
                }
                sessao.estado = EstadoSessao::Erro;
            }
            _ => ignorado = Some("evento_desconhecido"),
        }
        if let Some(motivo) = ignorado {
            self.ignorar(motivo);
        }
        self.registrar(fechamentos, &mut reacoes);
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
        let modo = self.config.modo;
        let mut fechamentos = Vec::new();
        if let Some(trocado) = sessao.trocado.take() {
            fechamentos.push(sessao.fechar_trocado(trocado, agora, modo));
        }
        fechamentos.extend(sessao.fechar(Fim::SessaoEncerrada, t, agora, modo));
        self.registrar(fechamentos, reacoes);
        let sobrou = self.sessoes.keys().any(|(teste, _)| *teste == chave.0);
        if sai && !sobrou && self.config.modo != ModoCelebracao::Desligada {
            let reacao = Reacao {
                nome: TCHAU,
                sid8: evento::curto(&sessao.sid),
                proj: sessao.proj.clone(),
                ts: agora.parede_ms,
                nivel: None,
                teste: chave.0,
            };
            self.reagir(reacao, reacoes);
        }
    }

    /// Prazos vencidos: trocados cujo Stop não veio (fecham sem festa),
    /// acomodações que terminaram (comemoram) e sessões que expiraram (somem
    /// caladas).
    pub fn tique(&mut self, agora: Agora) -> Vec<Reacao> {
        let mut reacoes = Vec::new();
        let modo = self.config.modo;
        let mut fechamentos = Vec::new();
        for sessao in self.sessoes.values_mut() {
            if sessao
                .trocado
                .as_ref()
                .is_some_and(|x| agora.mono_ms >= x.prazo_mono)
                && let Some(trocado) = sessao.trocado.take()
            {
                fechamentos.push(sessao.fechar_trocado(trocado, agora, modo));
            }
            let vencida = sessao
                .turno
                .as_ref()
                .and_then(|t| t.stop)
                .is_some_and(|s| agora.mono_ms >= s.prazo_mono);
            if vencida {
                let fim_ms = sessao
                    .turno
                    .as_ref()
                    .and_then(|t| t.stop)
                    .map_or(agora.parede_ms, |s| s.ts_ms);
                fechamentos.extend(sessao.fechar(Fim::Stop, fim_ms, agora, modo));
            }
        }
        self.registrar(fechamentos, &mut reacoes);
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
                let acomodacao = s.turno.as_ref().and_then(|t| t.stop).map(|p| p.prazo_mono);
                let trocado = s.trocado.as_ref().map(|x| x.prazo_mono);
                [Some(s.ultimo_mono + vida), acomodacao, trocado]
            })
            .flatten()
            .min()
    }

    pub fn resumo(&self) -> Resumo {
        Resumo {
            sessoes: self
                .sessoes
                .values()
                .map(|s| ResumoSessao {
                    sid8: evento::curto(&s.sid),
                    proj: s.proj.clone(),
                    ent: s.ent.clone(),
                    teste: s.teste,
                    estado: s.estado,
                    turno_aberto: s.turno.is_some(),
                    acomodando: s.turno.as_ref().is_some_and(|t| t.stop.is_some()),
                    ultimo_evento_ms: s.ultimo_parede,
                    contadores: s.contadores.clone(),
                })
                .collect(),
            ultima_reacao: self.ultima_reacao.clone(),
            turnos: self.turnos.iter().rev().cloned().collect(),
            ignorados: self.ignorados.clone(),
            origens: self.config.origens.clone(),
        }
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
            .arquivos
            .len();
        assert_eq!(lembrados, ARQUIVOS_POR_TURNO, "memória limitada");
        c.receber(&stop("p1"), BASE + 2, em(2));
        assert_eq!(c.tique(em(2 + ACOMODACAO_MS))[0].nome, PULINHO);
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
        assert_eq!(
            rodar(&mut com_modo(ModoCelebracao::Discreta), roteiro()),
            vec![(2_800, ACENO), (4_000, TCHAU)]
        );
        assert_eq!(
            rodar(&mut com_modo(ModoCelebracao::Desligada), roteiro()),
            vec![]
        );
        assert_eq!(
            rodar(&mut com_modo(ModoCelebracao::SempreGrande), roteiro()),
            vec![(2_800, PULINHO), (4_000, TCHAU)],
            "até o M5 não há nível maior"
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
        assert_eq!(c.proximo_prazo(), Some(VIDA_SESSAO_MS));
        c.receber(&stop("p1"), BASE + 1_000, em(1_000));
        assert_eq!(c.proximo_prazo(), Some(1_000 + ACOMODACAO_MS));
        assert!(c.tique(em(1_799)).is_empty());
        assert_eq!(c.tique(em(1_800))[0].nome, ACENO);
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
}
