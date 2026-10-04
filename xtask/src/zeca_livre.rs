//! `cargo xtask zeca-livre [--conferir]`: o Zeca original, arte livre (CC0 1.0) feita com o
//! Claude para o projeto bichinho (decisões 0065 e 0066).
//!
//! A fonte da arte é o gerador `arte/zeca-livre/zeca.py`: paleta, grades das peças, rig e
//! animações, só com a biblioteca padrão do Python. Os PNG saem dele, nunca de edição à mão.
//! Este comando roda o gerador numa pasta temporária (`zeca.py --quadros`: os quadros dos dois
//! visuais e o `anims.json`, com a verificação da própria arte e sem o ImageMagick) e grava no
//! repositório o que vai para o git:
//! - `arte/zeca-livre/anims.json`, o manifesto: quadros e durações de cada animação, os
//!   trechos do repouso, as transições de cada laço e os gatilhos sugeridos;
//! - as duas skins, montadas pela receita `arte/zeca-livre/skin.toml` (tags feitas de pedaços
//!   das animações, o mapa dos estados, os pés e as tags no chão): `skins/zeca-livre/` com o
//!   visual padrão (tema claro) e `skins/zeca-livre-escuro/` com os mesmos quadros e o anel
//!   de 1 px por fora (tema escuro; decisão 0066). Cada uma com `skin.json`
//!   (`redistribuivel: true`, `licenca: CC0-1.0`), a folha (`sheet.png` e `sheet.json` no
//!   json-array do Aseprite, quadros iguais numa célula só) e o `CREDITS.md`. O toque e o
//!   `corpo_px` saem da pose parada sem anel: as duas variantes ficam do mesmo tamanho.
//!
//! `--conferir` não grava nada: roda o gerador duas vezes e compara os bytes de tudo o que ele
//! escreveu (o gerador tem de ser determinístico), monta as skins de novo e confere que o
//! manifesto e as skins do git são o que sai agora (gerador ou receita mudados sem rodar
//! `cargo xtask zeca-livre`, ou arquivo mexido à mão, reprova). O `bin/pet verificar` roda o
//! `--conferir` quando há `python3`.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use pet_core::geometria::{self, Tamanho};
use pet_core::skin::{Skin, decodificar_png};
use serde::Deserialize;
use serde_json::json;

use crate::args::Args;
use crate::folha::{self, Folha, QuadroFolha, TagFolha};

/// Onde mora a arte, relativo à raiz do repositório.
pub const ARTE: &str = "arte/zeca-livre";

pub const USO: &str = "uso: cargo xtask zeca-livre [--conferir]";

/// O Python que roda o gerador (`PET_PYTHON` troca).
fn python() -> String {
    std::env::var("PET_PYTHON").unwrap_or_else(|_| "python3".into())
}

/// Pasta temporária que se apaga sozinha.
pub struct Temporaria(PathBuf);

