//! A memória das sessões (decisão 0093): o que o pet guarda em `/state` para
//! não esquecer as sessões abertas do Claude quando reinicia (uma atualização
//! refaz a produção, um crash, um `docker restart`).
//!
//! O cérebro guarda as sessões só na memória, e uma sessão parada não manda
//! nada (o `idle_prompt` sai uma vez por turno, ~60 s depois do Stop): sem
//! isto, o clique no Zeca dizia "nenhuma sessão do Claude aberta" com o Renan
//! cheio de sessões abertas, até cada uma ser usada de novo.
//!
//! **Só metadados**, e só das sessões reais (nunca as de teste): o `sid`
//! opaco, o nome da pasta do projeto, a origem, o estado e desde quando, a
//! hora do último evento, os agendamentos do último Stop, o aviso pendente
//! (o tipo, desde quando, o nível da escalada e quando o Renan viu o
//! diálogo), a janela do terminal (o endereço e a instância do compositor a
//! que ele pertence) e os ids de terminal. Nada de prompt, título de janela ou
//! caminho. Os tempos são de parede (ms desde 1970), para o `/v1/estado` e o
//! balão; o relógio do laço recomeça do zero com o processo, e quem restaura
//! conta os prazos de novo a partir deles ([`crate::cerebro::Agora::no_laco`]).
//!
//! **O tempo acordado** (decisão 0095): o relógio do laço não anda com a
//! máquina suspensa, e os prazos do pet que não reinicia contam só o tempo
//! acordado. Por isso cada instante também vai no relógio do laço de quem
//! gravou (`*_laco_ms`, com o `laco_ms` da gravação): quem restaura conta o
//! tempo acordado até a gravação mais a parada do pet ([`Volta`]). Um arquivo
//! sem eles (o da decisão 0093) conta pela parede.
//!
//! **O sossego** (decisão 0095): o "não perturbe" do último evento, a soneca
//! e a discrição do compartilhamento de tela, que seguram a escalada na L1 e
//! os nomes fora dos balões, também voltam ([`Sossego`]).
//!
//! **O arquivo** tem versão ([`VERSAO`]), o boot id da máquina (uma
//! reinicialização mata todo Claude: as sessões de antes seriam fantasmas) e
//! no máximo [`MAX_SESSOES`] sessões em até [`MAX_BYTES`]. Quem lê confere
//! campo a campo com as regras do fio v1 (`crate::evento`): uma sessão com um
//! campo ruim fica de fora; um sossego ruim vale como nenhum; um arquivo que
//! não é este JSON, de outra versão ou grande demais é ignorado inteiro. Os
//! erros nunca citam o conteúdo. A memória gravada há mais de
//! [`MEMORIA_VELHA_MS`] é velha ([`Volta::velha`]).
//!
//! Quem grava e lê o arquivo, e lê o boot id, é o daemon (o núcleo é puro).

use std::fmt;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::cerebro::{Agora, EstadoSessao, Instante, MAX_SESSOES, TipoAviso, TipoEspera, instante};
use crate::evento::{self, Terminal};
use crate::motor::janelas::Certeza;

/// A versão do arquivo que este pet escreve e lê.
pub const VERSAO: u64 = 1;
/// O arquivo, dentro da pasta do estado (`/state` no container).
pub const ARQUIVO: &str = "sessoes.json";
/// Maior arquivo que o pet lê: 64 sessões cabem em uns 40 KiB.
pub const MAX_BYTES: usize = 256 * 1024;
/// A memória gravada há mais que isto (pela parede) é velha (decisão 0095): o
/// `idle_prompt`, que sai uns 60 s depois do Stop e tira a espera respondida
/// com o pet fora (decisão 0094), pode ter se perdido também. A parada de uma
/// atualização ou de um crash fica bem abaixo (o pet grava na saída, e o
/// batimento regrava a memória que não mudou a cada 30 s).
pub const MEMORIA_VELHA_MS: u64 = 60_000;
/// O maior endereço de janela e a maior instância de compositor aceitos.
const MAX_ENDERECO: usize = 64;
const MAX_COMPOSITOR: usize = 128;
/// O boot id do Linux é um UUID (36 caracteres).
const MAX_BOOT: usize = 64;
/// O maior inteiro exato num número JSON (o mesmo do `ts` do fio v1).
const MAX_INTEIRO_JSON: u64 = (1 << 53) - 1;

