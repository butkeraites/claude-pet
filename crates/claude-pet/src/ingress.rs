//! Entrada HTTP dos hooks (decisão 0008).
//!
//! HTTP/1.1 feito à mão, sem dependências: `Connection: close`, timeouts de
//! 1 s, cabeçalhos e corpo limitados a 8 KiB, `Content-Length` obrigatório
//! em POST (chunked recebe 411). O corpo nunca vai para log.
//!
//! As rotas `/v1/*` só aceitam `Host` de loopback na porta pública e o
//! cabeçalho `X-Pet: 1`: isso barra requisições disparadas por navegador
//! (DNS rebinding, CSRF). As que recebem corpo exigem também
//! `Content-Type: application/json` (415), coisa que um formulário HTML não
//! consegue mandar sem preflight.
//!
//! - `GET /v1/estado`: o que o pet está vendo e pensando (só metadados).
//! - `POST /v1/evento`: um evento dos hooks no formato de fio v1
//!   (decisão 0019, [`pet_core::evento`]); 204, ou 400 sem `v`/`e` válidos.
//!   Vai para o laço principal pelo canal do calloop.
//! - `POST /v1/comando`, sempre `{"cmd": …, "arg": …}` e mais nada:
//!   - `tocar` com o nome da reação, `esconder` e `mostrar` (M3, decisão
//!     0019) vão para o laço principal sem esperar; 204, e nada persiste;
//!   - `aprovar_skin` com `{"id", "sha256"}` aprova o conteúdo exato da skin
//!     da imagem e guarda a cópia em `/state`; `revogar_skin` com o id apaga
//!     a aprovação (M2, decisão 0026). Uma por vez; a resposta (200, JSON)
//!     sai depois de o laço principal escolher o personagem de novo.
//!
//! Com `PET_DEBUG=1` existem também as rotas `/v1/debug/*` (mesmas
//! checagens): `eventos` (os últimos 200 eventos, já validados), `quadro`
//! (o RGBA esperado do sprite, para a checagem de nitidez), `esconder`,
//! `mostrar` e `estresse`. Sem debug elas respondem 404, como se não
//! existissem.

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pet_core::evento::{self, Lido};
use pet_core::skin::codificar_png;
use serde::Deserialize;
use serde_json::{Value, json};
use smithay_client_toolkit::reexports::calloop::channel::SyncSender;

use crate::aprovacao;
use crate::comando::{Comando, QuadroEsperado, Recebido};
use crate::estado::Compartilhado;
use crate::personagem::Onde;

pub const LIMITE_CABECALHOS: usize = 8 * 1024;
pub const LIMITE_CORPO: usize = 8 * 1024;
const MAX_CONEXOES: usize = 16;
const TEMPO_LIMITE: Duration = Duration::from_secs(1);
/// Quanto o `/v1/debug/quadro` espera o laço principal responder.
const ESPERA_QUADRO: Duration = Duration::from_secs(2);

#[derive(Debug, PartialEq, Eq)]
pub struct Requisicao {
    pub metodo: String,
    pub caminho: String,
    /// Nome em minúsculas, valor sem espaços nas pontas.
    pub cabecalhos: Vec<(String, String)>,
    pub corpo: Vec<u8>,
}

