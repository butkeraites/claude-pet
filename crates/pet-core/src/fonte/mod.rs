//! A fonte dos balões (M4, decisão 0052): a **monogram**, de Vinícius
//! Menézio (datagoblin), em domínio público (CC0 1.0; a licença e a origem
//! em `assets/fonte/monogram/`). A tabela ([`glifos`]) é assada pelo `cargo
//! xtask fonte` a partir do JSON de bitmaps do pacote e vai dentro do
//! binário.
//!
//! Cada glifo tem 12 linhas de bits (o bit 0 é a coluna da esquerda): 5
//! colunas, e uns poucos acentos passam para a sexta e a sétima. O avanço é
//! de 6 colunas. Um pixel da fonte vira um bloco inteiro de pixels do
//! dispositivo, como a arte do pet (nitidez por construção, decisão 0004).

mod glifos;

/// Linhas de um glifo.
pub const ALTURA: i32 = 12;
/// Colunas de um caractere ao próximo.
pub const AVANCO: i32 = 6;
/// Colunas que um glifo pode pintar.
pub const LARGURA_MAX: i32 = 8;

/// As linhas do glifo de `c`; sem ele na fonte, as do `?`.
pub fn glifo(c: char) -> &'static [u8; 12] {
    let busca = |c: char| {
        glifos::GLIFOS
            .binary_search_by_key(&c, |(g, _)| *g)
            .ok()
            .map(|i| &glifos::GLIFOS[i].1)
    };
    busca(c).or_else(|| busca('?')).unwrap_or(&[0; 12])
}

/// A fonte tem o glifo de `c`.
pub fn tem(c: char) -> bool {
    glifos::GLIFOS.binary_search_by_key(&c, |(g, _)| *g).is_ok()
}

/// Largura de `texto` em pixels da fonte (sem o espaço depois do último).
pub fn largura(texto: &str) -> i32 {
    match texto.chars().count() as i32 {
        0 => 0,
        n => n * AVANCO - 1,
    }
}

/// `texto` com no máximo `max` caracteres; cortado, termina em "…".
pub fn cortar(texto: &str, max: usize) -> String {
    if texto.chars().count() <= max {
        return texto.to_owned();
    }
    let mut cortado: String = texto.chars().take(max.saturating_sub(1)).collect();
    cortado.push('…');
    cortado
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn os_glifos_do_portugues_estao_la() {
        for c in "Ô, meu camarada! você não há sessão ação çÇãõâêéíóúà·…0123456789".chars()
        {
            assert!(tem(c), "{c:?}");
        }
        // Em ordem de código: a busca binária depende disso.
        assert!(glifos::GLIFOS.windows(2).all(|j| j[0].0 < j[1].0));
        assert_eq!(glifo('a'), &[0, 0, 0, 0, 0, 30, 17, 17, 17, 30, 0, 0]);
        assert_eq!(glifo('\u{1F99C}'), glifo('?'), "sem o glifo: o ?");
    }

    #[test]
    fn largura_e_corte() {
        assert_eq!(largura(""), 0);
        assert_eq!(largura("a"), 5);
        assert_eq!(largura("ação"), 4 * 6 - 1);
        assert_eq!(cortar("claude-pet", 20), "claude-pet");
        assert_eq!(cortar("agenda-presidencial", 8), "agenda-…");
        assert_eq!(cortar("ãããã", 3).chars().count(), 3);
    }

    #[test]
    fn a_tabela_e_a_do_json_da_monogram() {
        let caminho = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fonte/monogram/monogram-bitmap.json");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(caminho).unwrap()).unwrap();
        let mapa = json.as_object().unwrap();
        assert_eq!(mapa.len(), glifos::GLIFOS.len());
        for (c, linhas) in glifos::GLIFOS {
            let esperado: Vec<u64> = mapa[&c.to_string()]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap())
                .collect();
            let tabela: Vec<u64> = linhas.iter().map(|&b| b as u64).collect();
            assert_eq!(tabela, esperado, "{c:?}");
        }
    }
}