fn eh_falso(b: &bool) -> bool {
    !*b
}

/// O arquivo da memória.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Memoria {
    pub versao: u64,
    /// Quando foi gravada (ms desde 1970).
    pub gravada_ms: u64,
    /// O relógio do laço de quem gravou, na gravação (decisão 0095): com ele,
    /// os instantes `*_laco_ms` dizem o tempo acordado de antes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub laco_ms: Option<u64>,
    /// O boot id da máquina na gravação (no Linux,
    /// `/proc/sys/kernel/random/boot_id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot: Option<String>,
    /// O que segurava o pet quieto na gravação (decisão 0095).
    #[serde(skip_serializing_if = "Sossego::vazio")]
    pub sossego: Sossego,
    pub sessoes: Vec<SessaoGuardada>,
}

/// O que segura a escalada na L1 e os nomes fora dos balões (decisão 0095):
/// sem isto, a espera que volta com a memória escalaria por cima do "não
/// perturbe", da soneca e da tela compartilhada. Os instantes são do relógio
/// do laço de quem gravou ([`Memoria::laco_ms`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Sossego {
    /// O "não perturbe" do Omarchy, pelo `dnd` do último evento do Claude:
    /// vale até o próximo evento, como no pet que não reinicia.
    #[serde(skip_serializing_if = "eh_falso")]
    pub nao_perturbe: bool,
    /// O fim da soneca do clique direito (decisão 0053).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soneca_ate_laco_ms: Option<i64>,
    /// A discrição do compartilhamento de tela estava ligada (decisão 0081).
    #[serde(skip_serializing_if = "eh_falso")]
    pub discricao: bool,
    /// O último sinal do compartilhamento, com a discrição ligada; sem ele, o
    /// sinal ainda estava aceso na gravação.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discricao_sinal_laco_ms: Option<i64>,
}

impl Sossego {
    /// Nada segurava o pet quieto.
    pub fn vazio(&self) -> bool {
        *self == Sossego::default()
    }
}

/// Uma sessão real guardada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessaoGuardada {
    /// O `session_id` inteiro (opaco): o mesmo `sid` dos próximos eventos.
    pub sid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proj: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ent: Option<String>,
    pub estado: EstadoSessao,
    pub estado_desde_ms: u64,
    /// O mesmo no relógio do laço de quem gravou (decisão 0095).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estado_desde_laco_ms: Option<i64>,
    pub ultimo_evento_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ultimo_evento_laco_ms: Option<i64>,
    /// Os agendamentos do último Stop (`crn`), se o hook disse: separam o
    /// tique de um laço do prompt digitado (decisão 0073).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agendamentos: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aviso: Option<AvisoGuardado>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub janela: Option<JanelaGuardada>,
}

/// O aviso pendente de uma sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AvisoGuardado {
    pub tipo: TipoAviso,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub espera: Option<TipoEspera>,
    pub desde_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desde_laco_ms: Option<i64>,
    /// O nível da escalada, no aviso de espera que o pet chamava.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nivel: Option<u8>,
    /// Quando o Renan viu o diálogo no terminal da sessão (decisão 0090): a
    /// hora de parede anotada quando ele viu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vista_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vista_laco_ms: Option<i64>,
}

/// A janela do terminal de uma sessão (decisão 0055).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JanelaGuardada {
    /// O endereço da janela (opaco: no Hyprland, o do `activewindowv2`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endereco: Option<String>,
    /// A instância do compositor a que o endereço pertence (no Hyprland, a
    /// assinatura): noutra instância, o mesmo endereço não é a mesma janela.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compositor: Option<String>,
    pub certeza: Certeza,
    /// Hora (parede) do último prompt casado.
    pub em_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<Terminal>,
}

/// Por que a memória não foi restaurada. As mensagens nunca citam o
/// conteúdo do arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recusa {
    /// Maior que [`MAX_BYTES`].
    Grande,
    /// Não é o JSON da memória.
    Json,
    /// De outra versão (um pet mais novo, ou mais velho, escreveu).
    Versao(u64),
    /// A máquina reiniciou desde a gravação: todo Claude de antes morreu.
    MaquinaReiniciou,
    /// Sem o boot id da gravação ou o de agora: não dá para saber se a
    /// máquina reiniciou.
    SemBoot,
    /// O arquivo existe, mas não deu para ler (a permissão, o disco; quem lê
    /// o arquivo diz o erro no log).
    Ilegivel,
}

