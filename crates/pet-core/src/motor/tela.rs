//! A tela do M5 (decisão 0076): o que o pet mostra agora, decidido no Motor a
//! cada mudança do cérebro, do desktop e do relógio, e anotado nas intenções
//! (decisão 0077):
//!
//! - **a festa** de cada fim, com a mesclagem: um fim de outra sessão (do
//!   mesmo mundo: teste com teste, real com real) até [`MESCLA_MS`] depois do
//!   começo da festa de agora entra nela, com o nível maior e o balão "2
//!   prontos: api, web"; a reação só toca de novo se o nível subir, e a
//!   primeira festa sai na hora da acomodação dela, sem esperar;
//! - **a base**: o estado da skin da sessão mais alta na [`Prioridade`];
//! - **os selos** das outras sessões ([`Selos`]);
//! - **o pronto** que segura a base por [`PRONTO_NA_BASE_MS`] e depois vira
//!   bandeirinha;
//! - **o sono** do pet parado e sem nada pendente ([`Sono`]);
//! - **a discrição** do compartilhamento de tela: depois de
//!   [`DISCRICAO_APOS_MS`] compartilhando (somados: no Hyprland 0.56.2 o sinal
//!   pisca numa tela parada), nenhum balão leva nome de projeto, até
//!   [`SEGURA_MS`] depois do último sinal (decisão 0081).
//!
//! A reação e o balão vão para a tela daqui (decisão 0079); a base, os
//! selos, o confete, os voos e a faixa esperam quem desenha (a segunda metade
//! do M5), pelas intenções e pela fotografia de agora ([`PainelTela`], o
//! `/v1/estado.fotografia`; o `tela` de lá já é o da aprovação).

use std::collections::BTreeMap;

use serde::Serialize;

use super::{BOCEJO, DESPERTAR, Motor, balao, intencoes, janelas};
use crate::animador::{Base, Ritmo};
use crate::cerebro::{self, EstadoSessao, Nivel, Reacao, ResumoSessao, TipoAviso, TipoEspera};
use crate::config::ModoCelebracao;
use crate::fonte;

/// Um fim até isto depois do começo da festa de agora entra nela.
pub const MESCLA_MS: u64 = 3_000;
/// O pronto segura a base por isto depois da festa; depois, só a bandeirinha.
pub const PRONTO_NA_BASE_MS: u64 = 2 * 60 * 1000;
/// Parado e sem nada pendente por isto: o bocejo.
pub const BOCEJO_MS: u64 = 3 * 60 * 1000;
/// … por isto: dorme.
pub const SONO_MS: u64 = 8 * 60 * 1000;
/// … por isto, com o Renan longe do teclado e do mouse: dorme.
pub const SONO_LONGE_MS: u64 = 3 * 60 * 1000;
/// … por isto: o sono profundo (sem commit nenhum).
pub const SONO_PROFUNDO_MS: u64 = 30 * 60 * 1000;
/// Compartilhando a tela por isto (o sinal somado num episódio), os balões
/// perdem os nomes. Uma captura de tela acende o sinal por menos de meio
/// segundo (decisão 0081).
pub const DISCRICAO_APOS_MS: u64 = 2_000;
/// Antes de a discrição ligar, os sinais até isto um do outro somam no mesmo
/// episódio: no Hyprland 0.56.2 o sinal segue os quadros copiados, e numa
/// tela parada ele pisca a cada desenho (o do próprio pet também).
pub const JUNTA_MS: u64 = 60_000;
/// Ligada, a discrição dura até isto depois do último sinal: cobre as pausas
/// de uma tela parada (os ritmos da base nunca passam de 30 s sem desenhar,
/// fora o sono profundo) e a fonte dos eventos que cai e volta.
pub const SEGURA_MS: u64 = 5 * 60 * 1000;
/// Confetes do T2 e do T3.
pub const CONFETES_T2: u32 = 12;
pub const CONFETES_T3: u32 = 40;
/// Cores da paleta dos projetos (a bandeirinha do pronto).
pub const CORES: u32 = 8;
/// Bandeirinhas no máximo.
pub const MAX_BANDEIRAS: usize = 8;
/// Caracteres de cada nome no balão de uma festa mesclada.
const NOME_NA_FESTA: usize = 10;

/// O que uma sessão pede da tela, da menos à mais forte (decisão 0076):
/// esperando você > erro > cansado > pronto > trabalhando > compactando >
/// pensando > parado > dormindo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Prioridade {
    Dormindo,
    Parado,
    Pensando,
    Compactando,
    Trabalhando,
    Pronto,
    Cansado,
    Erro,
    Esperando,
}

