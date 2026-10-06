//! Observador de transcript (decisão 0108): uma thread que lê os transcripts
//! do Claude Code (`~/.claude/projects/*/*.jsonl`) e dá ao Zeca os eventos das
//! sessões que o hook ainda não cobre — as abertas antes de o plugin ser
//! instalado, que não mandam nada até um `/reload-plugins`. Alimenta o pet
//! pelo MESMO caminho do hook (a [`Caixa`] do laço), com os mesmos [`Evento`]s
//! do fio v1. Multiplataforma: `~/.claude` existe em todo sistema.
//!
//! **Privacidade** (como o hook): lê a linha só para o metadado do fio
//! ([`pet_core::transcript`]), descarta o resto na hora e **nunca** loga,
//! guarda nem manda o texto do prompt ou da resposta. O que vai ao pet são só
//! `sessionId`, `promptId`, a última pasta do `cwd`, o `entrypoint` e a forma
//! do turno.
//!
//! **Dedup:** não adota uma sessão que o pet já vê (o hook a alimenta, visto
//! pelo `sid8` do `/v1/estado`); as demais sessões ativas, adota — semeia um
//! `SessionStart` e segue o fim do arquivo. Se o Renan der `/reload` numa
//! adotada, o cérebro funde pelo sid completo e a idempotência dele (o Stop
//! repetido, o turno pelo `prompt_id`) evita a reação dobrada.
//!
//! Limites deste primeiro corte: a janela do terminal (para o clique) não vem
//! do transcript, só do hook; o `ts` é o do relógio de agora, não o da linha.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde_json::Value;

use pet_core::evento::Evento;
use pet_core::plataforma::Caixa;
use pet_core::transcript;

use crate::comando::{Comando, Recebido};
use crate::estado::Compartilhado;
use crate::ingress::agora_desde_1970_ms;

/// De quanto em quanto tempo varre os transcripts.
const POLL: Duration = Duration::from_secs(2);
/// Um jsonl mexido nos últimos 30 min conta como sessão aberta.
const JANELA_ATIVA: Duration = Duration::from_secs(30 * 60);

/// Uma sessão que o observador adotou (o hook não a cobria). O sid vem de cada
/// linha (todas têm `sessionId`), então não é guardado aqui.
struct Adotada {
    /// Bytes já lidos do arquivo.
    offset: u64,
    /// O `promptId` do último turno aberto (para não repetir o prompt).
    turno: Option<String>,
}

/// Sobe a thread do observador, se ligado ([`ligado`]). Recebe um clone da
/// [`Caixa`] do laço (que já carrega o `Despertador` da plataforma).
pub fn iniciar(comp: Arc<Compartilhado>, caixa: Caixa<Comando>) {
    if !ligado() {
        return;
    }
    let Some(raiz) = pasta_dos_projetos() else {
        return;
    };
    let _ = thread::Builder::new()
        .name("observador".into())
        .spawn(move || rodar(&raiz, &comp, &caixa));
}

/// Ligado por padrão; `PET_OBSERVADOR=0` desliga (privacidade: o daemon lê os
/// transcripts). Um liga/desliga no config é o próximo passo.
fn ligado() -> bool {
    std::env::var("PET_OBSERVADOR").ok().as_deref() != Some("0")
}

/// `PET_PROJETOS` (teste) ou `~/.claude/projects`.
fn pasta_dos_projetos() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("PET_PROJETOS") {
        return Some(PathBuf::from(p));
    }
    let casa = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(casa).join(".claude").join("projects"))
}

fn rodar(raiz: &Path, comp: &Compartilhado, caixa: &Caixa<Comando>) {
    let mut adotadas: HashMap<PathBuf, Option<Adotada>> = HashMap::new();
    loop {
        varrer(raiz, comp, caixa, &mut adotadas);
        thread::sleep(POLL);
    }
}