impl Recusa {
    /// O motivo nas intenções (`restauracao`).
    pub fn motivo(self) -> &'static str {
        match self {
            Recusa::Grande => "grande",
            Recusa::Json => "arquivo_ruim",
            Recusa::Versao(_) => "versao",
            Recusa::MaquinaReiniciou => "maquina_reiniciou",
            Recusa::SemBoot => "sem_boot",
            Recusa::Ilegivel => "erro_de_leitura",
        }
    }
}

impl fmt::Display for Recusa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Recusa::Grande => write!(f, "o arquivo passa de {} KiB", MAX_BYTES / 1024),
            Recusa::Json => write!(f, "o arquivo não é o JSON da memória das sessões"),
            Recusa::Versao(v) => write!(f, "o arquivo é da versão {v}, e este pet lê a {VERSAO}"),
            Recusa::MaquinaReiniciou => write!(
                f,
                "a máquina reiniciou desde a gravação (outro boot id): as sessões de antes morreram"
            ),
            Recusa::SemBoot => write!(
                f,
                "sem o boot id da gravação ou o de agora, não dá para saber se a máquina reiniciou"
            ),
            Recusa::Ilegivel => write!(f, "não deu para ler o arquivo"),
        }
    }
}

/// O tempo de antes da partida no relógio do laço novo (decisões 0093 e
/// 0095), para quem restaura a memória em `agora`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volta {
    pub agora: Agora,
    pub gravada_ms: u64,
    laco_ms: Option<u64>,
}

impl Volta {
    pub fn de(memoria: &Memoria, agora: Agora) -> Volta {
        Volta {
            agora,
            gravada_ms: memoria.gravada_ms,
            laco_ms: memoria.laco_ms,
        }
    }

    /// O instante, no relógio do laço novo, de um instante de antes: pelo
    /// relógio do laço de quem gravou (`laco`), o tempo acordado até a
    /// gravação (uma suspensão da máquina não conta, como no pet que não
    /// reiniciou) mais a parada do pet pela parede; sem ele (um arquivo da
    /// decisão 0093), pela hora de parede `parede_ms`.
    pub fn instante(&self, laco: Option<i64>, parede_ms: u64) -> Instante {
        let Some((x, gravacao)) = laco.zip(self.laco_ms) else {
            return self.agora.no_laco(parede_ms);
        };
        let acordado = instante(gravacao).saturating_sub(x);
        let parada = instante(self.agora.parede_ms.saturating_sub(self.gravada_ms));
        instante(self.agora.mono_ms)
            .saturating_sub(acordado)
            .saturating_sub(parada)
    }

    /// O instante da gravação no relógio do laço novo.
    pub fn gravacao(&self) -> Instante {
        self.instante(
            self.laco_ms.map(|l| i64::try_from(l).unwrap_or(i64::MAX)),
            self.gravada_ms,
        )
    }

    /// Um instante de antes depois da gravação (o arquivo mexido): fora.
    pub fn depois_da_gravacao(&self, laco: Option<i64>) -> bool {
        laco.zip(self.laco_ms)
            .is_some_and(|(x, gravacao)| x > instante(gravacao))
    }

    /// A memória é velha (decisão 0095): gravada há mais de
    /// [`MEMORIA_VELHA_MS`] pela parede, ou adiante disso (o relógio voltou).
    pub fn velha(&self) -> bool {
        self.agora.parede_ms.abs_diff(self.gravada_ms) > MEMORIA_VELHA_MS
    }
}

/// Uma memória lida: as sessões que passaram na conferência e quantas
/// ficaram de fora por um campo ruim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lida {
    pub memoria: Memoria,
    pub descartadas: usize,
}

impl Memoria {
    /// Uma memória vazia, gravada em `gravada_ms`.
    pub fn nova(gravada_ms: u64, boot: Option<String>) -> Memoria {
        Memoria {
            versao: VERSAO,
            gravada_ms,
            laco_ms: None,
            boot,
            sossego: Sossego::default(),
            sessoes: Vec::new(),
        }
    }

    /// Nada a lembrar: nenhuma sessão e nada segurando o pet quieto.
    pub fn vazia(&self) -> bool {
        self.sessoes.is_empty() && self.sossego.vazio()
    }

    /// O mesmo conteúdo de `outra`, fora a hora da gravação (o que diz se há
    /// algo novo a gravar).
    pub fn mesmo_conteudo(&self, outra: &Memoria) -> bool {
        self.versao == outra.versao
            && self.boot == outra.boot
            && self.sossego == outra.sossego
            && self.sessoes == outra.sessoes
    }

