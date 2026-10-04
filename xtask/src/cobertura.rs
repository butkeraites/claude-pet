//! `cargo xtask cobertura <pasta> [--saida <arquivo.md>]`: como a skin
//! cobre cada estado semântico do pet (`pet_core::estados::CATALOGO`) e
//! escreve `cobertura.md` (na pasta da skin, se não houver `--saida`).
//!
//! Cada estado sai como **nativo** (a skin tem tags para ele), **receita**
//! (o core monta a partir de outros, no M6), **reserva** (usa outro estado
//! que a skin cobre) ou **faltando**. Como toda skin que carrega tem `idle`,
//! a reserva de último caso, "faltando" só aparece em estado fora do
//! catálogo. Por isso o portão do personagem é `--nativos`: os estados
//! listados (ou `mvp`, a tabela do PLANO.md, item 4) têm de ser nativos. Sai
//! 1 se algum estado falta ou se algum exigido não é nativo.

use std::path::{Path, PathBuf};

use pet_core::estados::{self, CATALOGO, Cobertura};
use pet_core::skin::Skin;

use crate::args::Args;

pub struct Relatorio {
    pub linhas: Vec<(&'static str, &'static str, Cobertura)>,
    /// Estados da skin que o catálogo não conhece.
    pub extras: Vec<String>,
    /// Estados exigidos como nativos (`--nativos`) que não são.
    pub nao_nativos: Vec<String>,
    /// Quantos estados foram exigidos como nativos.
    pub exigidos: usize,
}

impl Relatorio {
    pub fn faltando(&self) -> usize {
        self.linhas
            .iter()
            .filter(|(_, _, c)| *c == Cobertura::Faltando)
            .count()
    }

    pub fn contagem(&self, nome: &str) -> usize {
        self.linhas
            .iter()
            .filter(|(_, _, c)| c.nome() == nome)
            .count()
    }

    pub fn resumo(&self) -> String {
        let mut r = format!(
            "{} nativos, {} por receita, {} por reserva, {} faltando",
            self.contagem("nativo"),
            self.contagem("receita"),
            self.contagem("reserva"),
            self.contagem("faltando")
        );
        if self.exigidos > 0 {
            r.push_str(&format!(
                "; {} de {} exigidos nativos",
                self.exigidos - self.nao_nativos.len(),
                self.exigidos
            ));
        }
        r
    }

    pub fn passou(&self) -> bool {
        self.faltando() == 0 && self.nao_nativos.is_empty()
    }
}

/// `mvp` ou uma lista separada por vírgulas.
pub fn exigidos(texto: &str) -> Result<Vec<String>, String> {
    if texto == "mvp" {
        return Ok(estados::NATIVOS_DO_MVP
            .iter()
            .map(|s| (*s).to_owned())
            .collect());
    }
    let lista: Vec<String> = texto
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    match lista.iter().find(|e| estados::estado(e).is_none()) {
        Some(e) => Err(format!("--nativos: «{e}» não é um estado do catálogo")),
        None if lista.is_empty() => Err("--nativos vazio".into()),
        None => Ok(lista),
    }
}

/// Cobertura de cada estado do catálogo; `nativos`: os que têm de ser
/// nativos (vazio: nenhum exigido).
pub fn cobrir(skin: &Skin, nativos: &[String]) -> Relatorio {
    let nativo = |estado: &str| -> Vec<String> {
        skin.tags_do_estado(estado)
            .into_iter()
            .map(|t| skin.tags[t].nome.clone())
            .collect()
    };
    let linhas = CATALOGO
        .iter()
        .map(|e| (e.chave, e.descricao, estados::cobrir(e.chave, &nativo)))
        .collect();
    let extras = skin
        .estados
        .keys()
        .filter(|k| estados::estado(k).is_none())
        .cloned()
        .collect();
    let nao_nativos = nativos
        .iter()
        .filter(|e| skin.tags_do_estado(e).is_empty())
        .cloned()
        .collect();
    Relatorio {
        linhas,
        extras,
        nao_nativos,
        exigidos: nativos.len(),
    }
}