impl Prioridade {
    /// O estado da skin que a base segura.
    pub fn estado_da_skin(self) -> &'static str {
        match self {
            Prioridade::Esperando => "waiting",
            Prioridade::Erro => "error",
            Prioridade::Cansado | Prioridade::Dormindo => "sleep",
            Prioridade::Pronto => "ready",
            Prioridade::Trabalhando => "working",
            Prioridade::Compactando | Prioridade::Pensando => "thinking",
            Prioridade::Parado => "idle",
        }
    }

    /// A de uma sessão agora (`agora_ms` no relógio do laço). O pronto só
    /// conta nos [`PRONTO_NA_BASE_MS`] depois da festa.
    pub fn da_sessao(s: &ResumoSessao, agora_ms: u64) -> Prioridade {
        let aviso = s.aviso;
        if s.estado == EstadoSessao::Esperando
            || aviso.is_some_and(|a| a.tipo == TipoAviso::Esperando)
        {
            return Prioridade::Esperando;
        }
        match s.estado {
            EstadoSessao::Erro => return Prioridade::Erro,
            EstadoSessao::Cansado => return Prioridade::Cansado,
            _ => {}
        }
        if aviso.is_some_and(|a| {
            a.tipo == TipoAviso::Pronto && agora_ms < a.desde_mono + PRONTO_NA_BASE_MS
        }) {
            return Prioridade::Pronto;
        }
        match s.estado {
            EstadoSessao::Trabalhando => Prioridade::Trabalhando,
            EstadoSessao::Compactando => Prioridade::Compactando,
            EstadoSessao::Pensando => Prioridade::Pensando,
            _ => Prioridade::Parado,
        }
    }
}

/// O sono do pet parado e sem nada pendente (decisão 0076).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sono {
    #[default]
    Acordado,
    Bocejou,
    Dormindo,
    Profundo,
}

impl Sono {
    fn seguinte(self) -> Sono {
        match self {
            Sono::Acordado => Sono::Bocejou,
            Sono::Bocejou => Sono::Dormindo,
            Sono::Dormindo | Sono::Profundo => Sono::Profundo,
        }
    }

    /// Quanto tempo parado leva a este sono.
    fn depois_de(self, longe: bool) -> u64 {
        match self {
            Sono::Acordado => 0,
            Sono::Bocejou => BOCEJO_MS,
            Sono::Dormindo if longe => SONO_LONGE_MS,
            Sono::Dormindo => SONO_MS,
            Sono::Profundo => SONO_PROFUNDO_MS,
        }
    }
}

/// Os selos das outras sessões (decisão 0076).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Selos {
    /// As outras sessões com aviso de espera ou de erro, ou trabalhando
    /// (pensando, compactando, esperando, no erro ou cansadas): o "+N".
    pub mais: u32,
    /// Uma bandeirinha por sessão com o pronto (fora a que segura a base), da
    /// mais velha para a mais nova, na cor do projeto (0 a 7, [`cor`]).
    pub bandeiras: Vec<u8>,
    /// Alguma sessão tem uma corrente de agentes aberta: o "…".
    pub corrente: bool,
}

/// A cor de um projeto na paleta de [`CORES`]: o FNV-1a de 32 bits do nome,
/// estável entre as execuções e as máquinas.
pub fn cor(nome: &str) -> u8 {
    let mut h: u32 = 0x811c_9dc5;
    for b in nome.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    (h % CORES) as u8
}

/// O que cada nível da festa faz além da reação (decisão 0076).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Efeitos {
    confete: u32,
    voo: Option<&'static str>,
    faixa: bool,
}

/// O T2 com o voo curto e 12 confetes; o T3 com o voo atravessando a tela,
/// 40 confetes e a faixa "PRONTO!". `sem_voo`: o "não perturbe" (nada de
/// voo pela tela).
fn efeitos(nivel: Nivel, sem_voo: bool) -> Efeitos {
    let (confete, voo, faixa) = match nivel {
        Nivel::T0 | Nivel::T1 => (0, None, false),
        Nivel::T2 => (CONFETES_T2, Some("curto"), false),
        Nivel::T3 => (CONFETES_T3, Some("atravessar"), true),
    };
    Efeitos {
        confete,
        voo: voo.filter(|_| !sem_voo),
        faixa,
    }
}

/// A festa de agora de um mundo: quem entrou nela e o nível.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FestaAtual {
    inicio_ms: u64,
    nivel: Nivel,
    /// As sessões na festa (sid8 e projeto), na ordem em que entraram.
    sessoes: Vec<(String, Option<String>)>,
}