impl Requisicao {
    pub fn cabecalho(&self, nome: &str) -> Option<&str> {
        self.cabecalhos
            .iter()
            .find(|(n, _)| n == nome)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErroHttp {
    Malformada,
    SemTamanho,
    CorpoGrande,
    CabecalhosGrandes,
    /// Conexão caiu ou estourou o tempo: não há a quem responder.
    Conexao,
}

impl ErroHttp {
    fn status(&self) -> Option<u16> {
        match self {
            ErroHttp::Malformada => Some(400),
            ErroHttp::SemTamanho => Some(411),
            ErroHttp::CorpoGrande => Some(413),
            ErroHttp::CabecalhosGrandes => Some(431),
            ErroHttp::Conexao => None,
        }
    }
}

/// Contexto das rotas.
pub struct Contexto {
    pub comp: Arc<Compartilhado>,
    pub porta_publica: u16,
    /// Rotas `/v1/debug/*` ligadas (`PET_DEBUG=1`).
    pub debug: bool,
    /// Canal para o laço principal.
    pub comandos: Option<SyncSender<Comando>>,
    /// Onde estão as skins da imagem e as aprovações (`/state`).
    pub onde: Onde,
    /// Uma aprovação ou revogação por vez.
    pub aprovando: Mutex<()>,
}

pub fn ler_requisicao(entrada: &mut impl Read) -> Result<Requisicao, ErroHttp> {
    let mut lido = Vec::with_capacity(1024);
    let mut pedaco = [0u8; 1024];
    let fim_cabecalhos = loop {
        if let Some(pos) = achar(&lido, b"\r\n\r\n") {
            break pos;
        }
        if lido.len() > LIMITE_CABECALHOS {
            return Err(ErroHttp::CabecalhosGrandes);
        }
        let n = entrada.read(&mut pedaco).map_err(|_| ErroHttp::Conexao)?;
        if n == 0 {
            return Err(if lido.is_empty() {
                ErroHttp::Conexao
            } else {
                ErroHttp::Malformada
            });
        }
        lido.extend_from_slice(&pedaco[..n]);
    };
    if fim_cabecalhos > LIMITE_CABECALHOS {
        return Err(ErroHttp::CabecalhosGrandes);
    }

    let texto = std::str::from_utf8(&lido[..fim_cabecalhos]).map_err(|_| ErroHttp::Malformada)?;
    let mut linhas = texto.split("\r\n");
    let primeira = linhas.next().ok_or(ErroHttp::Malformada)?;
    let mut partes = primeira.split(' ');
    let (Some(metodo), Some(caminho), Some(versao), None) =
        (partes.next(), partes.next(), partes.next(), partes.next())
    else {
        return Err(ErroHttp::Malformada);
    };
    if metodo.is_empty() || !caminho.starts_with('/') || !versao.starts_with("HTTP/1.") {
        return Err(ErroHttp::Malformada);
    }

    let mut cabecalhos = Vec::new();
    for linha in linhas {
        let (nome, valor) = linha.split_once(':').ok_or(ErroHttp::Malformada)?;
        let nome = nome.trim();
        if nome.is_empty() || nome.contains(' ') {
            return Err(ErroHttp::Malformada);
        }
        cabecalhos.push((nome.to_ascii_lowercase(), valor.trim().to_owned()));
    }
    let mut requisicao = Requisicao {
        metodo: metodo.to_owned(),
        caminho: caminho.to_owned(),
        cabecalhos,
        corpo: Vec::new(),
    };

    if requisicao.cabecalho("transfer-encoding").is_some() {
        return Err(ErroHttp::SemTamanho);
    }
    let tamanho = match requisicao.cabecalho("content-length") {
        Some(valor) => valor.parse::<usize>().map_err(|_| ErroHttp::Malformada)?,
        None if requisicao.metodo == "POST" => return Err(ErroHttp::SemTamanho),
        None => 0,
    };
    if tamanho > LIMITE_CORPO {
        return Err(ErroHttp::CorpoGrande);
    }

    let mut corpo = lido[fim_cabecalhos + 4..].to_vec();
    corpo.truncate(tamanho);
    while corpo.len() < tamanho {
        let n = entrada.read(&mut pedaco).map_err(|_| ErroHttp::Conexao)?;
        if n == 0 {
            return Err(ErroHttp::Malformada);
        }
        let falta = tamanho - corpo.len();
        corpo.extend_from_slice(&pedaco[..n.min(falta)]);
    }
    requisicao.corpo = corpo;
    Ok(requisicao)
}

pub fn escrever_resposta(saida: &mut impl Write, status: u16, corpo: &str) -> io::Result<()> {
    let razao = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        409 => "Conflict",
        404 => "Not Found",
        422 => "Unprocessable Content",
        500 => "Internal Server Error",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Status",
    };
    if status == 204 {
        write!(
            saida,
            "HTTP/1.1 204 {razao}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n"
        )
    } else {
        write!(
            saida,
            "HTTP/1.1 {status} {razao}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{corpo}",
            corpo.len()
        )
    }
}

/// `Host` precisa ser loopback (`127.0.0.1` ou `localhost`) na porta pública.
pub fn host_valido(host: Option<&str>, porta_publica: u16) -> bool {
    let Some((nome, porta)) = host.and_then(|h| h.rsplit_once(':')) else {
        return false;
    };
    (nome == "127.0.0.1" || nome == "localhost") && porta.parse::<u16>() == Ok(porta_publica)
}

fn erro_json(motivo: &str) -> String {
    json!({ "erro": motivo }).to_string()
}

pub fn rotear(req: &Requisicao, ctx: &Contexto) -> (u16, String) {
    let caminho = req.caminho.split('?').next().unwrap_or_default();
    if caminho == "/saude" {
        if req.metodo != "GET" {
            return (405, erro_json("use GET"));
        }
        let status = if ctx.comp.saudavel() { 200 } else { 503 };
        return (status, ctx.comp.saude_json().to_string());
    }
    if !caminho.starts_with("/v1/") {
        return (404, erro_json("rota desconhecida"));
    }
    if !host_valido(req.cabecalho("host"), ctx.porta_publica) {
        return (
            403,
            erro_json("Host precisa ser 127.0.0.1 ou localhost na porta do pet"),
        );
    }
    if req.cabecalho("x-pet") != Some("1") {
        return (403, erro_json("falta o cabeçalho X-Pet: 1"));
    }
    match (req.metodo.as_str(), caminho) {
        ("GET", "/v1/estado") => (200, ctx.comp.estado_json().to_string()),
        (_, "/v1/estado") => (405, erro_json("use GET")),
        ("POST", "/v1/evento") => receber_evento(req, ctx),
        (_, "/v1/evento") => (405, erro_json("use POST")),
        ("POST", "/v1/comando") => receber_comando(req, ctx),
        (_, "/v1/comando") => (405, erro_json("use POST")),
        (_, rota) if ctx.debug && rota.starts_with("/v1/debug/") => rotear_debug(req, ctx, rota),
        _ => (404, erro_json("rota desconhecida")),
    }
}

/// `Content-Type: application/json`, com ou sem parâmetros (`; charset=…`).
fn eh_json(req: &Requisicao) -> bool {
    req.cabecalho("content-type")
        .and_then(|valor| valor.split(';').next())
        .is_some_and(|tipo| tipo.trim().eq_ignore_ascii_case("application/json"))
}

/// Milissegundos desde 1970 no relógio do host (o do container é o mesmo).
pub fn agora_desde_1970_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// O evento como o `/v1/debug/eventos` mostra: só o que passou na
/// validação, a hora de chegada e os nomes dos campos descartados.
fn registro_de_debug(lido: &Lido, recebido_ms: u64) -> Value {
    let mut registro = serde_json::to_value(&lido.evento).unwrap_or_else(|_| json!({}));
    if let Some(mapa) = registro.as_object_mut() {
        mapa.insert("recebido_ms".into(), json!(recebido_ms));
        if !lido.descartados.is_empty() {
            mapa.insert("descartados".into(), json!(lido.descartados));
        }
    }
    registro
}

/// `POST /v1/evento`: valida (decisão 0019) e entrega ao laço principal.
fn receber_evento(req: &Requisicao, ctx: &Contexto) -> (u16, String) {
    if !eh_json(req) {
        ctx.comp.evento_recusado();
        return (415, erro_json("Content-Type precisa ser application/json"));
    }
    let lido = match evento::ler(&req.corpo) {
        Ok(lido) => lido,
        Err(motivo) => {
            ctx.comp.evento_recusado();
            return (400, erro_json(&motivo.to_string()));
        }
    };
    let Some(canal) = &ctx.comandos else {
        ctx.comp.evento_recusado();
        return (503, erro_json("laço principal indisponível"));
    };
    let chegada = Instant::now();
    let recebido_ms = agora_desde_1970_ms();
    let registro = registro_de_debug(&lido, recebido_ms);
    let recebido = Recebido {
        evento: lido.evento,
        recebido_ms,
        chegada,
    };
    match canal.try_send(Comando::Evento(Box::new(recebido))) {
        Ok(()) => {
            ctx.comp.evento_aceito(|| registro);
            (204, String::new())
        }
        Err(_) => {
            ctx.comp.evento_recusado();
            (503, erro_json("laço principal ocupado"))
        }
    }
}

/// Corpo do `/v1/comando`, o mesmo para todos os comandos.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PedidoComando {
    cmd: String,
    #[serde(default)]
    arg: Option<Value>,
}