    /// O texto do arquivo.
    pub fn texto(&self) -> String {
        let mut texto = serde_json::to_string_pretty(self).unwrap_or_default();
        texto.push('\n');
        texto
    }

    /// A memória é desta partida da máquina: o boot id da gravação é o de
    /// agora. Sem um dos dois, não se sabe, e nada volta (uma sessão
    /// fantasma por até uma semana, a vida de uma sessão sem evento, decisão
    /// 0096, é pior que esquecer).
    pub fn conferir_boot(&self, agora: Option<&str>) -> Result<(), Recusa> {
        match (self.boot.as_deref(), agora) {
            (Some(gravado), Some(agora)) if gravado == agora => Ok(()),
            (Some(_), Some(_)) => Err(Recusa::MaquinaReiniciou),
            _ => Err(Recusa::SemBoot),
        }
    }
}

/// Lê o texto do arquivo. A conferência é a do fio v1: uma sessão com um
/// campo ruim fica de fora (conta em [`Lida::descartadas`]); o que não é o
/// JSON da memória, de outra versão ou grande demais é recusado inteiro.
pub fn ler(texto: &str) -> Result<Lida, Recusa> {
    if texto.len() > MAX_BYTES {
        return Err(Recusa::Grande);
    }
    // O erro do serde_json pode citar o conteúdo: nunca é repassado.
    let valor: Value = serde_json::from_str(texto).map_err(|_| Recusa::Json)?;
    let objeto = valor.as_object().ok_or(Recusa::Json)?;
    let versao = objeto
        .get("versao")
        .and_then(Value::as_u64)
        .ok_or(Recusa::Json)?;
    if versao != VERSAO {
        return Err(Recusa::Versao(versao));
    }
    let gravada_ms = objeto
        .get("gravada_ms")
        .and_then(inteiro)
        .ok_or(Recusa::Json)?;
    let boot = match objeto.get("boot") {
        None | Some(Value::Null) => None,
        Some(v) => Some(texto_valido(v, eh_boot).ok_or(Recusa::Json)?),
    };
    // O relógio do laço e o sossego ruins valem como nenhum: as sessões
    // voltam pela parede, e nada segura o pet quieto.
    let laco_ms = objeto
        .get("laco_ms")
        .and_then(Value::as_u64)
        .filter(|n| *n <= MAX_INTEIRO_JSON);
    let sossego = objeto
        .get("sossego")
        .and_then(Value::as_object)
        .and_then(sossego)
        .unwrap_or_default();
    let lista = objeto
        .get("sessoes")
        .and_then(Value::as_array)
        .ok_or(Recusa::Json)?;
    let mut sessoes = Vec::new();
    let mut descartadas = 0;
    for item in lista {
        match item.as_object().and_then(sessao) {
            Some(s) if sessoes.len() < MAX_SESSOES => sessoes.push(s),
            _ => descartadas += 1,
        }
    }
    Ok(Lida {
        memoria: Memoria {
            versao,
            gravada_ms,
            laco_ms,
            boot,
            sossego,
            sessoes,
        },
        descartadas,
    })
}

/// O boot id: um token de até 64 (o do Linux é um UUID).
pub fn eh_boot(s: &str) -> bool {
    evento::eh_token(s, MAX_BOOT)
}

/// A instância de um compositor: um token de até 128 (a assinatura do
/// Hyprland é `<hash>_<epoch>_<rand>`).
pub fn eh_compositor(s: &str) -> bool {
    evento::eh_token(s, MAX_COMPOSITOR)
}

fn inteiro(v: &Value) -> Option<u64> {
    v.as_u64().filter(|n| (1..=MAX_INTEIRO_JSON).contains(n))
}

/// Um instante do relógio do laço de quem gravou: com sinal (o de antes da
/// partida dele é negativo), exato num número JSON.
fn instante_do_laco(v: &Value) -> Option<i64> {
    v.as_i64().filter(|n| n.unsigned_abs() <= MAX_INTEIRO_JSON)
}

