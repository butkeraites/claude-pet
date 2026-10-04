//! O foreign-toplevel genérico (`zwlr_foreign_toplevel_manager_v1`;
//! decisões 0039, 0043 e 0056): as janelas que o compositor anuncia, cada
//! uma com o endereço que a extensão do compositor dá (no Hyprland, o
//! `hyprland_toplevel_mapping_manager_v1`) e se está ativa. É por aqui que o
//! clique foca o terminal de uma sessão
//! (`zwlr_foreign_toplevel_handle_v1.activate(seat)`), sem o socket de
//! comandos do Hyprland (decisão 0006).
//!
//! O compositor manda também o título e o app id de cada janela: eles chegam
//! na conexão e são jogados fora na hora, nunca guardados (a regra de ouro
//! dos títulos).
//!
//! A janela ativa daqui vira a **semente** do anel de ativações
//! ([`EventoDesktop::JanelaInicial`]): o Motor só a usa com o anel vazio ou
//! num buraco (o socket2 fora), porque a fonte das trocas é o socket2.

use std::collections::HashMap;
use std::hash::Hash;

use pet_core::plataforma::{Alca, EventoDesktop};

/// O estado `activated` do `zwlr_foreign_toplevel_handle_v1`.
pub const ESTADO_ATIVA: u32 = 2;

/// A lista de estados do evento `state` (um `wl_array` de `uint32`) tem o
/// `activated`.
pub fn tem_ativa(estados: &[u8]) -> bool {
    estados
        .chunks_exact(4)
        .any(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]]) == ESTADO_ATIVA)
}

/// Uma janela anunciada: o handle (`H`, o objeto do Wayland; nos testes, um
/// número), o endereço e se está ativa. Nunca o título nem o app id.
#[derive(Debug, Clone)]
pub struct Janela<H> {
    pub handle: H,
    pub endereco: Option<String>,
    pub ativa: bool,
    /// A semente desta ativação já foi contada.
    semeada: bool,
}

/// As janelas da conexão, pelo id do objeto.
#[derive(Debug, Clone)]
pub struct Janelas<I, H> {
    mapa: HashMap<I, Janela<H>>,
}

impl<I, H> Default for Janelas<I, H> {
    fn default() -> Janelas<I, H> {
        Janelas {
            mapa: HashMap::new(),
        }
    }
}

impl<I: Hash + Eq + Clone, H> Janelas<I, H> {
    /// O compositor anunciou uma janela.
    pub fn nova(&mut self, id: I, handle: H) {
        self.mapa.insert(
            id,
            Janela {
                handle,
                endereco: None,
                ativa: false,
                semeada: false,
            },
        );
    }

    /// A semente, se a janela `id` está ativa, tem endereço e ainda não foi
    /// contada nesta ativação.
    fn semente(&mut self, id: &I, parede_ms: u64) -> Option<EventoDesktop> {
        let janela = self.mapa.get_mut(id)?;
        if !janela.ativa || janela.semeada {
            return None;
        }
        let endereco = janela.endereco.clone()?;
        janela.semeada = true;
        Some(EventoDesktop::JanelaInicial {
            janela: Alca(endereco),
            parede_ms,
        })
    }

    /// O endereço da janela `id` chegou (o mapeamento do compositor).
    pub fn endereco(&mut self, id: &I, endereco: String, parede_ms: u64) -> Option<EventoDesktop> {
        self.mapa.get_mut(id)?.endereco = Some(endereco);
        self.semente(id, parede_ms)
    }

    /// Os estados da janela `id` mudaram: ativa ou não.
    pub fn estado(&mut self, id: &I, ativa: bool, parede_ms: u64) -> Option<EventoDesktop> {
        let janela = self.mapa.get_mut(id)?;
        if !ativa {
            janela.semeada = false;
        }
        janela.ativa = ativa;
        self.semente(id, parede_ms)
    }

    /// A janela fechou (o handle já não serve).
    pub fn fechou(&mut self, id: &I) -> Option<Janela<H>> {
        self.mapa.remove(id)
    }

    /// O handle da janela com esse endereço.
    pub fn achar(&self, endereco: &str) -> Option<&H> {
        self.mapa
            .values()
            .find(|j| j.endereco.as_deref() == Some(endereco))
            .map(|j| &j.handle)
    }

    /// Janelas com endereço: as que dá para focar.
    pub fn com_endereco(&self) -> usize {
        self.mapa.values().filter(|j| j.endereco.is_some()).count()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn estados(lista: &[u32]) -> Vec<u8> {
        lista.iter().flat_map(|e| e.to_ne_bytes()).collect()
    }

    #[test]
    fn activated_na_lista_de_estados() {
        assert!(tem_ativa(&estados(&[2])));
        assert!(tem_ativa(&estados(&[1, 2, 3])));
        assert!(!tem_ativa(&estados(&[1, 3])));
        assert!(!tem_ativa(&[]));
        assert!(!tem_ativa(&[2, 0, 0]), "pedaço que não fecha um uint32");
    }

    #[test]
    fn a_semente_sai_uma_vez_por_ativacao_com_endereco() {
        let mut j: Janelas<u32, &str> = Janelas::default();
        j.nova(1, "h1");
        j.nova(2, "h2");
        // Ativa antes de o endereço chegar: a semente espera o endereço.
        assert_eq!(j.estado(&1, true, 10), None);
        assert_eq!(
            j.endereco(&1, "5bbf4e6128f0".into(), 20),
            Some(EventoDesktop::JanelaInicial {
                janela: Alca("5bbf4e6128f0".into()),
                parede_ms: 20
            })
        );
        assert_eq!(j.estado(&1, true, 30), None, "a mesma ativação");
        assert_eq!(j.endereco(&2, "abc".into(), 40), None, "não está ativa");
        // O foco vai para a 2: ela semeia; a 1 volta a poder semear depois.
        assert_eq!(j.estado(&1, false, 50), None);
        assert!(j.estado(&2, true, 50).is_some());
        assert!(j.estado(&1, true, 60).is_some());
        assert_eq!(j.achar("abc"), Some(&"h2"));
        assert_eq!(j.com_endereco(), 2);
        assert!(j.fechou(&2).is_some());
        assert_eq!(j.achar("abc"), None);
        assert_eq!(j.estado(&9, true, 70), None, "janela desconhecida");
    }
}