fn varrer(
    raiz: &Path,
    comp: &Compartilhado,
    caixa: &Caixa<Comando>,
    adotadas: &mut HashMap<PathBuf, Option<Adotada>>,
) {
    let vistas = sids_do_pet(comp);
    for arquivo in jsonl_ativos(raiz) {
        if !adotadas.contains_key(&arquivo) {
            let ad = primeiro_contato(&arquivo, &vistas, caixa);
            adotadas.insert(arquivo.clone(), ad);
        }
        if let Some(Some(ad)) = adotadas.get_mut(&arquivo) {
            seguir(&arquivo, ad, caixa);
        }
    }
}

/// O `sid8` das sessões que o pet já vê (o hook alimenta).
fn sids_do_pet(comp: &Compartilhado) -> BTreeSet<String> {
    comp.estado_json()
        .get("sessoes")
        .and_then(Value::as_array)
        .map(|sessoes| {
            sessoes
                .iter()
                .filter_map(|s| s.get("sid8").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn sid8(sid: &str) -> &str {
    sid.get(..8).unwrap_or(sid)
}

/// Primeiro contato com um arquivo: lê-o uma vez, decide adotar (sessão `cli`
/// que o hook não cobre), semeia o estado e marca de onde seguir.
fn primeiro_contato(
    arquivo: &Path,
    vistas: &BTreeSet<String>,
    caixa: &Caixa<Comando>,
) -> Option<Adotada> {
    let dados = fs::read_to_string(arquivo).ok()?;
    let mut sid: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut ent: Option<String> = None;
    let mut aberto = false;
    let mut ultimo_prompt: Option<Value> = None;
    for linha in dados.lines() {
        let Ok(v) = serde_json::from_str::<Value>(linha) else {
            continue;
        };
        if sid.is_none() {
            sid = transcript::sid(&v).map(str::to_owned);
        }
        if let Some(c) = v.get("cwd").and_then(Value::as_str) {
            cwd = Some(c.to_owned());
        }
        if let Some(e) = transcript::entrypoint(&v) {
            ent = Some(e.to_owned());
        }
        if transcript::eh_prompt(&v) {
            aberto = true;
            ultimo_prompt = Some(v);
        } else if transcript::eh_fim_de_turno(&v) {
            aberto = false;
        }
    }

    let sid = sid?;
    // Só sessões interativas (o cérebro só conta `cli`); e nunca o que o hook
    // já cobre.
    if ent.as_deref() != Some("cli") || vistas.contains(sid8(&sid)) {
        return None;
    }

    mandar(
        caixa,
        transcript::inicio(&sid, cwd.as_deref(), ent.as_deref()),
    );
    let turno = ultimo_prompt
        .as_ref()
        .and_then(|v| v.get("promptId").and_then(Value::as_str))
        .map(str::to_owned);
    if aberto {
        // A sessão está no meio de um turno: mostra "trabalhando".
        if let Some(ev) = ultimo_prompt.as_ref().and_then(transcript::classificar) {
            mandar(caixa, ev);
        }
    }
    Some(Adotada {
        offset: dados.len() as u64,
        turno,
    })
}

/// Segue o fim de um arquivo adotado: lê as linhas completas novas e manda os
/// prompts (de turno novo) e os fins de turno.
fn seguir(arquivo: &Path, ad: &mut Adotada, caixa: &Caixa<Comando>) {
    let Ok(mut f) = fs::File::open(arquivo) else {
        return;
    };
    let Ok(tam) = f.metadata().map(|m| m.len()) else {
        return;
    };
    if tam < ad.offset {
        ad.offset = 0; // arquivo truncado/rotacionado
    }
    if tam <= ad.offset || f.seek(SeekFrom::Start(ad.offset)).is_err() {
        return;
    }
    let mut buf = String::new();
    if f.read_to_string(&mut buf).is_err() {
        return;
    }
    let mut consumido: usize = 0;
    for linha in buf.split_inclusive('\n') {
        if !linha.ends_with('\n') {
            break; // linha ainda incompleta; fica para a próxima varredura
        }
        consumido += linha.len();
        let Ok(v) = serde_json::from_str::<Value>(linha.trim_end()) else {
            continue;
        };
        if transcript::eh_prompt(&v) {
            let pid = v.get("promptId").and_then(Value::as_str).map(str::to_owned);
            if pid != ad.turno {
                ad.turno = pid;
                if let Some(ev) = transcript::classificar(&v) {
                    mandar(caixa, ev);
                }
            }
        } else if transcript::eh_fim_de_turno(&v) {
            if let Some(ev) = transcript::classificar(&v) {
                mandar(caixa, ev);
            }
        }
    }
    ad.offset += consumido as u64;
}

/// Põe o relógio e enfileira o evento na caixa do laço (acorda o laço).
fn mandar(caixa: &Caixa<Comando>, mut evento: Evento) {
    let agora = agora_desde_1970_ms();
    evento.ts = Some(agora);
    let recebido = Recebido {
        evento,
        recebido_ms: agora,
        chegada: Instant::now(),
    };
    let _ = caixa.tentar(Comando::Evento(Box::new(recebido)));
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::mpsc::Receiver;

    use pet_core::plataforma::Despertador;

    struct Nada;
    impl Despertador for Nada {
        fn despertar(&self) {}
    }

    fn caixa() -> (Caixa<Comando>, Receiver<Comando>) {
        Caixa::nova(64, Arc::new(Nada))
    }

    fn eventos(rx: &Receiver<Comando>) -> Vec<Evento> {
        let mut v = Vec::new();
        while let Ok(Comando::Evento(r)) = rx.try_recv() {
            v.push(r.evento);
        }
        v
    }

    /// Um arquivo de transcript de mentira numa pasta temporária.
    fn arquivo_com(linhas: &[&str]) -> (PathBuf, PathBuf) {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let raiz = std::env::temp_dir().join(format!("pet-obs-{}-{n}", std::process::id()));
        let proj = raiz.join("-x-proj");
        fs::create_dir_all(&proj).unwrap();
        let arq = proj.join("sessao.jsonl");
        let mut texto = linhas.join("\n");
        texto.push('\n');
        fs::write(&arq, texto).unwrap();
        (raiz, arq)
    }

    const PROMPT: &str = r#"{"type":"user","sessionId":"aaaaaaaa-1111","promptId":"p1","cwd":"/x/proj","entrypoint":"cli","turnOrigin":"human","message":{"role":"user","content":"NÃO LER"}}"#;
    const FIM: &str = r#"{"type":"assistant","sessionId":"aaaaaaaa-1111","promptId":"p1","cwd":"/x/proj","entrypoint":"cli","message":{"role":"assistant","stop_reason":"end_turn","content":[]}}"#;

    #[test]
    fn adota_e_semeia_sessao_aberta() {
        let (_raiz, arq) = arquivo_com(&[PROMPT]); // turno aberto (sem fim)
        let (cx, rx) = caixa();
        let ad = primeiro_contato(&arq, &BTreeSet::new(), &cx).expect("adota");
        let evs = eventos(&rx);
        assert_eq!(evs.len(), 2, "SessionStart + UserPromptSubmit");
        assert_eq!(evs[0].e, "SessionStart");
        assert_eq!(evs[0].proj.as_deref(), Some("proj"));
        assert_eq!(evs[1].e, "UserPromptSubmit");
        assert_eq!(evs[1].sid.as_deref(), Some("aaaaaaaa-1111"));
        assert_eq!(ad.turno.as_deref(), Some("p1"));
        fs::remove_dir_all(_raiz).ok();
    }

    #[test]
    fn sessao_fechada_so_semeia_sessionstart() {
        let (_raiz, arq) = arquivo_com(&[PROMPT, FIM]); // turno fechado
        let (cx, rx) = caixa();
        primeiro_contato(&arq, &BTreeSet::new(), &cx).expect("adota");
        let evs = eventos(&rx);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].e, "SessionStart");
        fs::remove_dir_all(_raiz).ok();
    }

    #[test]
    fn nao_adota_sessao_que_o_hook_ja_ve() {
        let (_raiz, arq) = arquivo_com(&[PROMPT]);
        let (cx, rx) = caixa();
        let vistas: BTreeSet<String> = [String::from("aaaaaaaa")].into_iter().collect();
        assert!(
            primeiro_contato(&arq, &vistas, &cx).is_none(),
            "o hook cobre"
        );
        assert!(eventos(&rx).is_empty());
        fs::remove_dir_all(_raiz).ok();
    }

    #[test]
    fn nao_adota_sessao_nao_cli() {
        let sdk = r#"{"type":"user","sessionId":"bbbbbbbb-2222","promptId":"p1","cwd":"/x/proj","entrypoint":"sdk-cli","turnOrigin":"human","message":{"role":"user","content":"x"}}"#;
        let (_raiz, arq) = arquivo_com(&[sdk]);
        let (cx, rx) = caixa();
        assert!(primeiro_contato(&arq, &BTreeSet::new(), &cx).is_none());
        assert!(eventos(&rx).is_empty());
        fs::remove_dir_all(_raiz).ok();
    }

    #[test]
    fn seguir_le_so_as_linhas_novas() {
        let (_raiz, arq) = arquivo_com(&[PROMPT, FIM]);
        let (cx, rx) = caixa();
        let mut ad = primeiro_contato(&arq, &BTreeSet::new(), &cx).expect("adota");
        let _ = eventos(&rx); // limpa o SessionStart
        // chega um turno novo no arquivo
        let p2 = r#"{"type":"user","sessionId":"aaaaaaaa-1111","promptId":"p2","cwd":"/x/proj","entrypoint":"cli","turnOrigin":"human","message":{"role":"user","content":"y"}}"#;
        let fim2 = r#"{"type":"assistant","sessionId":"aaaaaaaa-1111","promptId":"p2","message":{"stop_reason":"end_turn","content":[]}}"#;
        let mut f = std::fs::OpenOptions::new().append(true).open(&arq).unwrap();
        use std::io::Write;
        writeln!(f, "{p2}").unwrap();
        writeln!(f, "{fim2}").unwrap();
        seguir(&arq, &mut ad, &cx);
        let evs = eventos(&rx);
        assert_eq!(
            evs.iter().map(|e| e.e.as_str()).collect::<Vec<_>>(),
            ["UserPromptSubmit", "Stop"]
        );
        assert_eq!(evs[0].turno.as_deref(), Some("p2"));
        fs::remove_dir_all(_raiz).ok();
    }

    #[test]
    fn jsonl_ativos_acha_o_arquivo() {
        let (raiz, _arq) = arquivo_com(&[PROMPT]);
        assert_eq!(jsonl_ativos(&raiz).len(), 1);
        fs::remove_dir_all(raiz).ok();
    }
}

/// Os jsonl mexidos na [`JANELA_ATIVA`] (sessões abertas), em `raiz/*/*.jsonl`.
fn jsonl_ativos(raiz: &Path) -> Vec<PathBuf> {
    let agora = SystemTime::now();
    let mut ativos = Vec::new();
    let Ok(projetos) = fs::read_dir(raiz) else {
        return ativos;
    };
    for projeto in projetos.flatten() {
        let Ok(arquivos) = fs::read_dir(projeto.path()) else {
            continue;
        };
        for arquivo in arquivos.flatten() {
            let caminho = arquivo.path();
            if caminho.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let recente = arquivo
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|m| agora.duration_since(m).ok())
                .is_some_and(|idade| idade <= JANELA_ATIVA);
            if recente {
                ativos.push(caminho);
            }
        }
    }
    ativos
}