fn sossego(o: &Map<String, Value>) -> Option<Sossego> {
    let booleano = |chave: &str| match o.get(chave) {
        None | Some(Value::Null) => Some(false),
        Some(v) => v.as_bool(),
    };
    Some(Sossego {
        nao_perturbe: booleano("nao_perturbe")?,
        soneca_ate_laco_ms: opcional(o, "soneca_ate_laco_ms", instante_do_laco)?,
        discricao: booleano("discricao")?,
        discricao_sinal_laco_ms: opcional(o, "discricao_sinal_laco_ms", instante_do_laco)?,
    })
}

fn texto_valido(v: &Value, regra: impl Fn(&str) -> bool) -> Option<String> {
    v.as_str().filter(|s| regra(s)).map(str::to_owned)
}

/// Um campo opcional: ausente ou `null` é `Some(None)`; presente e ruim,
/// `None` (a sessão fica de fora).
fn opcional<T>(
    objeto: &Map<String, Value>,
    chave: &str,
    regra: impl Fn(&Value) -> Option<T>,
) -> Option<Option<T>> {
    match objeto.get(chave) {
        None | Some(Value::Null) => Some(None),
        Some(v) => regra(v).map(Some),
    }
}

fn enumerado<T: serde::de::DeserializeOwned>(v: &Value) -> Option<T> {
    v.as_str()?;
    serde_json::from_value(v.clone()).ok()
}

fn sessao(o: &Map<String, Value>) -> Option<SessaoGuardada> {
    Some(SessaoGuardada {
        sid: texto_valido(o.get("sid")?, evento::eh_id)?,
        proj: opcional(o, "proj", |v| texto_valido(v, evento::eh_projeto))?,
        ent: opcional(o, "ent", |v| texto_valido(v, evento::eh_origem))?,
        estado: enumerado(o.get("estado")?)?,
        estado_desde_ms: inteiro(o.get("estado_desde_ms")?)?,
        estado_desde_laco_ms: opcional(o, "estado_desde_laco_ms", instante_do_laco)?,
        ultimo_evento_ms: inteiro(o.get("ultimo_evento_ms")?)?,
        ultimo_evento_laco_ms: opcional(o, "ultimo_evento_laco_ms", instante_do_laco)?,
        agendamentos: opcional(o, "agendamentos", |v| {
            v.as_u64().filter(|n| *n <= evento::MAX_CONTAGEM)
        })?,
        aviso: opcional(o, "aviso", |v| v.as_object().and_then(aviso))?,
        janela: opcional(o, "janela", |v| v.as_object().and_then(janela))?,
    })
}

fn aviso(o: &Map<String, Value>) -> Option<AvisoGuardado> {
    let tipo: TipoAviso = enumerado(o.get("tipo")?)?;
    let espera = opcional(o, "espera", enumerado::<TipoEspera>)?;
    // Só o aviso de espera diz o que espera.
    if espera.is_some() && tipo != TipoAviso::Esperando {
        return None;
    }
    Some(AvisoGuardado {
        tipo,
        espera,
        desde_ms: inteiro(o.get("desde_ms")?)?,
        desde_laco_ms: opcional(o, "desde_laco_ms", instante_do_laco)?,
        nivel: opcional(o, "nivel", |v| {
            v.as_u64()
                .filter(|n| (1..=4).contains(n))
                .and_then(|n| u8::try_from(n).ok())
        })?,
        vista_ms: opcional(o, "vista_ms", inteiro)?,
        vista_laco_ms: opcional(o, "vista_laco_ms", instante_do_laco)?,
    })
}

fn janela(o: &Map<String, Value>) -> Option<JanelaGuardada> {
    Some(JanelaGuardada {
        endereco: opcional(o, "endereco", |v| {
            texto_valido(v, |s| evento::eh_token(s, MAX_ENDERECO))
        })?,
        compositor: opcional(o, "compositor", |v| texto_valido(v, eh_compositor))?,
        certeza: enumerado(o.get("certeza")?)?,
        em_ms: inteiro(o.get("em_ms")?)?,
        terminal: opcional(o, "terminal", terminal)?,
    })
}

/// Os ids de terminal com as regras do fio v1: um id ruim derruba o campo
/// (e a sessão); vazio é ruim.
fn terminal(v: &Value) -> Option<Terminal> {
    let o = v.as_object()?;
    let mut t = Terminal::default();
    for (chave, valor) in o {
        match chave.as_str() {
            "tmux" => t.tmux = Some(texto_valido(valor, evento::eh_painel_tmux)?),
            "kitty" => t.kitty = Some(texto_valido(valor, evento::eh_numero_de_terminal)?),
            "wezterm" => t.wezterm = Some(texto_valido(valor, evento::eh_numero_de_terminal)?),
            _ => return None,
        }
    }
    (!t.vazio()).then_some(t)
}