impl Temporaria {
    pub fn nova(nome: &str) -> Result<Temporaria, String> {
        static N: AtomicU32 = AtomicU32::new(0);
        let pasta = std::env::temp_dir().join(format!(
            "bichinho-{nome}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&pasta);
        fs::create_dir_all(&pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
        Ok(Temporaria(pasta))
    }

    pub fn caminho(&self) -> &Path {
        &self.0
    }
}

impl Drop for Temporaria {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Roda `zeca.py --quadros <destino>` (quadros dos dois visuais e `anims.json`). Sem
/// `__pycache__` e com a semente de hash fixa: nada do ambiente entra na saída.
pub fn gerar(arte: &Path, destino: &Path) -> Result<(), String> {
    let gerador = arte.join("zeca.py");
    let saida = Command::new(python())
        .arg("-B")
        .arg(&gerador)
        .arg("--quadros")
        .arg(destino)
        .env("PYTHONHASHSEED", "0")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .map_err(|e| {
            format!(
                "{} {}: {e} (a arte livre sai de um gerador em Python; PET_PYTHON troca o interpretador)",
                python(),
                gerador.display()
            )
        })?;
    if !saida.status.success() {
        return Err(format!(
            "o gerador {} reprovou ({}):\n{}{}",
            gerador.display(),
            saida.status,
            String::from_utf8_lossy(&saida.stdout),
            String::from_utf8_lossy(&saida.stderr)
        ));
    }
    Ok(())
}

/// Todos os arquivos de uma pasta, pelo caminho relativo (com `/`), em ordem.
pub fn arquivos(pasta: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    fn andar(
        base: &Path,
        pasta: &Path,
        saida: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<(), String> {
        let mut entradas: Vec<PathBuf> = fs::read_dir(pasta)
            .map_err(|e| format!("{}: {e}", pasta.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        entradas.sort();
        for caminho in entradas {
            if caminho.is_dir() {
                andar(base, &caminho, saida)?;
            } else {
                let relativo = caminho
                    .strip_prefix(base)
                    .map_err(|e| e.to_string())?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                let dados =
                    fs::read(&caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
                saida.insert(relativo, dados);
            }
        }
        Ok(())
    }
    let mut saida = BTreeMap::new();
    andar(pasta, pasta, &mut saida)?;
    Ok(saida)
}

/// O que difere entre duas saídas do gerador: arquivos que só uma tem ou com bytes diferentes.
pub fn diferencas(a: &BTreeMap<String, Vec<u8>>, b: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut lista = Vec::new();
    for (nome, dados) in a {
        match b.get(nome) {
            None => lista.push(format!("{nome}: só na primeira")),
            Some(outro) if outro != dados => lista.push(format!("{nome}: bytes diferentes")),
            Some(_) => {}
        }
    }
    for nome in b.keys().filter(|n| !a.contains_key(*n)) {
        lista.push(format!("{nome}: só na segunda"));
    }
    lista
}

/// Lado da célula, em pixels de arte (o gerador desenha em 48x48).
pub const CELULA: u32 = 48;
/// A cor do anel do tema escuro (o `RING` do gerador).
pub const ANEL: [u8; 3] = [0x5E, 0x5A, 0x86];

/// `arte/zeca-livre/skin.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receita {
    pub skin: SkinReceita,
    pub escuro: VarianteReceita,
    pub estados: BTreeMap<String, Vec<String>>,
    pub tag: Vec<TagReceita>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkinReceita {
    pub id: String,
    pub nome: String,
    pub autor: String,
    pub licenca: String,
    pub fonte: String,
    pub escala_padrao: u32,
    pub pe: [i32; 2],
    pub chao: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VarianteReceita {
    pub id: String,
    pub nome: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagReceita {
    pub nome: String,
    #[serde(default)]
    pub nota: Option<String>,
    pub partes: Vec<Parte>,
}

/// Um pedaço de animação do gerador: o trecho `quadros = [de, ate]` (inclusive; sem ele, a
/// animação inteira), `vezes` seguidas.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parte {
    pub anim: String,
    #[serde(default)]
    pub quadros: Option<[usize; 2]>,
    #[serde(default)]
    pub vezes: Option<u32>,
}

impl Receita {
    pub fn ler(arte: &Path) -> Result<Receita, String> {
        let caminho = arte.join("skin.toml");
        let texto =
            fs::read_to_string(&caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
        toml::from_str(&texto).map_err(|e| format!("{}: {e}", caminho.display()))
    }
}

/// O que a montagem usa do `anims.json` do gerador (o resto do manifesto fica de fora).
#[derive(Debug, Clone, Deserialize)]
pub struct Manifesto {
    pub animacoes: BTreeMap<String, AnimManifesto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnimManifesto {
    pub quadros: Vec<QuadroManifesto>,
}

/// Um quadro: o arquivo do visual padrão, o do tema escuro (relativos à saída do gerador) e
/// a duração.
#[derive(Debug, Clone, Deserialize)]
pub struct QuadroManifesto {
    pub arquivo: String,
    pub escuro: String,
    pub ms: u32,
}

/// As duas skins que saem da mesma arte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variante {
    /// O visual padrão, para tema claro.
    Padrao,
    /// Os mesmos quadros com o anel de 1 px por fora, para tema escuro.
    Escuro,
}

/// Uma skin montada, em memória.
#[derive(Debug, Clone)]
pub struct Montada {
    pub id: String,
    pub skin_json: String,
    pub folha: Folha,
    pub creditos: String,
    pub toque: [i32; 4],
    pub corpo_px: u32,
    pub quadros: usize,
    pub tags: usize,
}

impl Montada {
    /// Os arquivos da pasta da skin: (nome, bytes).
    pub fn arquivos(&self) -> Vec<(&'static str, Vec<u8>)> {
        vec![
            ("skin.json", self.skin_json.clone().into_bytes()),
            ("sheet.json", self.folha.json.clone().into_bytes()),
            ("sheet.png", self.folha.png.clone()),
            ("CREDITS.md", self.creditos.clone().into_bytes()),
        ]
    }
}

/// Erros de dado na receita: tag repetida ou vazia, animação ou trecho que não existe, estado
/// ou chão apontando para tag que não existe, sem `idle`.
fn conferir_receita(r: &Receita, m: &Manifesto) -> Result<(), String> {
    let mut nomes: Vec<&str> = Vec::new();
    for t in &r.tag {
        if nomes.contains(&t.nome.as_str()) {
            return Err(format!("skin.toml: a tag «{}» aparece duas vezes", t.nome));
        }
        nomes.push(&t.nome);
        if t.partes.is_empty() {
            return Err(format!("skin.toml: a tag «{}» não tem partes", t.nome));
        }
        for p in &t.partes {
            let anim = m.animacoes.get(&p.anim).ok_or_else(|| {
                format!(
                    "skin.toml, tag «{}»: a animação «{}» não existe no anims.json",
                    t.nome, p.anim
                )
            })?;
            if let Some([de, ate]) = p.quadros
                && (de > ate || ate >= anim.quadros.len())
            {
                return Err(format!(
                    "skin.toml, tag «{}»: quadros {de}..={ate} fora de «{}» ({} quadros)",
                    t.nome,
                    p.anim,
                    anim.quadros.len()
                ));
            }
            if p.vezes == Some(0) {
                return Err(format!("skin.toml, tag «{}»: vezes = 0", t.nome));
            }
        }
    }
    for (estado, tags) in &r.estados {
        if tags.is_empty() {
            return Err(format!("skin.toml: o estado «{estado}» sem tag"));
        }
        if let Some(t) = tags.iter().find(|t| !nomes.contains(&t.as_str())) {
            return Err(format!(
                "skin.toml, estado «{estado}»: a tag «{t}» não existe"
            ));
        }
    }
    if !r.estados.contains_key("idle") {
        return Err("skin.toml: falta o estado «idle»".into());
    }
    if let Some(t) = r.skin.chao.iter().find(|t| !nomes.contains(&t.as_str())) {
        return Err(format!("skin.toml, chao: a tag «{t}» não existe"));
    }
    Ok(())
}

/// RGBA de um quadro do gerador (célula inteira, alfa só 0 ou 255), lido uma vez.
fn quadro<'a>(
    pasta: &Path,
    arquivo: &str,
    cache: &'a mut HashMap<String, Vec<u8>>,
) -> Result<&'a Vec<u8>, String> {
    if !cache.contains_key(arquivo) {
        let caminho = pasta.join(arquivo);
        let bytes = fs::read(&caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
        let img = decodificar_png(&bytes).map_err(|e| format!("{}: {e}", caminho.display()))?;
        if (img.largura, img.altura) != (CELULA as i32, CELULA as i32) {
            return Err(format!(
                "{}: {}x{}, a célula é {CELULA}x{CELULA}",
                caminho.display(),
                img.largura,
                img.altura
            ));
        }
        if img.rgba.chunks_exact(4).any(|p| p[3] != 0 && p[3] != 255) {
            return Err(format!("{}: alfa parcial", caminho.display()));
        }
        cache.insert(arquivo.to_owned(), img.rgba);
    }
    Ok(&cache[arquivo])
}

/// Os quadros e as tags da folha, na ordem da receita, para uma variante.
fn sequencias(
    r: &Receita,
    m: &Manifesto,
    pasta: &Path,
    variante: Variante,
    cache: &mut HashMap<String, Vec<u8>>,
) -> Result<(Vec<QuadroFolha>, Vec<TagFolha>), String> {
    let mut quadros = Vec::new();
    let mut tags = Vec::new();
    for t in &r.tag {
        let de = quadros.len();
        let mut origem = Vec::new();
        for p in &t.partes {
            let anim = &m.animacoes[&p.anim];
            let [a, b] = p.quadros.unwrap_or([0, anim.quadros.len() - 1]);
            let vezes = p.vezes.unwrap_or(1);
            let mut desc = p.anim.clone();
            if p.quadros.is_some() {
                desc.push_str(&format!(" {a}-{b}"));
            }
            if vezes > 1 {
                desc.push_str(&format!(" ×{vezes}"));
            }
            origem.push(desc);
            for _ in 0..vezes {
                for q in &anim.quadros[a..=b] {
                    let arquivo = match variante {
                        Variante::Padrao => &q.arquivo,
                        Variante::Escuro => &q.escuro,
                    };
                    quadros.push(QuadroFolha {
                        rgba: quadro(pasta, arquivo, cache)?.clone(),
                        duracao_ms: q.ms,
                    });
                }
            }
        }
        tags.push(TagFolha {
            nome: t.nome.clone(),
            // O campo `data` da tag: de onde ela veio (a folha de contato mostra).
            original: Some(origem.join(" + ")),
            de,
            ate: quadros.len() - 1,
            direcao: "forward".into(),
        });
    }
    Ok((quadros, tags))
}

/// O quadro do escuro é o do padrão com o anel: todo pixel opaco do padrão continua igual, e
/// todo pixel que o escuro acrescenta é da cor do anel (o tema escuro nunca muda o miolo).
pub fn so_anel(padrao: &[u8], escuro: &[u8]) -> bool {
    padrao.len() == escuro.len()
        && padrao
            .chunks_exact(4)
            .zip(escuro.chunks_exact(4))
            .all(|(p, e)| {
                if p[3] != 0 {
                    p == e
                } else {
                    e[3] == 0 || (e[3] == 255 && e[..3] == ANEL)
                }
            })
}

/// Duração de uma tag da receita, em ms.
fn duracao_ms(t: &TagReceita, m: &Manifesto) -> u32 {
    t.partes
        .iter()
        .map(|p| {
            let anim = &m.animacoes[&p.anim];
            let [a, b] = p.quadros.unwrap_or([0, anim.quadros.len() - 1]);
            anim.quadros[a..=b].iter().map(|q| q.ms).sum::<u32>() * p.vezes.unwrap_or(1)
        })
        .sum()
}

/// `CREDITS.md` da skin.
pub fn creditos(id: &str, variante: Variante) -> String {
    let visual = match variante {
        Variante::Padrao => "O visual padrão, para tema claro.",
        Variante::Escuro => {
            "Os mesmos quadros do visual padrão com o anel de 1 px `#5E5A86` por fora, para\n  \
             tema escuro."
        }
    };
    format!(
        "# Créditos da skin «{id}»\n\
         \n\
         - **Arte:** o Zeca original, arte original feita com o Claude para o projeto bichinho,\n  \
           desenhada do zero (nenhum pixel do pack *Cute Parrots!*). A fonte é o gerador\n  \
           `arte/zeca-livre/zeca.py`; esta pasta sai do `cargo xtask zeca-livre`.\n  \
           {visual}\n\
         - **Licença:** CC0 1.0, domínio público. Pode usar, mudar e redistribuir, para qualquer\n  \
           fim, sem pedir nem citar. A dedicação e o texto legal estão em\n  \
           `arte/zeca-livre/LICENSE`.\n\
         - **Crédito de cortesia** (não obrigatório): \"arte original feita com o Claude para o\n  \
           projeto bichinho\".\n"
    )
}

/// Commits por segundo do pet parado com a skin: o repouso do daemon
/// (`animador::Repouso`, pose fixa e rajadas) seguido por 10 min, contando as trocas de
/// quadro (cada uma é um commit Wayland; decisão 0005).
pub fn commits_parado(skin: &Skin) -> f64 {
    let repouso = pet_core::animador::Repouso::novo(skin, 0);
    let fim = 600_000;
    let (mut atual, mut t) = repouso.em(0);
    let mut commits = 1u64;
    while t < fim {
        let (q, proxima) = repouso.em(t);
        if q != atual {
            commits += 1;
            atual = q;
        }
        t = proxima.max(t + 1);
    }
    commits as f64 / (fim as f64 / 1000.0)
}

/// Monta as duas skins (padrão e escuro) a partir da saída do gerador em `pasta`.
pub fn montar(r: &Receita, m: &Manifesto, pasta: &Path) -> Result<Vec<Montada>, String> {
    conferir_receita(r, m)?;
    let mut cache = HashMap::new();
    let (padrao, tags) = sequencias(r, m, pasta, Variante::Padrao, &mut cache)?;
    // A pose parada: o primeiro quadro da primeira tag do `idle`, sem anel.
    let primeira = &r.estados["idle"][0];
    let pose = tags
        .iter()
        .find(|t| &t.nome == primeira)
        .map(|t| &padrao[t.de].rgba)
        .ok_or("a pose parada sumiu")?;
    let (x, y, w, h) =
        crate::importar::caixa_opaca(pose, (CELULA, CELULA)).ok_or("a pose parada está vazia")?;
    let toque = [x, y, w, h];
    let corpo_px = h as u32;
    let (escuro, tags_escuro) = sequencias(r, m, pasta, Variante::Escuro, &mut cache)?;
    if let Some(i) = (0..padrao.len()).find(|&i| !so_anel(&padrao[i].rgba, &escuro[i].rgba)) {
        return Err(format!(
            "o quadro {i} do escuro não é o padrão com o anel por fora (o gerador mudou o miolo?)"
        ));
    }
    let mut skins = Vec::new();
    for variante in [Variante::Padrao, Variante::Escuro] {
        let (quadros, tags) = match variante {
            Variante::Padrao => (&padrao, &tags),
            Variante::Escuro => (&escuro, &tags_escuro),
        };
        let (id, nome, pe) = match variante {
            Variante::Padrao => (r.skin.id.clone(), r.skin.nome.clone(), r.skin.pe),
            // O anel debaixo dos pés vira o chão.
            Variante::Escuro => (
                r.escuro.id.clone(),
                r.escuro.nome.clone(),
                [r.skin.pe[0], r.skin.pe[1] + 1],
            ),
        };
        let skin = json!({
            "formato": pet_core::skin::FORMATO,
            "id": id,
            "nome": nome,
            "autor": r.skin.autor,
            "licenca": r.skin.licenca,
            "fonte": r.skin.fonte,
            "redistribuivel": true,
            "folha": "sheet.png",
            "dados": "sheet.json",
            "celula": [CELULA, CELULA],
            "pe": pe,
            "toque": toque,
            "corpo_px": corpo_px,
            "escala_padrao": r.skin.escala_padrao,
            "estados": r.estados,
            "chao": r.skin.chao,
        });
        let skin_json = folha::json_bonito(&skin)?;
        let folha = folha::montar(
            (CELULA, CELULA),
            quadros,
            tags,
            "bichinho cargo xtask zeca-livre",
        )?;
        // Confere com o mesmo código do daemon, sem aviso nenhum.
        let carregada = Skin::de_partes(&skin_json, &folha.json, &folha.png)
            .map_err(|e| format!("a skin «{id}» montada não carrega: {e}"))?;
        if !carregada.avisos.is_empty() {
            return Err(format!(
                "a skin «{id}» montada carrega com avisos: {}",
                carregada.avisos.join("; ")
            ));
        }
        skins.push(Montada {
            creditos: creditos(&id, variante),
            id,
            skin_json,
            folha,
            toque,
            corpo_px,
            quadros: quadros.len(),
            tags: tags.len(),
        });
    }
    Ok(skins)
}

/// D (pixels do monitor por pixel de arte) nos dois monitores do Renan, nos três tamanhos.
fn d_nos_monitores(corpo_px: u32) -> String {
    let d = |altura: u32, t: Tamanho| geometria::calcular_d_com(altura, 1.5, corpo_px, t);
    let tres = |altura: u32| {
        [Tamanho::Pequeno, Tamanho::Normal, Tamanho::Grande]
            .map(|t| format!("{} {}", t.nome(), d(altura, t)))
            .join(", ")
    };
    format!(
        "D no eDP-1 (800 lógicos a 1.5): {}; no 4K (1440 lógicos a 1.5): {}",
        tres(800),
        tres(1440)
    )
}

/// Arquivos do repositório: (caminho relativo à raiz, bytes).
type ArquivosDoRepositorio = Vec<(String, Vec<u8>)>;

/// Arquivos do repositório que este comando escreve, com o conteúdo que sai agora do gerador
/// (na `pasta`) e da receita, e as skins montadas.
fn do_repositorio(
    arte: &Path,
    pasta: &Path,
    gerado: &BTreeMap<String, Vec<u8>>,
) -> Result<(ArquivosDoRepositorio, Vec<Montada>), String> {
    let manifesto = gerado
        .get("anims.json")
        .ok_or("o gerador não escreveu o anims.json")?;
    let mut lista = vec![(format!("{ARTE}/anims.json"), manifesto.clone())];
    let m: Manifesto =
        serde_json::from_slice(manifesto).map_err(|e| format!("anims.json do gerador: {e}"))?;
    let skins = montar(&Receita::ler(arte)?, &m, pasta)?;
    for s in &skins {
        for (nome, dados) in s.arquivos() {
            lista.push((format!("skins/{}/{nome}", s.id), dados));
        }
    }
    Ok((lista, skins))
}

/// Confere o determinismo e o que está no git, sem gravar nada. `Ok(problemas)`.
fn conferir(raiz: &Path) -> Result<Vec<String>, String> {
    let arte = raiz.join(ARTE);
    let (a, b) = (
        Temporaria::nova("zeca-livre")?,
        Temporaria::nova("zeca-livre")?,
    );
    gerar(&arte, a.caminho())?;
    gerar(&arte, b.caminho())?;
    let (primeira, segunda) = (arquivos(a.caminho())?, arquivos(b.caminho())?);
    let mut problemas: Vec<String> = diferencas(&primeira, &segunda)
        .into_iter()
        .map(|d| format!("o gerador não é determinístico: {d}"))
        .collect();
    let (lista, _) = do_repositorio(&arte, a.caminho(), &primeira)?;
    for (relativo, dados) in lista {
        match fs::read(raiz.join(&relativo)) {
            Ok(no_git) if no_git == dados => {}
            Ok(_) => problemas.push(format!(
                "{relativo} desatualizado: rode `cargo xtask zeca-livre`"
            )),
            Err(e) => problemas.push(format!("{relativo}: {e}")),
        }
    }
    println!(
        "zeca-livre: o gerador escreveu {} arquivos, duas vezes",
        primeira.len()
    );
    Ok(problemas)
}

pub fn executar(lista: &[String]) -> Result<bool, String> {
    let a = Args::ler(lista, &[], &["--conferir"])?;
    if !a.posicionais().is_empty() {
        return Err("argumento a mais".into());
    }
    let raiz = crate::raiz();
    if a.bandeira("--conferir") {
        let problemas = conferir(&raiz)?;
        for p in &problemas {
            println!("  ✗ {p}");
        }
        if problemas.is_empty() {
            println!("zeca-livre: determinístico e igual ao que está no git");
        }
        return Ok(problemas.is_empty());
    }
    let pasta = Temporaria::nova("zeca-livre")?;
    let arte = raiz.join(ARTE);
    gerar(&arte, pasta.caminho())?;
    let gerado = arquivos(pasta.caminho())?;
    let (lista, skins) = do_repositorio(&arte, pasta.caminho(), &gerado)?;
    for (relativo, dados) in lista {
        let destino = raiz.join(&relativo);
        if let Some(pai) = destino.parent() {
            fs::create_dir_all(pai).map_err(|e| format!("{}: {e}", pai.display()))?;
        }
        fs::write(&destino, dados).map_err(|e| format!("{}: {e}", destino.display()))?;
        println!("  {relativo}");
    }
    let quadros = gerado.keys().filter(|n| n.ends_with(".png")).count();
    println!("zeca-livre: {quadros} quadros gerados (os dois visuais)");
    for s in &skins {
        println!(
            "skin «{}»: {} quadros ({} células únicas), {} tags, toque {:?}, corpo_px {}",
            s.id, s.quadros, s.folha.celulas, s.tags, s.toque, s.corpo_px
        );
    }
    if let Some(s) = skins.first() {
        println!("{}", d_nos_monitores(s.corpo_px));
    }
    for s in &skins {
        let skin = Skin::carregar(&raiz.join("skins").join(&s.id))
            .map_err(|e| format!("skins/{}: {e}", s.id))?;
        println!(
            "parado «{}»: {:.2} commits/s (orçamento: até {})",
            s.id,
            commits_parado(&skin),
            pet_core::animador::COMMITS_POR_S_PARADO
        );
    }
    let receita = Receita::ler(&arte)?;
    let m: Manifesto = serde_json::from_slice(&gerado["anims.json"])
        .map_err(|e| format!("anims.json do gerador: {e}"))?;
    for t in &receita.tag {
        let estados: Vec<&str> = receita
            .estados
            .iter()
            .filter(|(_, tags)| tags.contains(&t.nome))
            .map(|(e, _)| e.as_str())
            .collect();
        println!(
            "  {:<15} {:>5} ms  {}{}",
            t.nome,
            duracao_ms(t, &m),
            t.nota.as_deref().unwrap_or(""),
            if estados.is_empty() {
                String::new()
            } else {
                format!(" [{}]", estados.join(", "))
            }
        );
    }
    Ok(true)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn diferencas_acham_bytes_e_nomes() {
        let a: BTreeMap<String, Vec<u8>> = [("x".into(), vec![1]), ("y".into(), vec![2])]
            .into_iter()
            .collect();
        let mut b = a.clone();
        assert!(diferencas(&a, &b).is_empty());
        b.insert("y".into(), vec![3]);
        b.insert("z".into(), vec![4]);
        b.remove("x");
        assert_eq!(
            diferencas(&a, &b),
            vec![
                "x: só na primeira".to_owned(),
                "y: bytes diferentes".to_owned(),
                "z: só na segunda".to_owned()
            ]
        );
    }

    #[test]
    fn arquivos_anda_nas_subpastas_em_ordem() {
        let t = Temporaria::nova("zeca-livre-teste").unwrap();
        fs::create_dir_all(t.caminho().join("frames/escuro")).unwrap();
        fs::write(t.caminho().join("frames/escuro/b.png"), [2]).unwrap();
        fs::write(t.caminho().join("frames/a.png"), [1]).unwrap();
        fs::write(t.caminho().join("anims.json"), [0]).unwrap();
        let lista = arquivos(t.caminho()).unwrap();
        assert_eq!(
            lista.keys().cloned().collect::<Vec<_>>(),
            vec!["anims.json", "frames/a.png", "frames/escuro/b.png"]
        );
        let caminho = t.caminho().to_path_buf();
        drop(t);
        assert!(!caminho.exists(), "a pasta temporária se apaga");
    }

    #[test]
    fn manifesto_do_git_tem_os_trechos_e_as_transicoes() {
        // O anims.json do repositório (sem rodar o gerador): o trecho «respira» é 0-3 (a
        // crítica: os quadros 4-6 repetiam os 0-2) e cada laço tem entrada e saída.
        let texto = fs::read_to_string(crate::raiz().join(ARTE).join("anims.json")).unwrap();
        let m: serde_json::Value = serde_json::from_str(&texto).unwrap();
        let trechos = &m["animacoes"]["idle"]["trechos"];
        assert_eq!(trechos["respira"]["quadros"], serde_json::json!([0, 3]));
        assert_eq!(trechos["ginga"]["quadros"], serde_json::json!([7, 13]));
        let lacos = &m["uso"]["transicoes"]["lacos"];
        for laco in ["work", "sleep", "attention", "fly"] {
            for lado in ["entrar", "sair"] {
                let anim = lacos[laco][lado].as_str().unwrap();
                assert!(
                    m["animacoes"][anim]["quadros"].as_array().is_some(),
                    "{laco}.{lado} → {anim} sem quadros"
                );
            }
        }
        assert!(m["licenca"].as_str().unwrap().starts_with("CC0-1.0"));
    }

    // --- a montagem, com uma saída de gerador de mentira ------------------------------------

    /// Quadro 48x48 com um bloco opaco (x0..=x1, y0..=y1) da cor dada.
    fn bloco(x0: usize, y0: usize, x1: usize, y1: usize, cor: [u8; 3]) -> Vec<u8> {
        let c = CELULA as usize;
        let mut rgba = vec![0u8; c * c * 4];
        for y in y0..=y1 {
            for x in x0..=x1 {
                rgba[(y * c + x) * 4..(y * c + x) * 4 + 4]
                    .copy_from_slice(&[cor[0], cor[1], cor[2], 255]);
            }
        }
        rgba
    }

    /// O mesmo quadro com o anel de 1 px (4 vizinhos) por fora, como o gerador faz.
    fn com_anel(rgba: &[u8]) -> Vec<u8> {
        let c = CELULA as i32;
        let opaco = |x: i32, y: i32| {
            (0..c).contains(&x) && (0..c).contains(&y) && rgba[((y * c + x) * 4 + 3) as usize] != 0
        };
        let mut saida = rgba.to_vec();
        for y in 0..c {
            for x in 0..c {
                if !opaco(x, y)
                    && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                        .iter()
                        .any(|(dx, dy)| opaco(x + dx, y + dy))
                {
                    let i = ((y * c + x) * 4) as usize;
                    saida[i..i + 4].copy_from_slice(&[ANEL[0], ANEL[1], ANEL[2], 255]);
                }
            }
        }
        saida
    }

    /// Uma saída de gerador de mentira: as animações `parado` (2 quadros, o corpo sobe 1 px
    /// no segundo) e `pulo` (3 quadros), nos dois visuais.
    fn gerador_de_mentira() -> (Temporaria, Manifesto) {
        let t = Temporaria::nova("zeca-livre-montar").unwrap();
        fs::create_dir_all(t.caminho().join("frames/escuro")).unwrap();
        let quadros: [(&str, Vec<u8>, u32); 5] = [
            ("parado_00", bloco(10, 20, 30, 44, [10, 200, 10]), 420),
            ("parado_01", bloco(10, 19, 30, 44, [10, 200, 10]), 140),
            ("pulo_00", bloco(10, 22, 30, 44, [10, 200, 10]), 100),
            ("pulo_01", bloco(10, 14, 30, 38, [10, 200, 10]), 100),
            ("pulo_02", bloco(10, 20, 30, 44, [200, 10, 10]), 200),
        ];
        let mut animacoes: BTreeMap<String, AnimManifesto> = BTreeMap::new();
        for (nome, rgba, ms) in &quadros {
            let png = pet_core::skin::codificar_png(CELULA, CELULA, rgba).unwrap();
            fs::write(t.caminho().join(format!("frames/{nome}.png")), &png).unwrap();
            let escuro = pet_core::skin::codificar_png(CELULA, CELULA, &com_anel(rgba)).unwrap();
            fs::write(
                t.caminho().join(format!("frames/escuro/{nome}.png")),
                &escuro,
            )
            .unwrap();
            let anim = nome.split('_').next().unwrap().to_owned();
            animacoes
                .entry(anim)
                .or_insert(AnimManifesto {
                    quadros: Vec::new(),
                })
                .quadros
                .push(QuadroManifesto {
                    arquivo: format!("frames/{nome}.png"),
                    escuro: format!("frames/escuro/{nome}.png"),
                    ms: *ms,
                });
        }
        (t, Manifesto { animacoes })
    }

    fn receita_de_mentira() -> Receita {
        toml::from_str(
            r#"
            [skin]
            id = "bicho"
            nome = "Bicho"
            autor = "testes"
            licenca = "CC0-1.0"
            fonte = "testes"
            escala_padrao = 3
            pe = [20, 45]
            chao = ["respira"]
            [escuro]
            id = "bicho-escuro"
            nome = "Bicho (escuro)"
            [estados]
            idle = ["respira"]
            done_small = ["festa"]
            [[tag]]
            nome = "respira"
            partes = [{ anim = "parado" }]
            [[tag]]
            nome = "festa"
            nota = "pula duas vezes e para"
            partes = [{ anim = "pulo", quadros = [0, 1], vezes = 2 }, { anim = "parado", quadros = [0, 0] }]
            "#,
        )
        .unwrap()
    }

    #[test]
    fn monta_as_duas_variantes_com_o_mesmo_tamanho() {
        let (pasta, m) = gerador_de_mentira();
        let skins = montar(&receita_de_mentira(), &m, pasta.caminho()).unwrap();
        assert_eq!(skins.len(), 2);
        let (claro, escuro) = (&skins[0], &skins[1]);
        assert_eq!(
            (claro.id.as_str(), escuro.id.as_str()),
            ("bicho", "bicho-escuro")
        );
        // A pose parada (parado_00) dá o toque e o corpo_px das duas.
        assert_eq!(claro.toque, [10, 20, 21, 25]);
        assert_eq!(
            (claro.toque, claro.corpo_px),
            (escuro.toque, escuro.corpo_px)
        );
        // respira (2) + festa (2 × 2 + 1) = 7 quadros; 4 imagens diferentes.
        assert_eq!((claro.quadros, claro.folha.celulas), (7, 4));
        let s = Skin::de_partes(&claro.skin_json, &claro.folha.json, &claro.folha.png).unwrap();
        assert!(s.redistribuivel && s.licenca == "CC0-1.0");
        assert_eq!(s.ancoras.pe, (20, 45));
        let festa = s.tag("festa").unwrap();
        let ms: Vec<u32> = (festa.de..=festa.ate)
            .map(|q| s.quadros[q].duracao_ms)
            .collect();
        assert_eq!(ms, vec![100, 100, 100, 100, 420]);
        let dados: serde_json::Value = serde_json::from_str(&claro.folha.json).unwrap();
        assert_eq!(
            dados["meta"]["frameTags"][1]["data"],
            "pulo 0-1 ×2 + parado 0-0"
        );
        // No escuro, o anel debaixo dos pés vira o chão.
        let e = Skin::de_partes(&escuro.skin_json, &escuro.folha.json, &escuro.folha.png).unwrap();
        assert_eq!(e.ancoras.pe, (20, 46));
        assert!(escuro.creditos.contains("anel") && escuro.creditos.contains("CC0 1.0"));
        // Mesmos bytes a cada montagem.
        let de_novo = montar(&receita_de_mentira(), &m, pasta.caminho()).unwrap();
        assert_eq!(de_novo[1].folha.png, escuro.folha.png);
        assert_eq!(de_novo[1].skin_json, escuro.skin_json);
    }

    #[test]
    fn escuro_que_mexe_no_miolo_reprova() {
        let (pasta, m) = gerador_de_mentira();
        // O escuro do pulo_02 com um pixel do miolo trocado.
        let mut rgba = com_anel(&bloco(10, 20, 30, 44, [200, 10, 10]));
        let i = ((30 * CELULA + 20) * 4) as usize;
        rgba[i..i + 3].copy_from_slice(&[1, 2, 3]);
        let png = pet_core::skin::codificar_png(CELULA, CELULA, &rgba).unwrap();
        fs::write(pasta.caminho().join("frames/escuro/pulo_02.png"), png).unwrap();
        let mut r = receita_de_mentira();
        r.tag[1].partes.push(Parte {
            anim: "pulo".into(),
            quadros: Some([2, 2]),
            vezes: None,
        });
        let erro = montar(&r, &m, pasta.caminho()).unwrap_err();
        assert!(erro.contains("anel por fora"), "{erro}");
        assert!(so_anel(
            &bloco(1, 1, 2, 2, [9, 9, 9]),
            &com_anel(&bloco(1, 1, 2, 2, [9, 9, 9]))
        ));
    }

    #[test]
    fn receita_errada_e_erro_claro() {
        let (pasta, m) = gerador_de_mentira();
        let erro = |mexer: &dyn Fn(&mut Receita)| {
            let mut r = receita_de_mentira();
            mexer(&mut r);
            montar(&r, &m, pasta.caminho()).unwrap_err()
        };
        assert!(erro(&|r| r.tag[1].partes[0].anim = "voo".into()).contains("«voo» não existe"));
        assert!(erro(&|r| r.tag[1].partes[0].quadros = Some([1, 3])).contains("fora de «pulo»"));
        assert!(erro(&|r| r.tag[1].partes[0].vezes = Some(0)).contains("vezes = 0"));
        assert!(erro(&|r| r.tag[1].nome = "respira".into()).contains("duas vezes"));
        assert!(
            erro(&|r| {
                r.estados.insert("nod".into(), vec!["aceno".into()]);
            })
            .contains("«aceno» não existe")
        );
        assert!(
            erro(&|r| {
                r.estados.remove("idle");
            })
            .contains("idle")
        );
        assert!(erro(&|r| r.skin.chao.push("voo".into())).contains("chao"));
    }

    // --- as skins do git ------------------------------------------------------------------

    fn skin_do_git(id: &str) -> Skin {
        Skin::carregar(&crate::raiz().join("skins").join(id)).unwrap()
    }

    #[test]
    fn skins_do_git_carregam_limpas_e_cobrem_o_mvp() {
        let mvp = crate::cobertura::exigidos("mvp").unwrap();
        for id in ["zeca-livre", "zeca-livre-escuro"] {
            let pasta = crate::raiz().join("skins").join(id);
            let skin = skin_do_git(id);
            assert!(skin.avisos.is_empty(), "{id}: {:?}", skin.avisos);
            assert!(skin.redistribuivel, "{id} é livre e mora em skins/");
            assert_eq!(skin.licenca, "CC0-1.0");
            assert_eq!(skin.ancoras.celula, (CELULA as i32, CELULA as i32));
            let lint = crate::lint::conferir(&pasta);
            assert!(lint.erros.is_empty(), "{id}: {:?}", lint.erros);
            let r = crate::cobertura::cobrir(&skin, &mvp);
            assert!(
                r.passou(),
                "{id}: {}",
                crate::cobertura::markdown(&skin, &r)
            );
            assert_eq!(r.contagem("reserva"), 0, "{id}: nada cai em reserva");
        }
    }

    #[test]
    fn escuro_do_git_e_o_padrao_com_o_anel() {
        let (claro, escuro) = (skin_do_git("zeca-livre"), skin_do_git("zeca-livre-escuro"));
        assert_eq!(claro.quadros.len(), escuro.quadros.len());
        assert_eq!(claro.tags, escuro.tags);
        assert_eq!(claro.estados, escuro.estados);
        assert_eq!(
            (claro.ancoras.toque, claro.corpo_px),
            (escuro.ancoras.toque, escuro.corpo_px)
        );
        assert_eq!(claro.ancoras.pe.1 + 1, escuro.ancoras.pe.1);
        for q in 0..claro.quadros.len() {
            assert_eq!(claro.quadros[q].duracao_ms, escuro.quadros[q].duracao_ms);
            let (a, b) = (
                crate::lint::celula(&claro, q),
                crate::lint::celula(&escuro, q),
            );
            assert!(so_anel(&a, &b), "quadro {q}: o escuro mexeu no miolo");
            assert_ne!(a, b, "quadro {q}: o escuro sem anel");
        }
    }

    #[test]
    fn reacoes_do_cerebro_e_o_aceno_proprio() {
        use pet_core::animador::{sequencia, tag_da_reacao};
        for id in ["zeca-livre", "zeca-livre-escuro"] {
            let skin = skin_do_git(id);
            let tag = |reacao: &str| {
                skin.tags[tag_da_reacao(&skin, reacao).unwrap()]
                    .nome
                    .as_str()
            };
            assert_eq!(tag(pet_core::cerebro::ACENO), "nod");
            assert_eq!(tag(pet_core::cerebro::PULINHO), "hop");
            assert_eq!(tag(pet_core::cerebro::TCHAU), "wave");
            // O aceno não é pedaço do repouso (o problema da skin do pack): fora a pose, nenhuma
            // imagem dele aparece nas rajadas do idle.
            let pose = skin.canonico[sequencia(&skin.tags[skin.tags_do_estado("idle")[0]])[0]];
            let do_idle: Vec<usize> = skin
                .tags_do_estado("idle")
                .into_iter()
                .flat_map(|t| sequencia(&skin.tags[t]))
                .map(|q| skin.canonico[q])
                .collect();
            let nod = skin.tags.iter().position(|t| t.nome == "nod").unwrap();
            for q in sequencia(&skin.tags[nod])
                .into_iter()
                .map(|q| skin.canonico[q])
            {
                assert!(
                    q == pose || !do_idle.contains(&q),
                    "{id}: o aceno repete o repouso"
                );
            }
            assert!(
                sequencia(&skin.tags[nod])
                    .iter()
                    .any(|&q| skin.canonico[q] != pose)
            );
        }
    }

    #[test]
    fn parado_cabe_no_orcamento_de_commits() {
        // O repouso do daemon (pose fixa e rajadas, decisão 0005): até 2 commits/s em média.
        for id in ["zeca-livre", "zeca-livre-escuro"] {
            let media = commits_parado(&skin_do_git(id));
            assert!(media > 0.5 && media <= 2.0, "{id}: {media} commits/s");
        }
    }
}
