//! `cargo xtask cobertura <pasta> [--saida <arquivo.md>]`: como a skin
//! cobre cada estado semântico do pet (`pet_core::estados::CATALOGO`) e
//! escreve `cobertura.md` (na pasta da skin, se não houver `--saida`).
//!
//! Cada estado sai como **nativo** (a skin tem tags para ele), **receita**
//! (o core monta a partir de outros, no M6), **reserva** (usa outro estado
//! que a skin cobre) ou **faltando**. Sai 1 se algum estado falta: o M2 exige
//! `cobertura.md` sem "faltando" (PLANO.md).

use std::path::{Path, PathBuf};

use pet_core::estados::{self, CATALOGO, Cobertura};
use pet_core::skin::Skin;

use crate::args::Args;

pub struct Relatorio {
    pub linhas: Vec<(&'static str, &'static str, Cobertura)>,
    /// Estados da skin que o catálogo não conhece.
    pub extras: Vec<String>,
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
        format!(
            "{} nativos, {} por receita, {} por reserva, {} faltando",
            self.contagem("nativo"),
            self.contagem("receita"),
            self.contagem("reserva"),
            self.contagem("faltando")
        )
    }
}

pub fn cobrir(skin: &Skin) -> Relatorio {
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
    Relatorio { linhas, extras }
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
    let a = Args::ler(lista, &["--saida"], &[])?;
    let [pasta] = a.posicionais() else {
        return Err("dê uma pasta de skin".into());
    };
    let pasta = Path::new(pasta);
    let skin = Skin::carregar(pasta).map_err(|e| format!("{}: {e}", pasta.display()))?;
    let r = cobrir(&skin);
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
    Ok(r.faltando() == 0)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn skin_de_teste_nao_tem_estado_faltando() {
        let skin = Skin::carregar(&crate::raiz().join("skins/_teste")).unwrap();
        let r = cobrir(&skin);
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