#[cfg(test)]
mod testes {
    use super::*;

    const T: u64 = 1_790_000_000_000;

    fn completa() -> SessaoGuardada {
        SessaoGuardada {
            sid: "aaaaaaaa-1111-4222-8333-444444444444".into(),
            proj: Some("agenda-açaí".into()),
            ent: Some("cli".into()),
            estado: EstadoSessao::Esperando,
            estado_desde_ms: T + 10,
            estado_desde_laco_ms: Some(-990),
            ultimo_evento_ms: T + 20,
            ultimo_evento_laco_ms: Some(-980),
            agendamentos: Some(2),
            aviso: Some(AvisoGuardado {
                tipo: TipoAviso::Esperando,
                espera: Some(TipoEspera::Pergunta),
                desde_ms: T + 10,
                desde_laco_ms: Some(-990),
                nivel: Some(3),
                vista_ms: Some(T + 15),
                vista_laco_ms: Some(-985),
            }),
            janela: Some(JanelaGuardada {
                endereco: Some("5bbf4e6128f0".into()),
                compositor: Some(
                    "efb50993780079460b0cbed1363e2166a2de1d9f_1790020208_1687561921".into(),
                ),
                certeza: Certeza::Certa,
                em_ms: T + 5,
                terminal: Some(Terminal {
                    tmux: Some("%3".into()),
                    ..Terminal::default()
                }),
            }),
        }
    }

    fn memoria(sessoes: Vec<SessaoGuardada>) -> Memoria {
        Memoria {
            versao: VERSAO,
            gravada_ms: T + 30,
            laco_ms: Some(30),
            boot: Some("4e45d6f5-4cb9-4bb6-bc18-875b91983f1d".into()),
            sossego: Sossego::default(),
            sessoes,
        }
    }