/// O que o Motor anunciou da tela (as intenções dizem o mesmo).
#[derive(Debug, Clone)]
pub(super) struct Tela {
    /// A festa de agora de cada mundo (o real e o de teste).
    festas: [Option<FestaAtual>; 2],
    /// A base anunciada: a prioridade e se é o sono profundo.
    base: (Prioridade, bool),
    selos: Selos,
    /// Desde quando o pet está parado sem nada pendente (ou desde o último
    /// evento ou clique que o acordou).
    parado_desde: Option<u64>,
    sono: Sono,
    /// O sinal do compartilhamento de tela está ligado desde aqui.
    sinal_desde: Option<u64>,
    /// O tempo de sinal somado no episódio (os trechos que já fecharam).
    somado_ms: u64,
    /// O último instante com sinal, com ele desligado agora: o episódio
    /// acaba [`JUNTA_MS`] (ou, com a discrição, [`SEGURA_MS`]) depois.
    ultimo_sinal: Option<u64>,
    /// A discrição do compartilhamento de tela está ligada.
    pub(super) discreto: bool,
    /// A prioridade de cada sessão na última olhada: na acomodação do Stop
    /// (0,8 s), a sessão segura a de antes, e a base não pisca parada antes
    /// da festa.
    vistas: BTreeMap<janelas::Chave, Prioridade>,
}

impl Default for Tela {
    fn default() -> Tela {
        Tela {
            festas: [None, None],
            base: (Prioridade::Parado, false),
            selos: Selos::default(),
            // O pet nasce parado: o relógio do sono começa com o do laço.
            parado_desde: Some(0),
            sono: Sono::Acordado,
            sinal_desde: None,
            somado_ms: 0,
            ultimo_sinal: None,
            discreto: false,
            vistas: BTreeMap::new(),
        }
    }
}

/// A tela como ela deveria estar agora.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Decisao {
    /// Parado sem nada pendente (o sono anda).
    parado: bool,
    sono: Sono,
    base: (Prioridade, bool),
    sid8: Option<String>,
    selos: Selos,
    /// A próxima mudança pelo relógio (o pronto que vira selo, o sono).
    proxima: Option<u64>,
    /// A prioridade de cada sessão.
    vistas: BTreeMap<janelas::Chave, Prioridade>,
}

/// A fotografia da tela no `/v1/estado.fotografia` (decisão 0077).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PainelTela {
    /// O estado da skin que a base segura.
    pub base: &'static str,
    pub prioridade: Prioridade,
    /// A sessão que manda na base, se há uma.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid8: Option<String>,
    pub sono: Sono,
    pub selos: Selos,
    /// A escalada do aviso que o pet chama (decisão 0075).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalada: Option<PainelEscalada>,
    /// A festa dos últimos [`MESCLA_MS`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub festa: Option<PainelFesta>,
    /// Os balões sem nomes (o compartilhamento de tela).
    pub discricao: bool,
}

