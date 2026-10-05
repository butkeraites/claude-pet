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
//! caminho. Os tempos são de parede (ms desde 1970): o relógio do laço
//! recomeça do zero com o processo, e quem restaura conta os prazos de novo a
//! partir deles ([`crate::cerebro::Agora::no_laco`]).
//!
//! **O arquivo** tem versão ([`VERSAO`]), o boot id da máquina (uma
//! reinicialização mata todo Claude: as sessões de antes seriam fantasmas por
//! 12 h) e no máximo [`MAX_SESSOES`] sessões em até [`MAX_BYTES`]. Quem lê
//! confere campo a campo com as regras do fio v1 (`crate::evento`): uma sessão
//! com um campo ruim fica de fora; um arquivo que não é este JSON, de outra
//! versão ou grande demais é ignorado inteiro. Os erros nunca citam o
//! conteúdo.
//!
//! Quem grava e lê o arquivo, e lê o boot id, é o daemon (o núcleo é puro).

use std::fmt;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::cerebro::{EstadoSessao, MAX_SESSOES, TipoAviso, TipoEspera};
use crate::evento::{self, Terminal};
use crate::motor::janelas::Certeza;

/// A versão do arquivo que este pet escreve e lê.
pub const VERSAO: u64 = 1;
/// O arquivo, dentro da pasta do estado (`/state` no container).
pub const ARQUIVO: &str = "sessoes.json";
/// Maior arquivo que o pet lê: 64 sessões cabem em uns 40 KiB.
pub const MAX_BYTES: usize = 256 * 1024;
/// O maior endereço de janela e a maior instância de compositor aceitos.
const MAX_ENDERECO: usize = 64;
const MAX_COMPOSITOR: usize = 128;
/// O boot id do Linux é um UUID (36 caracteres).
const MAX_BOOT: usize = 64;
/// O maior inteiro exato num número JSON (o mesmo do `ts` do fio v1).
const MAX_INTEIRO_JSON: u64 = (1 << 53) - 1;

/// O arquivo da memória.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Memoria {
    pub versao: u64,
    /// Quando foi gravada (ms desde 1970).
    pub gravada_ms: u64,
    /// O boot id da máquina na gravação (no Linux,
    /// `/proc/sys/kernel/random/boot_id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot: Option<String>,
    pub sessoes: Vec<SessaoGuardada>,
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
    pub ultimo_evento_ms: u64,
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
    /// O nível da escalada, no aviso de espera que o pet chamava.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nivel: Option<u8>,
    /// Quando o Renan viu o diálogo no terminal da sessão (decisão 0090).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vista_ms: Option<u64>,
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
        }
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
            boot,
            sessoes: Vec::new(),
        }
    }

    /// O texto do arquivo.
    pub fn texto(&self) -> String {
        let mut texto = serde_json::to_string_pretty(self).unwrap_or_default();
        texto.push('\n');
        texto
    }

    /// A memória é desta partida da máquina: o boot id da gravação é o de
    /// agora. Sem um dos dois, não se sabe, e nada volta (uma sessão
    /// fantasma por 12 h é pior que esquecer).
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
            boot,
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
        ultimo_evento_ms: inteiro(o.get("ultimo_evento_ms")?)?,
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
        nivel: opcional(o, "nivel", |v| {
            v.as_u64()
                .filter(|n| (1..=4).contains(n))
                .and_then(|n| u8::try_from(n).ok())
        })?,
        vista_ms: opcional(o, "vista_ms", inteiro)?,
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
            ultimo_evento_ms: T + 20,
            agendamentos: Some(2),
            aviso: Some(AvisoGuardado {
                tipo: TipoAviso::Esperando,
                espera: Some(TipoEspera::Pergunta),
                desde_ms: T + 10,
                nivel: Some(3),
                vista_ms: Some(T + 15),
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
            boot: Some("4e45d6f5-4cb9-4bb6-bc18-875b91983f1d".into()),
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
                ultimo_evento_ms: T,
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
            ("ultimo_evento_ms", Value::from("ontem")),
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