/// `arg` do `aprovar_skin`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PedidoAprovacao {
    id: String,
    sha256: String,
}

/// `POST /v1/comando`: as reações do M3 (`tocar <reação>`, `esconder`,
/// `mostrar`, ainda sem persistir) e as aprovações de personagem do M2
/// (`aprovar_skin`, `revogar_skin`), com as mesmas checagens de `Host`,
/// `X-Pet` e `Content-Type`.
fn receber_comando(req: &Requisicao, ctx: &Contexto) -> (u16, String) {
    if !eh_json(req) {
        return (415, erro_json("Content-Type precisa ser application/json"));
    }
    let objeto = req.corpo.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{');
    let pedido: PedidoComando = match serde_json::from_slice(&req.corpo) {
        Ok(pedido) if objeto => pedido,
        // O erro do serde pode citar o corpo: nunca é repassado.
        _ => {
            return (
                400,
                erro_json(r#"o corpo precisa ser {"cmd": "…", "arg": …} sem outros campos"#),
            );
        }
    };
    let comando = match (pedido.cmd.as_str(), pedido.arg) {
        ("tocar", Some(Value::String(reacao))) if evento::eh_enum(&reacao) => {
            Comando::Tocar(reacao)
        }
        ("tocar", _) => {
            return (
                400,
                erro_json("tocar precisa de arg com o nome da reação ([a-z_], até 40)"),
            );
        }
        ("esconder", None) => Comando::Esconder,
        ("mostrar", None) => Comando::Mostrar,
        ("esconder" | "mostrar", Some(_)) => {
            return (400, erro_json("esconder e mostrar não levam arg"));
        }
        ("aprovar_skin", Some(arg)) => return aprovar_skin(arg, ctx),
        ("revogar_skin", Some(arg)) => return revogar_skin(&arg, ctx),
        ("aprovar_skin" | "revogar_skin", None) => {
            return (400, erro_json("falta o arg"));
        }
        _ => {
            return (
                400,
                erro_json(
                    "cmd desconhecido: use tocar, esconder, mostrar, aprovar_skin ou revogar_skin",
                ),
            );
        }
    };
    let Some(canal) = &ctx.comandos else {
        return (503, erro_json("laço principal indisponível"));
    };
    match canal.try_send(comando) {
        Ok(()) => (204, String::new()),
        Err(_) => (503, erro_json("laço principal ocupado")),
    }
}

/// `aprovar_skin {id, sha256}` (decisão 0026): uma aprovação por vez.
fn aprovar_skin(arg: Value, ctx: &Contexto) -> (u16, String) {
    let Ok(PedidoAprovacao { id, sha256 }) = serde_json::from_value(arg) else {
        return (
            400,
            erro_json(r#"aprovar_skin leva arg {"id": "…", "sha256": "…"}"#),
        );
    };
    let _vez = ctx.aprovando.lock().unwrap_or_else(|e| e.into_inner());
    match aprovacao::aprovar(&id, &sha256, &ctx.onde.busca, &ctx.onde.estado) {
        Ok(r) => {
            info!("skin «{}» aprovada (sha {})", r.id, &r.sha256[..12]);
            let resposta = json!({
                "id": r.id,
                "sha256": r.sha256,
                "aprovada_em_ms": r.aprovada_em_ms,
                "snapshot": aprovacao::pasta_da_skin(&ctx.onde.estado, &r.id),
            });
            (200, escolher_de_novo(resposta, ctx).to_string())
        }
        Err(e) => (e.status(), erro_json(&e.mensagem())),
    }
}

/// `revogar_skin "<id>"` ou `{"id": …}` (decisão 0026).
fn revogar_skin(arg: &Value, ctx: &Contexto) -> (u16, String) {
    let id = match arg {
        Value::String(id) => id.clone(),
        outro => match outro.get("id").and_then(Value::as_str) {
            Some(id) if outro.as_object().is_some_and(|o| o.len() == 1) => id.to_owned(),
            _ => {
                return (
                    400,
                    erro_json(r#"revogar_skin leva arg "id" ou {"id": "…"}"#),
                );
            }
        },
    };
    let _vez = ctx.aprovando.lock().unwrap_or_else(|e| e.into_inner());
    match aprovacao::revogar(&id, &ctx.onde.estado) {
        Ok(revogada) => {
            info!(
                "skin «{id}»: aprovação {}",
                if revogada { "revogada" } else { "não existia" }
            );
            let resposta = json!({"id": id, "revogada": revogada});
            (200, escolher_de_novo(resposta, ctx).to_string())
        }
        Err(e) => (e.status(), erro_json(&e.mensagem())),
    }
}

/// A aprovação já está gravada; o laço escolhe o personagem de novo e a
/// resposta só sai depois (com prazo), para quem pediu ver o resultado.
fn escolher_de_novo(mut resposta: Value, ctx: &Contexto) -> Value {
    if let Some(canal) = &ctx.comandos {
        let (feito, espera) = mpsc::sync_channel(1);
        let aplicado = canal.send(Comando::RecarregarPersonagem(feito)).is_ok()
            && espera.recv_timeout(ESPERA_QUADRO).is_ok();
        if !aplicado {
            aviso!("o laço principal não confirmou a troca de personagem");
        }
        resposta["aplicado"] = json!(aplicado);
        resposta["personagem"] = ctx.comp.estado_json()["skin"].clone();
    }
    resposta
}

/// Rotas de debug (só com `PET_DEBUG=1`).
fn rotear_debug(req: &Requisicao, ctx: &Contexto, rota: &str) -> (u16, String) {
    let esperado = match rota {
        "/v1/debug/quadro" | "/v1/debug/eventos" => "GET",
        "/v1/debug/esconder" | "/v1/debug/mostrar" | "/v1/debug/estresse" => "POST",
        _ => return (404, erro_json("rota desconhecida")),
    };
    if req.metodo != esperado {
        return (405, erro_json(&format!("use {esperado}")));
    }
    if rota == "/v1/debug/eventos" {
        return (200, ctx.comp.eventos_debug_json().to_string());
    }
    let Some(canal) = &ctx.comandos else {
        return (503, erro_json("laço principal indisponível"));
    };
    let comando = match rota {
        "/v1/debug/quadro" => return pedir_quadro(canal),
        "/v1/debug/esconder" => Comando::Esconder,
        "/v1/debug/mostrar" => Comando::Mostrar,
        _ => match parametros_estresse(&req.corpo) {
            Ok((fps, segundos)) => Comando::Estresse { fps, segundos },
            Err(motivo) => return (400, erro_json(&motivo)),
        },
    };
    match canal.try_send(comando) {
        Ok(()) => (204, String::new()),
        Err(_) => (503, erro_json("laço principal ocupado")),
    }
}

/// `{"fps":30,"segundos":15}`; corpo vazio usa esses padrões.
fn parametros_estresse(corpo: &[u8]) -> Result<(u32, u32), String> {
    if corpo.iter().all(u8::is_ascii_whitespace) {
        return Ok((30, 15));
    }
    let valor: serde_json::Value =
        serde_json::from_slice(corpo).map_err(|_| "corpo precisa ser JSON".to_owned())?;
    let campo = |nome: &str, padrao: u64, max: u64| -> Result<u32, String> {
        match valor.get(nome) {
            None => Ok(padrao as u32),
            Some(v) => v
                .as_u64()
                .filter(|n| (1..=max).contains(n))
                .map(|n| n as u32)
                .ok_or_else(|| format!("{nome} precisa ser inteiro entre 1 e {max}")),
        }
    };
    Ok((campo("fps", 30, 60)?, campo("segundos", 15, 120)?))
}

fn pedir_quadro(canal: &SyncSender<Comando>) -> (u16, String) {
    let (resposta, recebe) = mpsc::sync_channel(1);
    if canal.try_send(Comando::Quadro(resposta)).is_err() {
        return (503, erro_json("laço principal ocupado"));
    }
    match recebe.recv_timeout(ESPERA_QUADRO) {
        Ok(Some(quadro)) => match quadro_json(&quadro) {
            Ok(corpo) => (200, corpo),
            Err(motivo) => (500, erro_json(&motivo)),
        },
        Ok(None) => (409, erro_json("o pet não está na tela")),
        Err(_) => (503, erro_json("o laço principal não respondeu")),
    }
}

fn quadro_json(q: &QuadroEsperado) -> Result<String, String> {
    let png = codificar_png(q.area.w as u32, q.area.h as u32, &q.rgba)?;
    Ok(json!({
        "monitor": q.monitor,
        "x": q.area.x,
        "y": q.area.y,
        "w": q.area.w,
        "h": q.area.h,
        "d": q.d,
        "grade": {"x": q.grade.0, "y": q.grade.1},
        "seq": q.seq,
        "idade_ms": q.idade_ms,
        "png_base64": base64(&png),
    })
    .to_string())
}

/// Base64 padrão (RFC 4648, com `=`).
pub fn base64(dados: &[u8]) -> String {
    const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut saida = String::with_capacity(dados.len().div_ceil(3) * 4);
    for bloco in dados.chunks(3) {
        let b = [
            bloco[0],
            bloco.get(1).copied().unwrap_or(0),
            bloco.get(2).copied().unwrap_or(0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= bloco.len() {
                saida.push(ALFABETO[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                saida.push('=');
            }
        }
    }
    saida
}

/// Aceita conexões para sempre, uma thread curta por conexão (no máximo
/// `MAX_CONEXOES` ao mesmo tempo; acima disso, 503 na hora).
pub fn servir(ouvinte: TcpListener, ctx: Arc<Contexto>) {
    let ativas = Arc::new(AtomicUsize::new(0));
    for conexao in ouvinte.incoming() {
        let Ok(mut fluxo) = conexao else {
            continue;
        };
        if ativas.load(Ordering::Relaxed) >= MAX_CONEXOES {
            let _ = fluxo.set_write_timeout(Some(TEMPO_LIMITE));
            let _ = escrever_resposta(&mut fluxo, 503, &erro_json("ocupado"));
            continue;
        }
        let vaga = Vaga::ocupar(Arc::clone(&ativas));
        let ctx = Arc::clone(&ctx);
        let criada = thread::Builder::new()
            .name("conexao".into())
            .spawn(move || {
                let _vaga = vaga;
                atender(fluxo, &ctx);
            });
        if let Err(e) = criada {
            aviso!("não consegui criar thread para a conexão: {e}");
        }
    }
}

/// Conta uma conexão ativa enquanto existir.
struct Vaga(Arc<AtomicUsize>);

impl Vaga {
    fn ocupar(ativas: Arc<AtomicUsize>) -> Self {
        ativas.fetch_add(1, Ordering::Relaxed);
        Vaga(ativas)
    }
}

impl Drop for Vaga {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

fn atender(mut fluxo: TcpStream, ctx: &Contexto) {
    let _ = fluxo.set_read_timeout(Some(TEMPO_LIMITE));
    let _ = fluxo.set_write_timeout(Some(TEMPO_LIMITE));
    let (status, corpo) = match ler_requisicao(&mut fluxo) {
        Ok(req) => {
            let resposta = rotear(&req, ctx);
            // Só a rota (sem a query, cortada): nada do corpo vai para log.
            let rota: String = req
                .caminho
                .split('?')
                .next()
                .unwrap_or_default()
                .chars()
                .take(64)
                .collect();
            let metodo: String = req.metodo.chars().take(16).collect();
            depurar!("{metodo} {rota} -> {}", resposta.0);
            resposta
        }
        Err(erro) => match erro.status() {
            Some(status) => (status, erro_json("requisição inválida")),
            None => return,
        },
    };
    let _ = escrever_resposta(&mut fluxo, status, &corpo);
    let _ = fluxo.flush();
}

fn achar(palheiro: &[u8], agulha: &[u8]) -> Option<usize> {
    palheiro
        .windows(agulha.len())
        .position(|janela| janela == agulha)
}

#[cfg(test)]
mod testes {
    use std::io::Cursor;
    use std::net::SocketAddr;

    use pet_core::config::ConfigEfetiva;

    use super::*;

    fn ler(bruto: &str) -> Result<Requisicao, ErroHttp> {
        ler_requisicao(&mut Cursor::new(bruto.as_bytes().to_vec()))
    }

    /// Como o daemon monta: o mesmo `debug` no contexto e no estado.
    fn contexto_com(debug: bool) -> Contexto {
        let comp = Arc::new(Compartilhado::novo(
            ConfigEfetiva::carregar(None, |_| None),
            debug,
        ));
        comp.bater();
        Contexto {
            comp,
            porta_publica: 27380,
            debug,
            comandos: None,
            onde: crate::personagem::Onde {
                busca: Vec::new(),
                estado: std::path::PathBuf::from("/nao/existe"),
                debug: false,
                debug_personagem: false,
            },
            aprovando: Mutex::new(()),
        }
    }

    fn contexto() -> Contexto {
        contexto_com(false)
    }

    /// Contexto de debug com o canal ligado a um receptor do teste.
    fn contexto_debug() -> (
        Contexto,
        smithay_client_toolkit::reexports::calloop::channel::Channel<Comando>,
    ) {
        let (canal, recebe) = smithay_client_toolkit::reexports::calloop::channel::sync_channel(16);
        let ctx = Contexto {
            comandos: Some(canal),
            ..contexto_com(true)
        };
        (ctx, recebe)
    }

    /// POST com `Content-Type` (`None`: sem o cabeçalho).
    fn post_tipo(caminho: &str, tipo: Option<&str>, corpo: &str) -> Requisicao {
        let tipo = tipo.map_or_else(String::new, |t| format!("Content-Type: {t}\r\n"));
        ler(&format!(
            "POST {caminho} HTTP/1.1\r\nHost: 127.0.0.1:27380\r\nX-Pet: 1\r\n{tipo}Content-Length: {}\r\n\r\n{corpo}",
            corpo.len()
        ))
        .unwrap()
    }

    fn post_json(caminho: &str, corpo: &str) -> Requisicao {
        post_tipo(caminho, Some("application/json"), corpo)
    }

    /// Tudo o que chegou no canal, na ordem.
    fn drenar(
        recebe: smithay_client_toolkit::reexports::calloop::channel::Channel<Comando>,
    ) -> Vec<Comando> {
        use smithay_client_toolkit::reexports::calloop::{EventLoop, channel::Event};
        let mut laco = EventLoop::<Vec<Comando>>::try_new().unwrap();
        laco.handle()
            .insert_source(recebe, |evento, _, lista: &mut Vec<Comando>| {
                if let Event::Msg(c) = evento {
                    lista.push(c);
                }
            })
            .unwrap();
        let mut recebidos = Vec::new();
        laco.dispatch(Some(Duration::from_millis(50)), &mut recebidos)
            .unwrap();
        recebidos
    }

    #[test]
    fn evento_valido_vai_para_o_laco() {
        let (ctx, recebe) = contexto_debug();
        let corpo = r#"{"v":1,"e":"Stop","sid":"s-1","ts":1790020208123,"prompt":"SEGREDO-1"}"#;
        assert_eq!(rotear(&post_json("/v1/evento", corpo), &ctx).0, 204);
        // Com parâmetros no Content-Type também vale.
        let req = post_tipo(
            "/v1/evento",
            Some("Application/JSON; charset=utf-8"),
            r#"{"v":1,"e":"SessionEnd"}"#,
        );
        assert_eq!(rotear(&req, &ctx).0, 204);
        let estado = ctx.comp.estado_json();
        assert_eq!(estado["eventos"]["aceitos"], 2);
        let debug = ctx.comp.eventos_debug_json().to_string();
        assert!(debug.contains("\"sid\":\"s-1\""), "{debug}");
        assert!(!debug.contains("SEGREDO"), "{debug}");
        drop(ctx);
        let recebidos = drenar(recebe);
        assert_eq!(recebidos.len(), 2);
        let Comando::Evento(r) = &recebidos[0] else {
            panic!("esperava evento: {:?}", recebidos[0]);
        };
        assert_eq!(r.evento.e, "Stop");
        assert_eq!(r.evento.sid.as_deref(), Some("s-1"));
        assert_eq!(r.evento.ts, Some(1_790_020_208_123));
        assert!(r.recebido_ms > 1_700_000_000_000, "relógio de parede");
    }

    #[test]
    fn evento_invalido_recusado() {
        let (ctx, recebe) = contexto_debug();
        let valido = r#"{"v":1,"e":"Stop"}"#;
        for (req, status) in [
            (post_tipo("/v1/evento", Some("text/plain"), valido), 415),
            (post_tipo("/v1/evento", None, valido), 415),
            (post_json("/v1/evento", "{lixo"), 400),
            (post_json("/v1/evento", r#"{"e":"Stop"}"#), 400),
            (post_json("/v1/evento", r#"{"v":1}"#), 400),
            (post_json("/v1/evento", r#"{"v":2,"e":"Stop"}"#), 400),
            (post_json("/v1/evento", r#"[1,"Stop"]"#), 400),
        ] {
            let (obtido, corpo) = rotear(&req, &ctx);
            assert_eq!(obtido, status, "{corpo}");
        }
        let ok = "Host: 127.0.0.1:27380\r\nX-Pet: 1\r\n";
        assert_eq!(rotear(&get("/v1/evento", ok), &ctx).0, 405);
        assert_eq!(ctx.comp.estado_json()["eventos"]["recusados"], 7);
        drop(ctx);
        assert!(drenar(recebe).is_empty());
    }

    #[test]
    fn evento_sem_laco_da_503() {
        let ctx = contexto();
        assert_eq!(
            rotear(&post_json("/v1/evento", r#"{"v":1,"e":"Stop"}"#), &ctx).0,
            503
        );
    }

    #[test]
    fn comandos() {
        let (ctx, recebe) = contexto_debug();
        for (corpo, status) in [
            (r#"{"cmd":"tocar","arg":"nod"}"#, 204),
            (r#"{"cmd":"esconder"}"#, 204),
            (r#"{"cmd":"mostrar","arg":null}"#, 204),
            (r#"{"cmd":"tocar"}"#, 400),
            (r#"{"cmd":"tocar","arg":"Nod!"}"#, 400),
            (r#"{"cmd":"tocar","arg":3}"#, 400),
            (r#"{"cmd":"esconder","arg":"30m"}"#, 400),
            (r#"{"cmd":"soneca"}"#, 400),
            (r#"{"cmd":"tocar","arg":"nod","x":1}"#, 400),
            (r#"["tocar","nod"]"#, 400),
            ("lixo", 400),
        ] {
            let (obtido, resposta) = rotear(&post_json("/v1/comando", corpo), &ctx);
            assert_eq!(obtido, status, "{corpo}: {resposta}");
        }
        let req = post_tipo("/v1/comando", Some("text/plain"), r#"{"cmd":"mostrar"}"#);
        assert_eq!(rotear(&req, &ctx).0, 415);
        drop(ctx);
        let recebidos: Vec<String> = drenar(recebe).iter().map(|c| format!("{c:?}")).collect();
        assert_eq!(recebidos, vec!["Tocar(\"nod\")", "Esconder", "Mostrar"]);
    }

    #[test]
    fn um_comando_so_para_reacoes_e_aprovacoes() {
        // As reações do M3 e as aprovações do M2 dividem o `/v1/comando`,
        // com o mesmo formato e as mesmas checagens (decisão 0030).
        let (canal, recebe) = smithay_client_toolkit::reexports::calloop::channel::sync_channel(16);
        let laco = laco_que_confirma(recebe);
        let ctx = Contexto {
            comandos: Some(canal),
            ..contexto()
        };
        let pedido = |cabecalhos: &str, corpo: &str| {
            ler(&format!(
                "POST /v1/comando HTTP/1.1\r\n{cabecalhos}Content-Length: {}\r\n\r\n{corpo}",
                corpo.len()
            ))
            .unwrap()
        };
        for corpo in [
            r#"{"cmd":"tocar","arg":"nod"}"#,
            r#"{"cmd":"revogar_skin","arg":"zeca"}"#,
        ] {
            let json = "Content-Type: application/json\r\n";
            for (cabecalhos, status) in [
                (format!("Host: evil.com:27380\r\nX-Pet: 1\r\n{json}"), 403),
                (format!("Host: 127.0.0.1:27380\r\n{json}"), 403),
                ("Host: 127.0.0.1:27380\r\nX-Pet: 1\r\n".to_owned(), 415),
            ] {
                let (obtido, resposta) = rotear(&pedido(&cabecalhos, corpo), &ctx);
                assert_eq!(obtido, status, "{corpo} com {cabecalhos:?}: {resposta}");
            }
            let a_mais = corpo.replacen('{', r#"{"x":1,"#, 1);
            assert_eq!(rotear(&post_json("/v1/comando", &a_mais), &ctx).0, 400);
        }
        // Aceitos: a reação vai sem esperar (204); a revogação espera o laço
        // escolher o personagem de novo (200, com o resultado).
        assert_eq!(
            rotear(
                &post_json("/v1/comando", r#"{"cmd":"tocar","arg":"nod"}"#),
                &ctx
            )
            .0,
            204
        );
        let (status, resposta) = rotear(
            &post_json("/v1/comando", r#"{"cmd":"revogar_skin","arg":"zeca"}"#),
            &ctx,
        );
        assert_eq!(status, 200, "{resposta}");
        assert!(resposta.contains(r#""revogada":false"#), "{resposta}");
        assert!(resposta.contains(r#""aplicado":true"#), "{resposta}");
        let (status, resposta) = rotear(&post_json("/v1/comando", r#"{"cmd":"voar"}"#), &ctx);
        assert_eq!(status, 400);
        for cmd in [
            "tocar",
            "esconder",
            "mostrar",
            "aprovar_skin",
            "revogar_skin",
        ] {
            assert!(resposta.contains(cmd), "{resposta}");
        }
        drop(ctx);
        assert_eq!(
            laco.join().unwrap(),
            1,
            "só a revogação troca de personagem"
        );
    }

    #[test]
    fn debug_eventos_so_em_debug() {
        let ok = "Host: 127.0.0.1:27380\r\nX-Pet: 1\r\n";
        assert_eq!(rotear(&get("/v1/debug/eventos", ok), &contexto()).0, 404);
        let (ctx, _recebe) = contexto_debug();
        let (status, corpo) = rotear(&get("/v1/debug/eventos", ok), &ctx);
        assert_eq!((status, corpo.as_str()), (200, r#"{"eventos":[]}"#));
        assert_eq!(rotear(&post_json("/v1/debug/eventos", "{}"), &ctx).0, 405);
    }

    fn post(caminho: &str, corpo: &str) -> Requisicao {
        ler(&format!(
            "POST {caminho} HTTP/1.1\r\nHost: 127.0.0.1:27380\r\nX-Pet: 1\r\nContent-Length: {}\r\n\r\n{corpo}",
            corpo.len()
        ))
        .unwrap()
    }

    #[test]
    fn comando_aprova_e_revoga_e_avisa_o_laco() {
        let a = crate::aprovacao::testes::Ambiente::novo("ingress");
        let (canal, recebe) = smithay_client_toolkit::reexports::calloop::channel::sync_channel(4);
        let laco = laco_que_confirma(recebe);
        let ctx = Contexto {
            comandos: Some(canal),
            onde: crate::personagem::Onde {
                busca: a.busca(),
                estado: a.estado.clone(),
                debug: false,
                debug_personagem: false,
            },
            ..contexto()
        };
        let sha = a.sha();
        let corpo = format!(r#"{{"cmd":"aprovar_skin","arg":{{"id":"zeca","sha256":"{sha}"}}}}"#);
        let (status, resposta) = rotear(&post_json("/v1/comando", &corpo), &ctx);
        assert_eq!(status, 200, "{resposta}");
        assert!(resposta.contains(&sha));
        assert!(resposta.contains("\"aplicado\":true"), "{resposta}");
        let outro = format!(
            r#"{{"cmd":"aprovar_skin","arg":{{"id":"zeca","sha256":"{}"}}}}"#,
            "1".repeat(64)
        );
        assert_eq!(rotear(&post_json("/v1/comando", &outro), &ctx).0, 409);
        let revoga = r#"{"cmd":"revogar_skin","arg":"zeca"}"#;
        let (status, resposta) = rotear(&post_json("/v1/comando", revoga), &ctx);
        assert_eq!(
            (status, resposta.contains("\"revogada\":true")),
            (200, true)
        );
        let objeto = r#"{"cmd":"revogar_skin","arg":{"id":"zeca"}}"#;
        assert!(
            rotear(&post_json("/v1/comando", objeto), &ctx)
                .1
                .contains("\"revogada\":false")
        );
        drop(ctx);
        assert_eq!(
            laco.join().unwrap(),
            3,
            "aprovou, revogou e revogou de novo: 3 trocas"
        );
    }

    /// Um "laço principal" numa thread: confirma cada troca de personagem e
    /// devolve quantas foram quando o canal fecha.
    fn laco_que_confirma(
        recebe: smithay_client_toolkit::reexports::calloop::channel::Channel<Comando>,
    ) -> thread::JoinHandle<usize> {
        use smithay_client_toolkit::reexports::calloop::EventLoop;
        use smithay_client_toolkit::reexports::calloop::channel::Event;
        thread::spawn(move || {
            let mut laco = EventLoop::<(usize, bool)>::try_new().unwrap();
            laco.handle()
                .insert_source(
                    recebe,
                    |evento, _, estado: &mut (usize, bool)| match evento {
                        Event::Msg(Comando::RecarregarPersonagem(feito)) => {
                            estado.0 += 1;
                            let _ = feito.try_send(());
                        }
                        Event::Msg(_) => {}
                        Event::Closed => estado.1 = true,
                    },
                )
                .unwrap();
            let mut estado = (0, false);
            while !estado.1 {
                laco.dispatch(Some(Duration::from_millis(20)), &mut estado)
                    .unwrap();
            }
            estado.0
        })
    }

    #[test]
    fn comando_mal_formado() {
        let ctx = contexto();
        for (corpo, esperado) in [
            (r#"{"cmd":"aprovar_skin"}"#, 400),
            (r#"{"cmd":"aprovar_skin","arg":{"id":"zeca"}}"#, 400),
            (
                r#"{"cmd":"aprovar_skin","arg":{"id":"zeca","sha256":"x","y":1}}"#,
                400,
            ),
            (r#"{"cmd":"revogar_skin","arg":3}"#, 400),
            (r#"{"cmd":"soneca"}"#, 400),
            (r#"{"cmd":"revogar_skin","arg":"zeca","x":1}"#, 400),
            (r#"["cmd"]"#, 400),
            (
                r#"{"cmd":"aprovar_skin","arg":{"id":"_teste","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
                403,
            ),
        ] {
            let (obtido, resposta) = rotear(&post_json("/v1/comando", corpo), &ctx);
            assert_eq!(obtido, esperado, "{corpo} → {resposta}");
        }
        let sem_tipo = post("/v1/comando", r#"{"cmd":"revogar_skin","arg":"zeca"}"#);
        assert_eq!(rotear(&sem_tipo, &ctx).0, 415);
        let ok = "Host: 127.0.0.1:27380\r\nX-Pet: 1\r\n";
        assert_eq!(rotear(&get("/v1/comando", ok), &ctx).0, 405);
    }

    #[test]
    fn rotas_de_debug_nao_existem_sem_debug() {
        let ctx = contexto();
        let ok = "Host: 127.0.0.1:27380\r\nX-Pet: 1\r\n";
        assert_eq!(rotear(&get("/v1/debug/quadro", ok), &ctx).0, 404);
        assert_eq!(rotear(&post("/v1/debug/esconder", ""), &ctx).0, 404);
    }

    #[test]
    fn rotas_de_debug_mandam_comandos() {
        let (ctx, recebe) = contexto_debug();
        assert_eq!(rotear(&post("/v1/debug/esconder", ""), &ctx).0, 204);
        assert_eq!(rotear(&post("/v1/debug/mostrar", ""), &ctx).0, 204);
        assert_eq!(
            rotear(
                &post("/v1/debug/estresse", r#"{"fps":20,"segundos":3}"#),
                &ctx
            )
            .0,
            204
        );
        assert_eq!(
            rotear(&post("/v1/debug/estresse", "{\"fps\":0}"), &ctx).0,
            400
        );
        assert_eq!(rotear(&post("/v1/debug/estresse", "lixo"), &ctx).0, 400);
        let ok = "Host: 127.0.0.1:27380\r\nX-Pet: 1\r\n";
        assert_eq!(rotear(&get("/v1/debug/esconder", ok), &ctx).0, 405);
        assert_eq!(rotear(&get("/v1/debug/nada", ok), &ctx).0, 404);
        // As mesmas checagens de Host e X-Pet valem para o debug.
        assert_eq!(
            rotear(
                &get("/v1/debug/quadro", "Host: evil.com:27380\r\nX-Pet: 1\r\n"),
                &ctx
            )
            .0,
            403
        );
        drop(ctx);
        let mut recebidos = Vec::new();
        let mut laco =
            smithay_client_toolkit::reexports::calloop::EventLoop::<Vec<String>>::try_new()
                .unwrap();
        laco.handle()
            .insert_source(recebe, |evento, _, lista: &mut Vec<String>| {
                if let smithay_client_toolkit::reexports::calloop::channel::Event::Msg(c) = evento {
                    lista.push(format!("{c:?}"));
                }
            })
            .unwrap();
        laco.dispatch(Some(Duration::from_millis(50)), &mut recebidos)
            .unwrap();
        assert_eq!(
            recebidos,
            vec![
                "Esconder".to_owned(),
                "Mostrar".to_owned(),
                "Estresse { fps: 20, segundos: 3 }".to_owned()
            ]
        );
    }

    #[test]
    fn parametros_do_estresse() {
        assert_eq!(parametros_estresse(b""), Ok((30, 15)));
        assert_eq!(parametros_estresse(br#"{"segundos":5}"#), Ok((30, 5)));
        assert!(parametros_estresse(br#"{"fps":61}"#).is_err());
        assert!(parametros_estresse(br#"{"segundos":-1}"#).is_err());
    }

    #[test]
    fn base64_rfc4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0x00]), "//4A");
    }

    #[test]
    fn quadro_vira_json_com_png() {
        let q = QuadroEsperado {
            monitor: "eDP-1".into(),
            area: pet_core::geometria::Ret::novo(10, 20, 2, 1),
            grade: (10, 20),
            d: 5,
            seq: 3,
            idade_ms: 1500,
            rgba: vec![255, 0, 0, 255, 0, 0, 0, 0],
        };
        let v: serde_json::Value = serde_json::from_str(&quadro_json(&q).unwrap()).unwrap();
        assert_eq!(v["monitor"], "eDP-1");
        assert_eq!(v["w"], 2);
        assert_eq!(v["grade"]["y"], 20);
        assert!(v["png_base64"].as_str().unwrap().starts_with("iVBORw0KGgo"));
    }

    fn get(caminho: &str, extra: &str) -> Requisicao {
        ler(&format!("GET {caminho} HTTP/1.1\r\n{extra}\r\n")).unwrap()
    }

    #[test]
    fn le_get_simples() {
        let r = ler("GET /saude HTTP/1.1\r\nHost: 127.0.0.1:27380\r\n\r\n").unwrap();
        assert_eq!(r.metodo, "GET");
        assert_eq!(r.caminho, "/saude");
        assert_eq!(r.cabecalho("host"), Some("127.0.0.1:27380"));
        assert!(r.corpo.is_empty());
    }

    #[test]
    fn le_post_com_corpo() {
        let r = ler("POST /v1/evento HTTP/1.1\r\nContent-Length: 7\r\n\r\n{\"v\":1}").unwrap();
        assert_eq!(r.corpo, b"{\"v\":1}");
    }

    #[test]
    fn post_sem_tamanho_e_chunked_dao_411() {
        assert_eq!(
            ler("POST /v1/evento HTTP/1.1\r\n\r\n"),
            Err(ErroHttp::SemTamanho)
        );
        assert_eq!(
            ler("POST /v1/evento HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n"),
            Err(ErroHttp::SemTamanho)
        );
    }

    #[test]
    fn corpo_grande_da_413() {
        let bruto = format!(
            "POST /v1/evento HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            LIMITE_CORPO + 1
        );
        assert_eq!(ler(&bruto), Err(ErroHttp::CorpoGrande));
    }

    #[test]
    fn cabecalhos_grandes_dao_431() {
        let bruto = format!(
            "GET / HTTP/1.1\r\nX: {}\r\n\r\n",
            "a".repeat(LIMITE_CABECALHOS)
        );
        assert_eq!(ler(&bruto), Err(ErroHttp::CabecalhosGrandes));
    }

    #[test]
    fn lixo_da_400_e_conexao_vazia_nao_responde() {
        assert_eq!(ler("oi\r\n\r\n"), Err(ErroHttp::Malformada));
        assert_eq!(
            ler("GET /x HTTP/1.1\r\nsem-dois-pontos\r\n\r\n"),
            Err(ErroHttp::Malformada)
        );
        assert_eq!(ler(""), Err(ErroHttp::Conexao));
    }

    #[test]
    fn host_de_loopback_na_porta_publica() {
        assert!(host_valido(Some("127.0.0.1:27380"), 27380));
        assert!(host_valido(Some("localhost:27380"), 27380));
        assert!(!host_valido(Some("evil.com:27380"), 27380));
        assert!(!host_valido(Some("127.0.0.1:80"), 27380));
        assert!(!host_valido(Some("127.0.0.1"), 27380));
        assert!(!host_valido(None, 27380));
    }

    #[test]
    fn saude_responde_sem_cabecalhos_especiais() {
        let (status, corpo) = rotear(&get("/saude", ""), &contexto());
        assert_eq!(status, 200);
        assert!(corpo.contains("\"ok\":true"));
    }

    #[test]
    fn estado_exige_host_e_x_pet() {
        let ctx = contexto();
        let ok = "Host: 127.0.0.1:27380\r\nX-Pet: 1\r\n";
        assert_eq!(rotear(&get("/v1/estado", ok), &ctx).0, 200);
        assert_eq!(
            rotear(
                &get("/v1/estado", "Host: evil.com:27380\r\nX-Pet: 1\r\n"),
                &ctx
            )
            .0,
            403
        );
        assert_eq!(
            rotear(&get("/v1/estado", "Host: 127.0.0.1:27380\r\n"), &ctx).0,
            403
        );
        assert_eq!(rotear(&get("/v1/nada", ok), &ctx).0, 404);
        assert_eq!(rotear(&get("/nada", ""), &ctx).0, 404);
    }

    #[test]
    fn resposta_204_sem_corpo() {
        let mut saida = Vec::new();
        escrever_resposta(&mut saida, 204, "").unwrap();
        let texto = String::from_utf8(saida).unwrap();
        assert!(texto.starts_with("HTTP/1.1 204 No Content\r\n"));
        assert!(!texto.contains("Content-Length"));
    }

    #[test]
    fn servidor_de_verdade_responde_saude() {
        let ouvinte = TcpListener::bind("127.0.0.1:0").unwrap();
        let endereco: SocketAddr = ouvinte.local_addr().unwrap();
        let ctx = Arc::new(contexto());
        thread::spawn(move || servir(ouvinte, ctx));
        let mut fluxo = TcpStream::connect(endereco).unwrap();
        write!(fluxo, "GET /saude HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut resposta = String::new();
        fluxo.read_to_string(&mut resposta).unwrap();
        assert!(resposta.starts_with("HTTP/1.1 200 OK\r\n"), "{resposta}");
        assert!(resposta.contains("\"tela\":\"aguardando\""));
    }
}