impl Default for PainelTela {
    fn default() -> PainelTela {
        PainelTela {
            base: Prioridade::Parado.estado_da_skin(),
            prioridade: Prioridade::Parado,
            sid8: None,
            sono: Sono::Acordado,
            selos: Selos::default(),
            escalada: None,
            festa: None,
            discricao: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PainelEscalada {
    pub sid8: String,
    pub nivel: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub espera: Option<TipoEspera>,
    pub pulso: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PainelFesta {
    pub nivel: Nivel,
    pub sessoes: u32,
    pub ha_ms: u64,
}

impl Motor {
    // --- a festa --------------------------------------------------------------

    /// As reações do cérebro viram festas (ou entram na de agora), o tchau e
    /// o pulinho discreto; o que volta é o que o animador toca.
    pub(super) fn festejar(&mut self, reacoes: Vec<Reacao>, agora_ms: u64) -> Vec<Reacao> {
        let mut saida = Vec::new();
        let modo = self.cerebro.config().modo;
        for r in reacoes {
            if r.nome == cerebro::TCHAU {
                self.anotar_reacao(&r, r.nome, "tchau", agora_ms);
                saida.push(r);
                continue;
            }
            // O fim de máquina e o do modo discreto: o pulinho sem balão; na
            // festa de agora, só no registro (decisão 0076).
            let discreta =
                r.discreta || (modo == ModoCelebracao::Discreta && r.nivel > Some(Nivel::T0));
            if discreta {
                if self.na_festa(r.teste, agora_ms) || !self.na_tela() {
                    continue;
                }
                let nome = if self.soneca(agora_ms).is_some() {
                    cerebro::ACENO
                } else {
                    r.nome
                };
                self.anotar_reacao(&r, nome, "fim_discreto", agora_ms);
                saida.push(Reacao { nome, ..r });
                continue;
            }
            saida.extend(self.festa(r, agora_ms));
        }
        saida
    }

    fn anotar_reacao(&mut self, r: &Reacao, nome: &str, motivo: &'static str, agora_ms: u64) {
        self.anotar(
            agora_ms,
            intencoes::Tipo::Reacao {
                nome: nome.to_owned(),
                motivo,
                sid8: Some(r.sid8.clone()),
                nivel: r.nivel,
            },
        );
    }

    /// Há uma festa do mundo `teste` que começou há menos de [`MESCLA_MS`].
    fn na_festa(&self, teste: bool, agora_ms: u64) -> bool {
        self.tela.festas[usize::from(teste)]
            .as_ref()
            .is_some_and(|f| agora_ms < f.inicio_ms + MESCLA_MS)
    }

    /// A festa de um fim: nova, ou dentro da de agora. Escondido, nada toca
    /// e nada fica para depois (decisão 0076).
    fn festa(&mut self, r: Reacao, agora_ms: u64) -> Option<Reacao> {
        let nivel = r.nivel.unwrap_or(Nivel::T0);
        if !self.na_tela() {
            self.anotar(
                agora_ms,
                intencoes::Tipo::Festa {
                    sid8: r.sid8.clone(),
                    nivel,
                    reacao: Some(r.nome),
                    confete: 0,
                    voo: None,
                    faixa: false,
                    escondida: true,
                },
            );
            return None;
        }
        // Na soneca, só as reações pequenas (decisão 0053): o aceno, sem
        // confete, voo nem faixa. Com o "não perturbe", sem voo.
        let soneca = self.soneca(agora_ms).is_some();
        let sem_voo = self.nao_perturbe;
        let mundo = usize::from(r.teste);
        if self.na_festa(r.teste, agora_ms) {
            let f = self.tela.festas[mundo].as_mut().expect("na festa");
            if !f.sessoes.iter().any(|(s, _)| *s == r.sid8) {
                f.sessoes.push((r.sid8.clone(), r.proj.clone()));
            }
            let subiu = nivel > f.nivel;
            f.nivel = f.nivel.max(nivel);
            let f = f.clone();
            let toca = subiu && !soneca;
            let e = if toca {
                efeitos(f.nivel, sem_voo)
            } else {
                Efeitos::default()
            };
            self.anotar(
                agora_ms,
                intencoes::Tipo::FestaMesclada {
                    sid8: r.sid8.clone(),
                    sessoes: f.sessoes.len() as u32,
                    nivel: f.nivel,
                    reacao: toca.then_some(r.nome),
                    confete: e.confete,
                    voo: e.voo,
                    faixa: e.faixa,
                },
            );
            if f.nivel > Nivel::T0 {
                let linhas = self.linhas_da_festa(&f.sessoes);
                self.balao_decidido(linhas, "festa", agora_ms);
            }
            return toca.then_some(r);
        }
        let sessoes = vec![(r.sid8.clone(), r.proj.clone())];
        self.tela.festas[mundo] = Some(FestaAtual {
            inicio_ms: agora_ms,
            nivel,
            sessoes: sessoes.clone(),
        });
        let (nome, e) = if soneca {
            (cerebro::ACENO, Efeitos::default())
        } else {
            (r.nome, efeitos(nivel, sem_voo))
        };
        self.anotar(
            agora_ms,
            intencoes::Tipo::Festa {
                sid8: r.sid8.clone(),
                nivel,
                reacao: Some(nome),
                confete: e.confete,
                voo: e.voo,
                faixa: e.faixa,
                escondida: false,
            },
        );
        if nivel > Nivel::T0 {
            let linhas = self.linhas_da_festa(&sessoes);
            self.balao_decidido(linhas, "festa", agora_ms);
        }
        Some(Reacao { nome, ..r })
    }

    /// "Prontinho! api", ou "2 prontos: api, web" (os nomes cortados e sem
    /// repetir); sem os nomes na discrição.
    fn linhas_da_festa(&self, sessoes: &[(String, Option<String>)]) -> Vec<String> {
        let nomes = !self.tela.discreto;
        if let [(_, proj)] = sessoes {
            return vec![balao::com_projeto(
                "Prontinho!",
                proj.as_deref().filter(|_| nomes),
            )];
        }
        let n = sessoes.len();
        if !nomes {
            return vec![format!("{n} prontos")];
        }
        let mut lista: Vec<String> = Vec::new();
        for (_, proj) in sessoes {
            let nome = fonte::cortar(proj.as_deref().unwrap_or("sem pasta"), NOME_NA_FESTA);
            if !lista.contains(&nome) {
                lista.push(nome);
            }
        }
        vec![format!("{n} prontos: {}", lista.join(", "))]
    }

    // --- a base, os selos e o sono ------------------------------------------

    /// A tela como ela deveria estar em `agora_ms`, do que o cérebro sabe.
    fn decidir_tela(&self, agora_ms: u64) -> Decisao {
        let sessoes = self.cerebro.resumo_das_sessoes();
        let chamando = self.chamando.as_ref().map(|c| &c.chave);
        // A sessão que manda: a mais alta na prioridade; no empate, a real
        // antes da de teste, a que o pet chama (o aviso mais velho) e a de
        // evento mais novo.
        let vistas: BTreeMap<janelas::Chave, Prioridade> = sessoes
            .iter()
            .map(|s| {
                let mut p = Prioridade::da_sessao(s, agora_ms);
                if s.acomodando && p == Prioridade::Parado {
                    p = self.tela.vistas.get(&s.chave).copied().unwrap_or(p);
                }
                (s.chave.clone(), p)
            })
            .collect();
        let lider = sessoes
            .iter()
            .map(|s| (vistas[&s.chave], s))
            .max_by_key(|(p, s)| (*p, !s.teste, chamando == Some(&s.chave), s.ultimo_evento_ms));
        let prioridade = lider.map_or(Prioridade::Parado, |(p, _)| p);
        let pendente = sessoes.iter().any(|s| {
            s.aviso
                .is_some_and(|a| matches!(a.tipo, TipoAviso::Esperando | TipoAviso::Erro))
        });
        let parado = prioridade == Prioridade::Parado && !pendente;
        let longe = self.desktop.ocioso == Some(true);
        let mut proxima: Vec<u64> = Vec::new();
        let sono = if parado {
            let desde = self.tela.parado_desde.unwrap_or(agora_ms);
            let mut sono = self.tela.sono;
            while sono < Sono::Profundo && agora_ms >= desde + sono.seguinte().depois_de(longe) {
                sono = sono.seguinte();
            }
            if sono < Sono::Profundo {
                proxima.push(desde + sono.seguinte().depois_de(longe));
            }
            sono
        } else {
            Sono::Acordado
        };
        let (base, sid8) = if sono >= Sono::Dormindo {
            ((Prioridade::Dormindo, sono == Sono::Profundo), None)
        } else {
            (
                (prioridade, false),
                lider
                    .filter(|(p, _)| *p > Prioridade::Parado)
                    .map(|(_, s)| s.sid8.clone()),
            )
        };
        let lider_chave = lider.map(|(_, s)| &s.chave);
        let mut prontos: Vec<&ResumoSessao> = Vec::new();
        let mut mais = 0;
        for s in &sessoes {
            if let Some(a) = s.aviso.filter(|a| a.tipo == TipoAviso::Pronto) {
                let fim = a.desde_mono + PRONTO_NA_BASE_MS;
                if fim > agora_ms {
                    proxima.push(fim);
                }
            }
            if Some(&s.chave) == lider_chave {
                if prioridade != Prioridade::Pronto
                    && s.aviso.is_some_and(|a| a.tipo == TipoAviso::Pronto)
                {
                    prontos.push(s);
                }
                continue;
            }
            match s.aviso.map(|a| a.tipo) {
                Some(TipoAviso::Pronto) => prontos.push(s),
                Some(_) => mais += 1,
                None if matches!(
                    s.estado,
                    EstadoSessao::Trabalhando
                        | EstadoSessao::Compactando
                        | EstadoSessao::Pensando
                        | EstadoSessao::Esperando
                        | EstadoSessao::Erro
                        | EstadoSessao::Cansado
                ) =>
                {
                    mais += 1
                }
                None => {}
            }
        }
        prontos.sort_by_key(|s| s.aviso.map(|a| a.desde_ms));
        let selos = Selos {
            mais,
            bandeiras: prontos
                .iter()
                .take(MAX_BANDEIRAS)
                .map(|s| cor(s.proj.as_deref().unwrap_or(&s.sid8)))
                .collect(),
            corrente: sessoes.iter().any(|s| s.corrente.is_some_and(|c| c.aberta)),
        };
        Decisao {
            parado,
            sono,
            base,
            sid8,
            selos,
            proxima: proxima.into_iter().filter(|t| *t > agora_ms).min(),
            vistas,
        }
    }

    /// Leva a tela ao que ela deveria ser agora: o sono anda (o bocejo), o
    /// pet acorda (o despertar) e a base e os selos que mudaram viram
    /// intenções. Devolve as reações para o animador.
    pub(super) fn observar_tela(&mut self, agora_ms: u64) -> Vec<Reacao> {
        let d = self.decidir_tela(agora_ms);
        self.tela.vistas = d.vistas.clone();
        let mut reacoes = Vec::new();
        if d.parado {
            self.tela.parado_desde.get_or_insert(agora_ms);
            while self.tela.sono < d.sono {
                self.tela.sono = self.tela.sono.seguinte();
                if self.tela.sono == Sono::Bocejou {
                    reacoes.extend(self.reacao_da_tela(BOCEJO, "sono", agora_ms));
                }
            }
        } else {
            self.tela.parado_desde = None;
            reacoes.extend(self.acordar(agora_ms, true));
        }
        if self.tela.base != d.base {
            self.tela.base = d.base;
            let (prioridade, profundo) = d.base;
            self.anotar(
                agora_ms,
                intencoes::Tipo::Base {
                    estado: prioridade.estado_da_skin(),
                    prioridade,
                    sid8: d.sid8,
                    profundo,
                },
            );
        }
        if self.tela.selos != d.selos {
            self.tela.selos = d.selos.clone();
            self.anotar(agora_ms, intencoes::Tipo::Selos(d.selos));
        }
        self.sincronizar_base(agora_ms);
        reacoes
    }

    /// A base que o pet deve segurar agora (decisão 0082): o estado da skin
    /// da base anunciada, no ritmo dele.
    pub(super) fn base_desejada(&self) -> Base {
        let (prioridade, profundo) = self.tela.base;
        let ritmo = match prioridade {
            Prioridade::Dormindo if profundo => Ritmo::Parado,
            Prioridade::Dormindo | Prioridade::Cansado => Ritmo::Laco,
            Prioridade::Trabalhando | Prioridade::Compactando | Prioridade::Pensando => {
                Ritmo::Quieto
            }
            // No teto da escalada, só a pose: o selo do aviso pulsa e a
            // rajada vem a cada minuto (decisão 0075).
            Prioridade::Esperando if self.nivel_da_escalada() >= 4 => Ritmo::Parado,
            _ => Ritmo::Repouso,
        };
        Base {
            estado: prioridade.estado_da_skin().to_owned(),
            ritmo,
        }
    }

    /// Leva a base de agora ao animador; se ela mudou, a pose nova entra no
    /// próximo quadro, já (ou no fim da reação que estiver tocando).
    pub(super) fn sincronizar_base(&mut self, agora_ms: u64) {
        let base = self.base_desejada();
        if let Some(pet) = self.pet.as_mut()
            && pet.definir_base(base, agora_ms)
        {
            self.proximo_quadro = Some(self.proximo_quadro.map_or(agora_ms, |p| p.min(agora_ms)));
        }
    }

    /// Um evento do Claude ou um clique: o pet acorda (o despertar, se
    /// dormia e `anunciar`) e o relógio do sono recomeça.
    pub(super) fn acordar(&mut self, agora_ms: u64, anunciar: bool) -> Vec<Reacao> {
        let dormia = self.tela.sono >= Sono::Dormindo;
        self.tela.sono = Sono::Acordado;
        if self.tela.parado_desde.is_some() {
            self.tela.parado_desde = Some(agora_ms);
        }
        if dormia && anunciar {
            self.reacao_da_tela(DESPERTAR, "acordou", agora_ms)
                .into_iter()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Uma reação do pet, não de uma sessão (o bocejo, o despertar): na
    /// intenção e para o animador, se o pet pode aparecer.
    fn reacao_da_tela(
        &mut self,
        nome: &'static str,
        motivo: &'static str,
        agora_ms: u64,
    ) -> Option<Reacao> {
        if !self.na_tela() {
            return None;
        }
        self.anotar(
            agora_ms,
            intencoes::Tipo::Reacao {
                nome: nome.to_owned(),
                motivo,
                sid8: None,
                nivel: None,
            },
        );
        Some(Reacao {
            nome,
            sid8: String::new(),
            proj: None,
            ts: self.parede(agora_ms),
            nivel: None,
            teste: false,
            discreta: false,
        })
    }

    // --- a discrição do compartilhamento de tela ------------------------------

    /// O sinal do compartilhamento de tela ligou ou desligou (o desktop
    /// contou, ou a fonte dos eventos caiu: desligado). A discrição liga com
    /// [`DISCRICAO_APOS_MS`] de sinal somados no episódio e só desliga
    /// [`SEGURA_MS`] depois do último sinal ([`Self::vencer_discricao`];
    /// decisão 0081).
    pub(super) fn compartilhamento(&mut self, compartilhando: bool, agora_ms: u64) {
        // Um episódio que acabou antes deste sinal fecha primeiro.
        self.vencer_discricao(agora_ms);
        if compartilhando {
            self.tela.sinal_desde.get_or_insert(agora_ms);
        } else if let Some(desde) = self.tela.sinal_desde.take() {
            self.tela.somado_ms = self
                .tela
                .somado_ms
                .saturating_add(agora_ms.saturating_sub(desde));
            self.tela.ultimo_sinal = Some(agora_ms);
        }
        // Um trecho que fechou já passando dos 2 s liga agora.
        self.vencer_discricao(agora_ms);
    }

    /// O sinal somado no episódio até `agora_ms`.
    fn sinal_somado(&self, agora_ms: u64) -> u64 {
        let trecho = self
            .tela
            .sinal_desde
            .map_or(0, |d| agora_ms.saturating_sub(d));
        self.tela.somado_ms.saturating_add(trecho)
    }

    /// Quando o episódio do compartilhamento acaba (com o sinal desligado):
    /// [`JUNTA_MS`] depois do último sinal, ou [`SEGURA_MS`] com a discrição.
    fn fim_do_episodio(&self) -> Option<u64> {
        if self.tela.sinal_desde.is_some() {
            return None;
        }
        let segura = if self.tela.discreto {
            SEGURA_MS
        } else {
            JUNTA_MS
        };
        self.tela.ultimo_sinal.map(|u| u + segura)
    }

    /// A discrição liga (um balão na tela sai: pode ter nome) ou o episódio
    /// acaba (os nomes voltam).
    fn vencer_discricao(&mut self, agora_ms: u64) {
        if !self.tela.discreto && self.sinal_somado(agora_ms) >= DISCRICAO_APOS_MS {
            self.tela.discreto = true;
            let tirou_balao = self.balao(agora_ms).is_some();
            if tirou_balao {
                self.balao = None;
                if self.pet.is_some() {
                    self.proximo_quadro =
                        Some(self.proximo_quadro.map_or(agora_ms, |p| p.min(agora_ms)));
                }
            }
            self.anotar(
                agora_ms,
                intencoes::Tipo::Discricao {
                    ligada: true,
                    tirou_balao,
                },
            );
        }
        if self.fim_do_episodio().is_some_and(|fim| agora_ms >= fim) {
            self.tela.ultimo_sinal = None;
            self.tela.somado_ms = 0;
            if self.tela.discreto {
                self.tela.discreto = false;
                self.anotar(
                    agora_ms,
                    intencoes::Tipo::Discricao {
                        ligada: false,
                        tirou_balao: false,
                    },
                );
            }
        }
    }

    /// O próximo prazo da discrição: a hora em que o sinal somado chega aos
    /// 2 s, ou o fim do episódio.
    pub(super) fn prazo_da_discricao(&self, agora_ms: u64) -> Option<u64> {
        match self.tela.sinal_desde {
            Some(desde) if !self.tela.discreto => {
                Some((desde + DISCRICAO_APOS_MS.saturating_sub(self.tela.somado_ms)).max(agora_ms))
            }
            Some(_) => None,
            None => self.fim_do_episodio().map(|fim| fim.max(agora_ms)),
        }
    }

    /// Os prazos da tela que venceram: a discrição que liga ou acaba, e o
    /// pronto que vira selo, o sono.
    pub(super) fn vencer_tela(&mut self, agora_ms: u64) -> Vec<Reacao> {
        self.vencer_discricao(agora_ms);
        self.observar_tela(agora_ms)
    }

    /// Quando a tela muda de novo pelo relógio: já, se o que ela deveria ser
    /// agora não é o anunciado; senão o pronto que vira selo, o próximo passo
    /// do sono e a discrição.
    pub(super) fn prazo_da_tela(&self) -> Option<u64> {
        let agora = self.relogio_ms;
        let d = self.decidir_tela(agora);
        if d.base != self.tela.base
            || d.selos != self.tela.selos
            || d.sono != self.tela.sono
            || d.parado != self.tela.parado_desde.is_some()
        {
            return Some(agora);
        }
        [d.proxima, self.prazo_da_discricao(agora)]
            .into_iter()
            .flatten()
            .min()
    }

    /// Os nomes dos projetos podem ir para os balões (sem a discrição).
    pub(super) fn nomes_visiveis(&self) -> bool {
        !self.tela.discreto
    }

    /// Como uma sessão aparece num balão: o projeto ou, na discrição, "sessão
    /// N" (a posição dela na lista do clique).
    pub(super) fn rotulo(&self, chave: &janelas::Chave, proj: Option<&str>) -> Option<String> {
        if self.nomes_visiveis() {
            return proj.map(str::to_owned);
        }
        let posicao = self
            .sessoes_da_lista()
            .iter()
            .position(|s| s.chave == *chave)
            .map_or(1, |i| i + 1);
        Some(format!("sessão {posicao}"))
    }

    /// A fotografia da tela para o `/v1/estado`.
    pub fn painel_da_tela(&self, agora_ms: u64) -> PainelTela {
        let (prioridade, _) = self.tela.base;
        let sid8 = (prioridade > Prioridade::Parado)
            .then(|| self.decidir_tela(agora_ms).sid8)
            .flatten();
        PainelTela {
            base: prioridade.estado_da_skin(),
            prioridade,
            sid8,
            sono: self.tela.sono,
            selos: self.tela.selos.clone(),
            escalada: self.chamando.as_ref().map(|c| PainelEscalada {
                sid8: c.sid8.clone(),
                nivel: c.escalada.nivel,
                espera: c.espera,
                pulso: c.escalada.pulso,
            }),
            festa: self
                .tela
                .festas
                .iter()
                .flatten()
                .filter(|f| agora_ms < f.inicio_ms + MESCLA_MS)
                .max_by_key(|f| f.inicio_ms)
                .map(|f| PainelFesta {
                    nivel: f.nivel,
                    sessoes: f.sessoes.len() as u32,
                    ha_ms: agora_ms.saturating_sub(f.inicio_ms),
                }),
            discricao: self.tela.discreto,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_cor_do_projeto_e_estavel_e_cabe_na_paleta() {
        assert_eq!(cor("api"), cor("api"));
        let cores: std::collections::BTreeSet<u8> =
            ["api", "web", "claude-pet", "agenda", "x", "y", "z", "w"]
                .iter()
                .map(|n| cor(n))
                .collect();
        assert!(cores.iter().all(|c| u32::from(*c) < CORES));
        assert!(cores.len() > 2, "espalha: {cores:?}");
        // O FNV-1a de 32 bits de referência.
        let mut h: u32 = 0x811c_9dc5;
        for b in b"a" {
            h ^= u32::from(*b);
            h = h.wrapping_mul(0x0100_0193);
        }
        assert_eq!(h, 0xe40c_292c);
        assert_eq!(u32::from(cor("a")), 0xe40c_292c % CORES);
    }

    #[test]
    fn a_ordem_da_prioridade_e_a_do_plano() {
        use Prioridade::*;
        let ordem = [
            Esperando,
            Erro,
            Cansado,
            Pronto,
            Trabalhando,
            Compactando,
            Pensando,
            Parado,
            Dormindo,
        ];
        assert!(ordem.windows(2).all(|j| j[0] > j[1]));
        assert_eq!(Cansado.estado_da_skin(), "sleep");
        assert_eq!(Compactando.estado_da_skin(), "thinking");
    }

    #[test]
    fn os_efeitos_de_cada_nivel() {
        assert_eq!(efeitos(Nivel::T1, false), Efeitos::default());
        assert_eq!(
            efeitos(Nivel::T2, false),
            Efeitos {
                confete: 12,
                voo: Some("curto"),
                faixa: false
            }
        );
        assert_eq!(
            efeitos(Nivel::T3, true),
            Efeitos {
                confete: 40,
                voo: None,
                faixa: true
            },
            "o não perturbe tira só o voo"
        );
    }

    #[test]
    fn o_sono_anda_pelo_tempo_parado() {
        assert_eq!(Sono::Bocejou.depois_de(false), 3 * 60_000);
        assert_eq!(Sono::Dormindo.depois_de(false), 8 * 60_000);
        assert_eq!(Sono::Dormindo.depois_de(true), 3 * 60_000);
        assert_eq!(Sono::Profundo.depois_de(true), 30 * 60_000);
        assert_eq!(Sono::Profundo.seguinte(), Sono::Profundo);
    }
}
