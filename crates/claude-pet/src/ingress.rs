//! Entrada HTTP dos hooks (decisão 0008).
//!
//! HTTP/1.1 feito à mão, sem dependências: `Connection: close`, timeouts de
//! 1 s, cabeçalhos e corpo limitados a 8 KiB, `Content-Length` obrigatório
//! em POST (chunked recebe 411). O corpo nunca vai para log.
//!
//! As rotas `/v1/*` só aceitam `Host` de loopback na porta pública e o
//! cabeçalho `X-Pet: 1`: isso barra requisições disparadas por navegador
//! (DNS rebinding, CSRF).

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::estado::Compartilhado;

pub const LIMITE_CABECALHOS: usize = 8 * 1024;
pub const LIMITE_CORPO: usize = 8 * 1024;
const MAX_CONEXOES: usize = 16;
const TEMPO_LIMITE: Duration = Duration::from_secs(1);

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
        404 => "Not Found",
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
        _ => (404, erro_json("rota desconhecida")),
    }
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
            depurar!("{} {} -> {}", req.metodo, req.caminho, resposta.0);
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

    fn contexto() -> Contexto {
        let comp = Arc::new(Compartilhado::novo(
            ConfigEfetiva::carregar(None, |_| None),
            false,
        ));
        comp.bater();
        Contexto {
            comp,
            porta_publica: 27380,
        }
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
