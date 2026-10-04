//! A arte nossa do Zeca, como dado (`arte/zeca/`, MIT, no git):
//!
//! - `paleta.toml`: cores do pack que o encaixe usa (olho, clarão), a troca
//!   de paleta do corpo (vazia no visual "Malandro rosa": o bico rosa fica),
//!   a cor de cada letra das grades dos acessórios e a cor do contorno creme;
//! - `acessorios/*.txt`: chapéu-palheta, gravata-borboleta e as variantes do
//!   chapéu voando, desenhados em grades de texto (`.` é transparente; linhas
//!   começando com `;` são comentário). O nome do arquivo é o da variante;
//! - `ancoras.json`: a regra do olho (onde chapéu e gravata ficam em relação
//!   ao olho branco) e as correções por tag e por quadro do pack;
//! - `chapeu_voando.json`: "chapéu voa e volta" (decisão 0024): para as tags
//!   do mergulho e do susto, a lista de quadros com o corpo do pack, a
//!   duração e onde está o chapéu (solto na célula ou assentado);
//! - `zeca.toml`: metadados do `skin.json`, estados, tags compostas, tags no
//!   chão e tags do pack que ficam de fora.
//!
//! Nada aqui é derivado do pack além de coordenadas: os pixels do pack só
//! entram na hora de montar, em `skins-locais/`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

/// Cor RGBA a partir de `#RRGGBB`.
pub fn cor(texto: &str) -> Result<[u8; 4], String> {
    let h = texto
        .strip_prefix('#')
        .filter(|h| h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| format!("cor inválida: «{texto}» (use #RRGGBB)"))?;
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|e| e.to_string());
    Ok([c(0)?, c(2)?, c(4)?, 255])
}

pub fn hex(c: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

// --- paleta.toml -------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaletaToml {
    pack: PackToml,
    #[serde(default)]
    troca: BTreeMap<String, String>,
    letras: BTreeMap<String, String>,
    contorno: ContornoToml,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackToml {
    olho: Vec<String>,
    bico: Vec<String>,
    clarao: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContornoToml {
    cor: String,
}

#[derive(Debug, Clone)]
pub struct Paleta {
    /// Cores do olho branco (a âncora da cabeça).
    pub olho: Vec<[u8; 4]>,
    /// Cores do bico (o encaixe avisa se um acessório cobre o bico).
    pub bico: Vec<[u8; 4]>,
    /// Branco da silhueta dos quadros de clarão; os acessórios viram ele.
    pub clarao: [u8; 4],
    /// Cor do pack → cor nova, aplicada ao corpo inteiro.
    pub troca: Vec<([u8; 4], [u8; 4])>,
    /// Letra das grades → cor.
    pub letras: BTreeMap<char, [u8; 4]>,
    pub contorno: [u8; 4],
}

// --- acessórios ----------------------------------------------------------------

/// Um acessório desenhado: pixels opacos com cor, `None` transparente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    pub largura: i32,
    pub altura: i32,
    pub pixels: Vec<Option<[u8; 4]>>,
}

impl Sprite {
    pub fn em(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        self.pixels[(y * self.largura + x) as usize]
    }

    /// Lê uma grade de texto com as cores da paleta.
    pub fn de_grade(texto: &str, letras: &BTreeMap<char, [u8; 4]>) -> Result<Sprite, String> {
        let linhas: Vec<&str> = texto
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.is_empty() && !l.starts_with(';'))
            .collect();
        let largura = linhas.first().map_or(0, |l| l.chars().count());
        if largura == 0 {
            return Err("grade vazia".into());
        }
        let mut pixels = Vec::with_capacity(largura * linhas.len());
        for (n, linha) in linhas.iter().enumerate() {
            if linha.chars().count() != largura {
                return Err(format!(
                    "a linha {} tem {} colunas; a primeira tem {largura}",
                    n + 1,
                    linha.chars().count()
                ));
            }
            for c in linha.chars() {
                if c == '.' {
                    pixels.push(None);
                } else {
                    let cor = letras
                        .get(&c)
                        .ok_or_else(|| format!("letra «{c}» sem cor em paleta.toml"))?;
                    pixels.push(Some(*cor));
                }
            }
        }
        Ok(Sprite {
            largura: largura as i32,
            altura: linhas.len() as i32,
            pixels,
        })
    }
}