    #[test]
    fn grava_e_le_de_volta_igual() {
        let m = memoria(vec![
            completa(),
            SessaoGuardada {
                sid: "bbbb".into(),
                proj: None,
                ent: Some("cli".into()),
                estado: EstadoSessao::Parada,
                estado_desde_ms: T,
                estado_desde_laco_ms: None,
                ultimo_evento_ms: T,
                ultimo_evento_laco_ms: None,
                agendamentos: None,
                aviso: None,
                janela: None,
            },
        ]);
        let texto = m.texto();
        assert!(texto.ends_with("}\n"));
        let lida = ler(&texto).unwrap();
        assert_eq!(lida.memoria, m);
        assert_eq!(lida.descartadas, 0);
        // Os enums em snake_case, os opcionais ausentes fora do arquivo.
        assert!(texto.contains(r#""estado": "esperando""#), "{texto}");
        assert!(texto.contains(r#""certeza": "certa""#));
        assert!(!texto.contains("null"), "{texto}");
    }

    #[test]
    fn um_campo_ruim_tira_so_a_sessao_dele() {
        let boa = serde_json::to_value(completa()).unwrap();
        let ruins: Vec<(&str, Value)> = vec![
            ("sid", Value::from("tem espaço")),
            ("sid", Value::from(42)),
            ("proj", Value::from("../etc")),
            ("ent", Value::from("CLI")),
            ("estado", Value::from("dormindo_muito")),
            ("estado_desde_ms", Value::from(-1)),
            ("estado_desde_laco_ms", Value::from(1.5)),
            ("ultimo_evento_ms", Value::from("ontem")),
            ("ultimo_evento_laco_ms", Value::from("ontem")),
            ("ultimo_evento_laco_ms", Value::from(-(1_i64 << 54))),
            ("agendamentos", Value::from(10_001)),
            (
                "aviso",
                serde_json::json!({"tipo": "pronto", "espera": "pergunta", "desde_ms": T}),
            ),
            (
                "aviso",
                serde_json::json!({"tipo": "esperando", "desde_ms": T, "nivel": 5}),
            ),
            ("aviso", serde_json::json!({"tipo": "outro", "desde_ms": T})),
            (
                "aviso",
                serde_json::json!({"tipo": "pronto", "desde_ms": T, "desde_laco_ms": "x"}),
            ),
            (
                "aviso",
                serde_json::json!({"tipo": "esperando", "desde_ms": T, "vista_laco_ms": true}),
            ),
            (
                "janela",
                serde_json::json!({"endereco": "a b", "certeza": "certa", "em_ms": T}),
            ),
            (
                "janela",
                serde_json::json!({"compositor": "/tmp/x", "certeza": "certa", "em_ms": T}),
            ),
            (
                "janela",
                serde_json::json!({"certeza": "talvez", "em_ms": T}),
            ),
            (
                "janela",
                serde_json::json!({"certeza": "certa", "em_ms": T, "terminal": {"tmux": "3"}}),
            ),
            (
                "janela",
                serde_json::json!({"certeza": "certa", "em_ms": T, "terminal": {"titulo": "x"}}),
            ),
        ];
        for (campo, valor) in ruins {
            let mut ruim = boa.clone();
            ruim[campo] = valor.clone();
            let texto = serde_json::json!({
                "versao": 1, "gravada_ms": T, "boot": "b", "sessoes": [boa.clone(), ruim]
            })
            .to_string();
            let lida = ler(&texto).unwrap_or_else(|e| panic!("{campo}: {e}"));
            assert_eq!(
                (lida.memoria.sessoes.len(), lida.descartadas),
                (1, 1),
                "{campo} = {valor}"
            );
        }
        // Um campo obrigatório que falta também.
        let mut sem_sid = boa.clone();
        sem_sid.as_object_mut().unwrap().remove("sid");
        let texto = serde_json::json!({"versao": 1, "gravada_ms": T, "sessoes": [sem_sid]});
        assert_eq!(ler(&texto.to_string()).unwrap().descartadas, 1);
    }

    #[test]
    fn o_arquivo_ruim_de_outra_versao_ou_grande_e_recusado_sem_citar_nada() {
        for (texto, recusa) in [
            ("", Recusa::Json),
            ("{\"versao\": 1, \"segredo", Recusa::Json),
            ("[1, 2]", Recusa::Json),
            (r#"{"gravada_ms": 1, "sessoes": []}"#, Recusa::Json),
            (r#"{"versao": 1, "sessoes": []}"#, Recusa::Json),
            (
                r#"{"versao": 1, "gravada_ms": 1, "sessoes": {}}"#,
                Recusa::Json,
            ),
            (
                r#"{"versao": 1, "gravada_ms": 1, "boot": "a b", "sessoes": []}"#,
                Recusa::Json,
            ),
            (
                r#"{"versao": 2, "gravada_ms": 1, "sessoes": []}"#,
                Recusa::Versao(2),
            ),
        ] {
            assert_eq!(ler(texto), Err(recusa), "{texto}");
        }
        let grande = format!(
            r#"{{"versao": 1, "gravada_ms": 1, "sessoes": [], "x": "{}"}}"#,
            "a".repeat(MAX_BYTES)
        );
        assert_eq!(ler(&grande), Err(Recusa::Grande));
        for recusa in [
            Recusa::Grande,
            Recusa::Json,
            Recusa::Versao(7),
            Recusa::MaquinaReiniciou,
            Recusa::SemBoot,
            Recusa::Ilegivel,
        ] {
            let texto = recusa.to_string();
            assert!(!texto.contains("segredo") && !texto.is_empty());
            assert!(!recusa.motivo().is_empty());
        }
    }

    #[test]
    fn so_mais_sessoes_que_o_teto_ficam_de_fora() {
        let sessoes: Vec<SessaoGuardada> = (0..MAX_SESSOES + 3)
            .map(|i| SessaoGuardada {
                sid: format!("s{i}"),
                ..completa()
            })
            .collect();
        let lida = ler(&memoria(sessoes).texto()).unwrap();
        assert_eq!(lida.memoria.sessoes.len(), MAX_SESSOES);
        assert_eq!(lida.descartadas, 3);
    }

    #[test]
    fn o_sossego_e_o_relogio_do_laco_vao_e_voltam_e_os_ruins_valem_como_nenhum() {
        let mut m = memoria(vec![completa()]);
        m.sossego = Sossego {
            nao_perturbe: true,
            soneca_ate_laco_ms: Some(1_200_000),
            discricao: true,
            discricao_sinal_laco_ms: Some(-40_000),
        };
        let texto = m.texto();
        assert!(texto.contains(r#""laco_ms": 30"#), "{texto}");
        assert_eq!(ler(&texto).unwrap().memoria, m);
        // Sem nada segurando o pet, o sossego nem vai para o arquivo.
        assert!(!memoria(Vec::new()).texto().contains("sossego"));
        // Um sossego ou um relógio do laço ruins: as sessões voltam, pela
        // parede, e nada segura o pet quieto.
        let boa = serde_json::to_value(completa()).unwrap();
        for (chave, valor) in [
            ("sossego", serde_json::json!({"nao_perturbe": "sim"})),
            ("sossego", serde_json::json!({"soneca_ate_laco_ms": 1.5})),
            ("sossego", serde_json::json!([true])),
            ("laco_ms", Value::from(-1)),
        ] {
            let mut arquivo = serde_json::json!({
                "versao": 1, "gravada_ms": T, "laco_ms": 30, "boot": "b", "sessoes": [boa.clone()]
            });
            arquivo[chave] = valor.clone();
            let lida = ler(&arquivo.to_string()).unwrap_or_else(|e| panic!("{chave}: {e}"));
            assert_eq!(lida.memoria.sessoes.len(), 1, "{chave} = {valor}");
            assert!(lida.memoria.sossego.vazio(), "{chave} = {valor}");
            if chave == "laco_ms" {
                assert_eq!(lida.memoria.laco_ms, None);
            }
        }
    }

    #[test]
    fn a_volta_conta_o_tempo_acordado_ate_a_gravacao_e_a_parada_pela_parede() {
        // O pet A gravou com o relógio do laço em 10 h (36 000 000 ms) e a
        // parede 14 h depois da partida dele: a máquina dormiu 4 h. O último
        // evento foi 1 h de relógio do laço antes da gravação (a suspensão
        // veio depois dele).
        const H: u64 = 60 * 60 * 1000;
        let mut m = memoria(Vec::new());
        m.gravada_ms = T + 14 * H;
        m.laco_ms = Some(10 * H);
        let evento = instante(9 * H);
        // O pet B parte 20 s depois da gravação, com o relógio do laço em 0.
        let agora = Agora {
            parede_ms: T + 14 * H + 20_000,
            mono_ms: 0,
        };
        let volta = Volta::de(&m, agora);
        let idade = 1_000 * 60 * 60 + 20_000;
        assert_eq!(
            volta.instante(Some(evento), T),
            -(idade as i64),
            "1 h acordado mais os 20 s da parada, não as 5 h da parede"
        );
        assert_eq!(volta.gravacao(), -20_000);
        // Sem o relógio do laço (o arquivo da decisão 0093), pela parede.
        assert_eq!(volta.instante(None, T + 9 * H), agora.no_laco(T + 9 * H));
        m.laco_ms = None;
        assert_eq!(
            Volta::de(&m, agora).instante(Some(evento), T + 9 * H),
            -(5 * H as i64) - 20_000
        );
        assert!(!volta.velha(), "20 s");
        assert!(!volta.depois_da_gravacao(Some(evento)));
        assert!(volta.depois_da_gravacao(Some(instante(10 * H) + 1)));
        // A parada de mais de 60 s, ou o relógio que voltou mais que isso.
        for parede in [
            T + 14 * H + MEMORIA_VELHA_MS + 1,
            T + 14 * H - MEMORIA_VELHA_MS - 1,
        ] {
            let volta = Volta::de(
                &m,
                Agora {
                    parede_ms: parede,
                    mono_ms: 0,
                },
            );
            assert!(volta.velha(), "{parede}");
        }
    }

    #[test]
    fn o_boot_id_diz_se_a_maquina_reiniciou() {
        let m = memoria(Vec::new());
        assert_eq!(
            m.conferir_boot(Some("4e45d6f5-4cb9-4bb6-bc18-875b91983f1d")),
            Ok(())
        );
        assert_eq!(
            m.conferir_boot(Some("00000000-0000-0000-0000-000000000000")),
            Err(Recusa::MaquinaReiniciou)
        );
        assert_eq!(m.conferir_boot(None), Err(Recusa::SemBoot));
        let sem = Memoria::nova(T, None);
        assert_eq!(sem.conferir_boot(Some("x")), Err(Recusa::SemBoot));
        assert!(eh_boot("4e45d6f5-4cb9-4bb6-bc18-875b91983f1d"));
        assert!(!eh_boot("4e45d6f5 4cb9"));
        assert!(eh_compositor(
            "efb50993780079460b0cbed1363e2166a2de1d9f_1790020208_1687561921"
        ));
        assert!(!eh_compositor("../hypr"));
    }
}
