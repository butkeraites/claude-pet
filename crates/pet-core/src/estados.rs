//! Catálogo dos estados semânticos do pet: as chaves que o `skin.json` mapeia
//! para tags (PLANO.md, "Zeca: arte e skin", item 4).
//!
//! Para cada estado, o que fazer quando a skin não tem tag para ele:
//! - **receita:** o core monta o estado a partir de outros (por exemplo, a
//!   pose parada com um "…" em cima). As receitas são desenhadas no M6; aqui
//!   fica o que cada uma precisa;
//! - **reserva:** usa outro estado no lugar (o primeiro da lista que a skin
//!   cobrir). As reservas de `nod` e `done_small` são as do animador do M3.
//!
//! `cargo xtask cobertura` classifica cada estado de uma skin em nativo,
//! receita, reserva ou faltando.

/// Um estado semântico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estado {
    pub chave: &'static str,
    /// O que é, em português, como no PLANO.
    pub descricao: &'static str,
    /// Estados que servem no lugar, em ordem.
    pub reservas: &'static [&'static str],
    /// Receita do core, se houver: o que ela faz e de que estados precisa.
    pub receita: Option<Receita>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receita {
    pub descricao: &'static str,
    pub precisa: &'static [&'static str],
}

const fn receita(descricao: &'static str, precisa: &'static [&'static str]) -> Option<Receita> {
    Some(Receita { descricao, precisa })
}

/// Todos os estados que o pet usa ou vai usar (M1–M6).
pub const CATALOGO: &[Estado] = &[
    Estado {
        chave: "idle",
        descricao: "parado (pose fixa com rajadas)",
        reservas: &[],
        receita: None,
    },
    Estado {
        chave: "working",
        descricao: "trabalhando",
        reservas: &["idle"],
        receita: None,
    },
    Estado {
        chave: "thinking",
        descricao: "pensando",
        reservas: &["idle"],
        receita: receita("pose parada com «…» em cima", &["idle"]),
    },
    Estado {
        chave: "waiting",
        descricao: "esperando você",
        reservas: &["alert", "idle"],
        receita: receita("pose parada com «!» quicando", &["idle"]),
    },
    Estado {
        chave: "alert",
        descricao: "chamada (L1, antecipação e pio)",
        reservas: &["waiting", "idle"],
        receita: receita("pose parada com «!» quicando", &["idle"]),
    },
    Estado {
        chave: "ready",
        descricao: "pronto",
        reservas: &["idle"],
        receita: receita("pose parada com a bandeirinha", &["idle"]),
    },
    Estado {
        chave: "nod",
        descricao: "aceno discreto (T0)",
        reservas: &["wave"],
        receita: None,
    },
    Estado {
        chave: "wave",
        descricao: "aceno",
        reservas: &["idle"],
        receita: None,
    },
    Estado {
        chave: "done_small",
        descricao: "terminou pequeno (T1, pulinho)",
        reservas: &["wave"],
        receita: receita("aceno com o pulo do core", &["wave"]),
    },
    Estado {
        chave: "done_medium",
        descricao: "terminou médio (T2, voo curto)",
        reservas: &["done_small"],
        receita: None,
    },
    Estado {
        chave: "done_big",
        descricao: "terminou grande (T3, voo atravessando a tela)",
        reservas: &["done_medium"],
        receita: None,
    },
    Estado {
        chave: "error",
        descricao: "erro",
        reservas: &["idle"],
        receita: receita("pose parada com raios e tremida", &["idle"]),
    },
    Estado {
        chave: "yawn",
        descricao: "cansado (bocejo)",
        reservas: &["sleep"],
        receita: None,
    },
    Estado {
        chave: "sleep",
        descricao: "dormindo",
        reservas: &["idle"],
        receita: receita("pose parada com zZ", &["idle"]),
    },
    Estado {
        chave: "wake",
        descricao: "acordando",
        reservas: &["idle"],
        receita: None,
    },
    Estado {
        chave: "dangle",
        descricao: "arrastado",
        reservas: &["idle"],
        receita: receita("pose parada balançando como pêndulo", &["idle"]),
    },
    Estado {
        chave: "land",
        descricao: "solto (pouso)",
        reservas: &["idle"],
        receita: receita("pose parada com quique e poeira", &["idle"]),
    },
    Estado {
        chave: "giggle",
        descricao: "clique (risadinha)",
        reservas: &["wave"],
        receita: receita("pose parada com coração", &["idle"]),
    },
    Estado {
        chave: "hello",
        descricao: "oi",
        reservas: &["wave"],
        receita: None,
    },
    Estado {
        chave: "bye",
        descricao: "tchau",
        reservas: &[],
        receita: receita("poof por cima da pose", &["idle"]),
    },
    Estado {
        chave: "poof_in",
        descricao: "chega no monitor novo",
        reservas: &[],
        receita: receita("poof procedural", &[]),
    },
    Estado {
        chave: "poof_out",
        descricao: "sai do monitor",
        reservas: &[],
        receita: receita("poof procedural", &[]),
    },
];

/// Estados que um personagem de verdade precisa ter **nativos** no MVP: a
/// tabela do PLANO.md ("Zeca: arte e skin", item 4). Receita e reserva
/// existem para skins de teste e para o que falta até o M6; o personagem
/// aprovado não depende delas (`cargo xtask cobertura --nativos mvp`).
pub const NATIVOS_DO_MVP: &[&str] = &[
    "idle",
    "working",
    "thinking",
    "waiting",
    "ready",
    "done_small",
    "done_medium",
    "done_big",
    "error",
    "yawn",
    "sleep",
    "wake",
    "dangle",
    "land",
    "giggle",
    "hello",
    "bye",
];

pub fn estado(chave: &str) -> Option<&'static Estado> {
    CATALOGO.iter().find(|e| e.chave == chave)
}