/// `cobertura.md`.
pub fn markdown(skin: &Skin, r: &Relatorio) -> String {
    let mut md = format!(
        "# Cobertura da skin «{}»\n\n\
         Gerado por `cargo xtask cobertura`. Nativo: a skin tem tags para o estado. \
         Receita: o core monta a partir de outros estados (M6). Reserva: usa outro \
         estado que a skin cobre. Faltando: nada.\n\n\
         | Estado | O que é | Cobertura | Como |\n|---|---|---|---|\n",
        skin.id
    );
    for (chave, descricao, c) in &r.linhas {
        let como = match c {
            Cobertura::Nativo(tags) => tags
                .iter()
                .map(|t| format!("`{t}`"))
                .collect::<Vec<_>>()
                .join(", "),
            Cobertura::Receita(d) => (*d).to_owned(),
            Cobertura::Reserva(e) => format!("usa `{e}`"),
            Cobertura::Faltando => "—".to_owned(),
        };
        md.push_str(&format!(
            "| `{chave}` | {descricao} | {} | {como} |\n",
            c.nome()
        ));
    }
    md.push_str(&format!("\n**Resumo:** {}.\n", r.resumo()));
    if !r.nao_nativos.is_empty() {
        md.push_str(&format!(
            "\n**Exigidos como nativos e não são:** {}.\n",
            r.nao_nativos
                .iter()
                .map(|e| format!("`{e}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !r.extras.is_empty() {
        md.push_str(&format!(
            "\nEstados da skin fora do catálogo: {}.\n",
            r.extras
                .iter()
                .map(|e| format!("`{e}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    md
}

pub fn executar(lista: &[String]) -> Result<bool, String> {
    let a = Args::ler(lista, &["--saida", "--nativos"], &[])?;
    let [pasta] = a.posicionais() else {
        return Err("dê uma pasta de skin".into());
    };
    let pasta = Path::new(pasta);
    let skin = Skin::carregar(pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    let nativos = a.valor("--nativos").map(exigidos).transpose()?;
    let r = cobrir(&skin, nativos.as_deref().unwrap_or_default());
    let saida = a
        .valor("--saida")
        .map(PathBuf::from)
        .unwrap_or_else(|| pasta.join("cobertura.md"));
    std::fs::write(&saida, markdown(&skin, &r)).map_err(|e| format!("{}: {e}", saida.display()))?;
    println!(
        "cobertura «{}»: {} → {}",
        skin.id,
        r.resumo(),
        saida.display()
    );
    for (chave, _, c) in &r.linhas {
        if !matches!(c, Cobertura::Nativo(_)) {
            let detalhe = match c {
                Cobertura::Receita(d) => format!("receita: {d}"),
                Cobertura::Reserva(e) => format!("reserva: usa {e}"),
                _ => "FALTANDO".to_owned(),
            };
            println!("  {chave:<12} {detalhe}");
        }
    }
    for e in &r.nao_nativos {
        println!("  {e:<12} EXIGIDO NATIVO e não é");
    }
    Ok(r.passou())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn idle_sozinho_cobre_tudo_mas_nao_passa_no_mvp() {
        // Uma skin só com idle (o esqueleto do skin-importar): nada falta,
        // porque idle é a reserva de tudo; mas o personagem precisa dos
        // estados do MVP nativos.
        let skin = Skin::carregar(&crate::raiz().join("skins/_teste")).unwrap();
        let mut so_idle = skin.clone();
        so_idle.estados.retain(|e, _| e == "idle");
        let solto = cobrir(&so_idle, &[]);
        assert_eq!(solto.faltando(), 0);
        assert!(solto.passou());
        let mvp = exigidos("mvp").unwrap();
        let r = cobrir(&so_idle, &mvp);
        assert!(!r.passou());
        assert_eq!(r.nao_nativos.len(), mvp.len() - 1, "só o idle é nativo");
        assert!(markdown(&so_idle, &r).contains("Exigidos como nativos e não são"));
        assert!(exigidos("idle, wave").is_ok());
        assert!(exigidos("idle,inventado").is_err());
    }

    #[test]
    fn skin_de_teste_nao_tem_estado_faltando() {
        let skin = Skin::carregar(&crate::raiz().join("skins/_teste")).unwrap();
        let r = cobrir(&skin, &[]);
        assert_eq!(r.faltando(), 0, "{}", markdown(&skin, &r));
        assert_eq!(r.linhas.len(), CATALOGO.len());
        let md = markdown(&skin, &r);
        assert!(
            md.contains("| `idle` | parado (pose fixa com rajadas) | nativo | `idle`, `blink` |"),
            "{md}"
        );
        assert!(
            md.contains("| `nod` | aceno discreto (T0) | reserva | usa `wave` |"),
            "{md}"
        );
        assert!(md.contains("Resumo"));
    }
}