// --- ancoras.json ----------------------------------------------------------------

/// Ajuste de uma peça. Campos ausentes não mudam nada; `oculto` tira a
/// peça; `x`/`y` são absolutos na célula; `dx`/`dy` somam à posição.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ajuste {
    #[serde(default)]
    pub oculto: bool,
    pub variante: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub dx: Option<i32>,
    pub dy: Option<i32>,
    #[serde(default)]
    pub _nota: Option<String>,
}

/// Onde a peça fica em relação ao canto superior esquerdo do olho.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regra {
    pub variante: String,
    pub dx: i32,
    pub dy: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegrasOlho {
    pub chapeu: Regra,
    pub gravata: Regra,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AjustesQuadro {
    pub chapeu: Option<Ajuste>,
    pub gravata: Option<Ajuste>,
    #[serde(default)]
    pub _nota: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AncorasTag {
    /// Vale para todos os quadros da tag.
    pub chapeu: Option<Ajuste>,
    pub gravata: Option<Ajuste>,
    /// Índice do quadro dentro da tag do pack (0, 1, …) → ajustes.
    #[serde(default)]
    pub quadros: BTreeMap<String, AjustesQuadro>,
    #[serde(default)]
    pub _nota: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ancoras {
    #[serde(default)]
    pub _sobre: Option<Value>,
    pub regra: RegrasOlho,
    #[serde(default)]
    pub tags: BTreeMap<String, AncorasTag>,
}

// --- chapeu_voando.json ------------------------------------------------------------

/// O chapéu num quadro do voo: `assentado` usa o encaixe normal daquele
/// corpo (com `dx`/`dy` e `variante` opcionais, para pulinhos e amassados);
/// senão, a `variante` solta na célula, pelo canto superior esquerdo (`x`,
/// `y`) ou pelo centro (`cx`, `cy`). O centro é o jeito de desenhar uma
/// cambalhota: as variantes têm tamanhos diferentes (11x5 de pé, 5x11 de
/// lado), e girar em volta do mesmo centro não faz o chapéu pular.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapeuVoo {
    #[serde(default)]
    pub assentado: bool,
    pub variante: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub cx: Option<i32>,
    pub cy: Option<i32>,
    pub dx: Option<i32>,
    pub dy: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuadroVoo {
    /// Índice do corpo dentro da tag do pack.
    pub corpo: usize,
    pub ms: u32,
    pub chapeu: ChapeuVoo,
    #[serde(default)]
    pub _nota: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Voo {
    /// Sem `base`, o voo troca os quadros da tag do pack de mesmo nome. Com
    /// `base`, vira uma tag nova (o nome da chave) feita com os corpos da tag
    /// `base`, que continua como está: por exemplo o pouso do voo grande, em
    /// que o chapéu do mergulho ainda está no ar.
    #[serde(default)]
    pub base: Option<String>,
    pub quadros: Vec<QuadroVoo>,
    #[serde(default)]
    pub _nota: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Voos {
    #[serde(default)]
    pub _sobre: Option<Value>,
    #[serde(default)]
    pub tags: BTreeMap<String, Voo>,
}

// --- zeca.toml ----------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkinToml {
    pub id: String,
    pub nome: String,
    pub autor: String,
    pub licenca: String,
    pub fonte: String,
    pub escala_padrao: u32,
    pub pe: [i32; 2],
    /// Calculados da pose parada (com chapéu) quando ausentes.
    pub toque: Option<[i32; 4]>,
    pub corpo_px: Option<u32>,
    #[serde(default)]
    pub chao: Vec<String>,
    #[serde(default)]
    pub excluir: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parte {
    pub tag: String,
    #[serde(default = "um")]
    pub vezes: u32,
}

fn um() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Composicao {
    pub nome: String,
    #[serde(default)]
    pub nota: Option<String>,
    pub partes: Vec<Parte>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZecaToml {
    pub skin: SkinToml,
    pub estados: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub composicao: Vec<Composicao>,
}

// --- tudo junto -------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Arte {
    pub paleta: Paleta,
    pub sprites: BTreeMap<String, Sprite>,
    pub ancoras: Ancoras,
    pub voos: Voos,
    pub zeca: ZecaToml,
    /// Avisos do `zeca` já olhados e aceitos (`avisos-aceitos.txt`, um por
    /// linha, `#` comenta): com `--estrito`, só aviso novo reprova.
    pub aceitos: Vec<String>,
}

fn ler(pasta: &Path, nome: &str) -> Result<String, String> {
    fs::read_to_string(pasta.join(nome)).map_err(|e| format!("{}: {e}", pasta.join(nome).display()))
}

impl Arte {
    pub fn carregar(pasta: &Path) -> Result<Arte, String> {
        let p: PaletaToml =
            toml::from_str(&ler(pasta, "paleta.toml")?).map_err(|e| format!("paleta.toml: {e}"))?;
        let mut letras = BTreeMap::new();
        for (letra, valor) in &p.letras {
            let mut chars = letra.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else {
                return Err(format!("paleta.toml: «{letra}» precisa ser uma letra só"));
            };
            if c == '.' {
                return Err("paleta.toml: «.» é sempre transparente".into());
            }
            letras.insert(c, cor(valor)?);
        }
        for (letra, valor) in &p.letras {
            if cor(valor)?[..3] == [0, 0, 0] {
                return Err(format!(
                    "paleta.toml: «{letra}» = #000000; o contorno é a tinta do pack (#1D2427), nunca preto"
                ));
            }
        }
        let paleta = Paleta {
            olho: p
                .pack
                .olho
                .iter()
                .map(|c| cor(c))
                .collect::<Result<_, _>>()?,
            bico: p
                .pack
                .bico
                .iter()
                .map(|c| cor(c))
                .collect::<Result<_, _>>()?,
            clarao: cor(&p.pack.clarao)?,
            troca: p
                .troca
                .iter()
                .map(|(de, para)| Ok((cor(de)?, cor(para)?)))
                .collect::<Result<_, String>>()?,
            letras,
            contorno: cor(&p.contorno.cor)?,
        };
        let mut sprites = BTreeMap::new();
        let pasta_acessorios = pasta.join("acessorios");
        let mut arquivos: Vec<_> = fs::read_dir(&pasta_acessorios)
            .map_err(|e| format!("{}: {e}", pasta_acessorios.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "txt"))
            .collect();
        arquivos.sort();
        for arquivo in arquivos {
            let nome = arquivo
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let texto =
                fs::read_to_string(&arquivo).map_err(|e| format!("{}: {e}", arquivo.display()))?;
            let sprite = Sprite::de_grade(&texto, &paleta.letras)
                .map_err(|e| format!("acessorios/{nome}.txt: {e}"))?;
            sprites.insert(nome, sprite);
        }
        let ancoras: Ancoras = serde_json::from_str(&ler(pasta, "ancoras.json")?)
            .map_err(|e| format!("ancoras.json: {e}"))?;
        let voos: Voos = serde_json::from_str(&ler(pasta, "chapeu_voando.json")?)
            .map_err(|e| format!("chapeu_voando.json: {e}"))?;
        let zeca: ZecaToml =
            toml::from_str(&ler(pasta, "zeca.toml")?).map_err(|e| format!("zeca.toml: {e}"))?;
        let aceitos = match fs::read_to_string(pasta.join("avisos-aceitos.txt")) {
            Ok(texto) => texto
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_owned)
                .collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(format!("avisos-aceitos.txt: {e}")),
        };
        let arte = Arte {
            paleta,
            sprites,
            ancoras,
            voos,
            zeca,
            aceitos,
        };
        arte.conferir_variantes()?;
        Ok(arte)
    }

    /// Toda variante citada existe em `acessorios/`.
    fn conferir_variantes(&self) -> Result<(), String> {
        for v in [
            &self.ancoras.regra.chapeu.variante,
            &self.ancoras.regra.gravata.variante,
        ] {
            if !self.sprites.contains_key(v) {
                return Err(format!(
                    "ancoras.json, regra: variante «{v}» não existe em acessorios/"
                ));
            }
        }
        for (tag, a) in &self.ancoras.tags {
            let mut ajustes: Vec<&Ajuste> = a.chapeu.iter().chain(a.gravata.iter()).collect();
            for q in a.quadros.values() {
                ajustes.extend(q.chapeu.iter().chain(q.gravata.iter()));
            }
            for aj in &ajustes {
                if let Some(v) = &aj.variante
                    && !self.sprites.contains_key(v)
                {
                    return Err(format!(
                        "ancoras.json, tag {tag}: variante «{v}» não existe em acessorios/"
                    ));
                }
                let mexe = aj.variante.is_some()
                    || aj.x.is_some()
                    || aj.y.is_some()
                    || aj.dx.is_some()
                    || aj.dy.is_some();
                if aj.oculto && mexe {
                    return Err(format!(
                        "ancoras.json, tag {tag}: «oculto» junto com posição ou variante não faz sentido"
                    ));
                }
            }
            // O índice tem de ser o número escrito do jeito simples: «01» ou
            // «+1» passariam no parse e nunca casariam com quadro nenhum.
            for indice in a.quadros.keys() {
                let ok = indice
                    .parse::<usize>()
                    .is_ok_and(|n| n.to_string() == *indice);
                if !ok {
                    return Err(format!(
                        "ancoras.json, tag {tag}: quadro «{indice}» não é um índice (0, 1, 2…)"
                    ));
                }
            }
        }
        for (tag, voo) in &self.voos.tags {
            for (i, q) in voo.quadros.iter().enumerate() {
                let c = &q.chapeu;
                if let Some(v) = &c.variante
                    && !self.sprites.contains_key(v)
                {
                    return Err(format!(
                        "chapeu_voando.json, {tag}[{i}]: variante «{v}» não existe em acessorios/"
                    ));
                }
                let canto = c.x.is_some() || c.y.is_some();
                let centro = c.cx.is_some() || c.cy.is_some();
                if c.assentado && (canto || centro) {
                    return Err(format!(
                        "chapeu_voando.json, {tag}[{i}]: chapéu assentado só aceita variante, dx e dy"
                    ));
                }
                let par = |a: Option<i32>, b: Option<i32>| a.is_some() && b.is_some();
                let posicao = (par(c.x, c.y) && !centro) || (par(c.cx, c.cy) && !canto);
                if !c.assentado && (c.variante.is_none() || !posicao) {
                    return Err(format!(
                        "chapeu_voando.json, {tag}[{i}]: chapéu solto precisa de variante e de x/y ou cx/cy (um par só)"
                    ));
                }
                if q.ms == 0 {
                    return Err(format!(
                        "chapeu_voando.json, {tag}[{i}]: ms precisa ser maior que 0"
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cores_e_grades() {
        assert_eq!(cor("#1D2427"), Ok([0x1D, 0x24, 0x27, 255]));
        assert!(cor("1D2427").is_err());
        assert!(cor("#12345").is_err());
        assert_eq!(hex([0x1D, 0x24, 0x27, 255]), "#1D2427");
        let letras = BTreeMap::from([('#', [1, 2, 3, 255]), ('o', [9, 9, 9, 255])]);
        let s = Sprite::de_grade("; comentário\no.o\n#o#\n", &letras).unwrap();
        assert_eq!((s.largura, s.altura), (3, 2));
        assert_eq!(s.em(0, 0), Some([9, 9, 9, 255]));
        assert_eq!(s.em(1, 0), None);
        assert_eq!(s.em(2, 1), Some([1, 2, 3, 255]));
        assert!(
            Sprite::de_grade("o.o\no\n", &letras).is_err(),
            "linhas de larguras diferentes"
        );
        assert!(Sprite::de_grade("x\n", &letras).is_err(), "letra sem cor");
        assert!(Sprite::de_grade("; só comentário\n", &letras).is_err());
    }

    /// Copia `arte/zeca` para uma pasta temporária e troca um texto num
    /// arquivo, para provar que dado errado é recusado ao carregar.
    fn arte_com(nome: &str, arquivo: &str, de: &str, para: &str) -> Result<Arte, String> {
        let origem = crate::raiz().join("arte/zeca");
        let pasta =
            std::env::temp_dir().join(format!("claude-pet-arte-{}-{nome}", std::process::id()));
        let _ = fs::remove_dir_all(&pasta);
        fs::create_dir_all(pasta.join("acessorios")).unwrap();
        for entrada in fs::read_dir(&origem)
            .unwrap()
            .chain(fs::read_dir(origem.join("acessorios")).unwrap())
        {
            let caminho = entrada.unwrap().path();
            if caminho.is_file() {
                let relativo = caminho.strip_prefix(&origem).unwrap();
                fs::copy(&caminho, pasta.join(relativo)).unwrap();
            }
        }
        let alvo = pasta.join(arquivo);
        let texto = fs::read_to_string(&alvo).unwrap();
        assert!(texto.contains(de), "«{de}» não está em {arquivo}");
        fs::write(&alvo, texto.replacen(de, para, 1)).unwrap();
        let arte = Arte::carregar(&pasta);
        let _ = fs::remove_dir_all(&pasta);
        arte
    }

    #[test]
    fn dado_torto_e_recusado_ao_carregar() {
        // Índice de quadro que nunca casaria («01» não é «1»).
        let erro = arte_com(
            "indice",
            "ancoras.json",
            "\"9\": {\"gravata\"",
            "\"09\": {\"gravata\"",
        )
        .unwrap_err();
        assert!(erro.contains("«09»"), "{erro}");
        // Esconder e mexer ao mesmo tempo.
        let erro = arte_com(
            "oculto",
            "ancoras.json",
            "\"oculto\": true}",
            "\"oculto\": true, \"dx\": 1}",
        )
        .unwrap_err();
        assert!(erro.contains("oculto"), "{erro}");
        // Chapéu solto com canto e centro juntos.
        let erro = arte_com(
            "centro",
            "chapeu_voando.json",
            "\"cx\": 26, \"cy\": 9}",
            "\"cx\": 26, \"cy\": 9, \"x\": 1, \"y\": 1}",
        )
        .unwrap_err();
        assert!(erro.contains("um par só"), "{erro}");
        // Preto nunca é cor dos acessórios.
        let erro = arte_com("preto", "paleta.toml", "\"#1D2427\"", "\"#000000\"").unwrap_err();
        assert!(erro.contains("#000000"), "{erro}");
        assert!(arte_com("igual", "zeca.toml", "zeca", "zeca").is_ok());
    }

    #[test]
    fn arte_do_repositorio_carrega() {
        let arte = Arte::carregar(&crate::raiz().join("arte/zeca")).unwrap();
        for v in ["chapeu", "gravata"] {
            assert!(arte.sprites.contains_key(v), "{v}");
        }
        assert_eq!(
            arte.paleta.troca,
            vec![],
            "visual 1: o bico rosa original fica"
        );
        assert!(arte.zeca.estados.contains_key("idle"));
        assert!(arte.zeca.skin.excluir.contains(&"death".to_owned()));
    }
}