/// Como uma skin cobre um estado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cobertura {
    /// A skin tem tags para o estado.
    Nativo(Vec<String>),
    /// O core monta com uma receita.
    Receita(&'static str),
    /// Usa outro estado que a skin cobre.
    Reserva(&'static str),
    Faltando,
}

impl Cobertura {
    pub fn nome(&self) -> &'static str {
        match self {
            Cobertura::Nativo(_) => "nativo",
            Cobertura::Receita(_) => "receita",
            Cobertura::Reserva(_) => "reserva",
            Cobertura::Faltando => "faltando",
        }
    }
}

/// Cobertura de `chave`, dado `nativo(estado)` = as tags da skin para ele
/// (vazio se não tem). Receita antes de reserva: a receita fica mais perto
/// do estado de verdade.
pub fn cobrir(chave: &str, nativo: &dyn Fn(&str) -> Vec<String>) -> Cobertura {
    let tags = nativo(chave);
    if !tags.is_empty() {
        return Cobertura::Nativo(tags);
    }
    let Some(e) = estado(chave) else {
        return Cobertura::Faltando;
    };
    if let Some(r) = e.receita
        && r.precisa.iter().all(|p| !nativo(p).is_empty())
    {
        return Cobertura::Receita(r.descricao);
    }
    // Reservas em profundidade, sem repetir (as listas formam um grafo).
    let mut fila: Vec<&'static str> = e.reservas.to_vec();
    let mut vistos: Vec<&str> = vec![chave];
    while !fila.is_empty() {
        let r = fila.remove(0);
        if vistos.contains(&r) {
            continue;
        }
        vistos.push(r);
        if !nativo(r).is_empty() {
            return Cobertura::Reserva(r);
        }
        if let Some(outro) = estado(r) {
            fila.extend(outro.reservas.iter().copied());
        }
    }
    Cobertura::Faltando
}

#[cfg(test)]
mod testes {
    use super::*;

    fn skin<'a>(estados: &'a [&'a str]) -> impl Fn(&str) -> Vec<String> + 'a {
        move |e: &str| {
            if estados.contains(&e) {
                vec![format!("tag_{e}")]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn catalogo_coerente() {
        for e in CATALOGO {
            for r in e
                .reservas
                .iter()
                .chain(e.receita.iter().flat_map(|r| r.precisa))
            {
                assert!(
                    estado(r).is_some(),
                    "{} cita «{r}», que não existe",
                    e.chave
                );
            }
            assert_eq!(CATALOGO.iter().filter(|x| x.chave == e.chave).count(), 1);
        }
        // As reservas do M3 (animador::RESERVAS na branch m3-hooks).
        assert_eq!(estado("nod").unwrap().reservas, &["wave"]);
        assert_eq!(estado("done_small").unwrap().reservas, &["wave"]);
        for e in NATIVOS_DO_MVP {
            assert!(estado(e).is_some(), "MVP cita «{e}», fora do catálogo");
        }
    }

    #[test]
    fn nativo_receita_reserva_e_faltando() {
        let so_idle = skin(&["idle"]);
        assert_eq!(
            cobrir("idle", &so_idle),
            Cobertura::Nativo(vec!["tag_idle".into()])
        );
        assert!(matches!(
            cobrir("thinking", &so_idle),
            Cobertura::Receita(_)
        ));
        assert_eq!(cobrir("working", &so_idle), Cobertura::Reserva("idle"));
        // done_big → done_medium → done_small → wave → idle.
        assert_eq!(cobrir("done_big", &so_idle), Cobertura::Reserva("idle"));
        assert!(matches!(cobrir("poof_in", &so_idle), Cobertura::Receita(_)));
        let nada = skin(&[]);
        assert_eq!(cobrir("nod", &nada), Cobertura::Faltando);
        assert_eq!(cobrir("inventado", &nada), Cobertura::Faltando);
        let com_wave = skin(&["idle", "wave"]);
        assert_eq!(cobrir("nod", &com_wave), Cobertura::Reserva("wave"));
    }
}
