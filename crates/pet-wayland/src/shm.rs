//! Buffers SHM da camada (decisão 0004, revisão de viabilidade).
//!
//! Cada superfície ganha um **SlotPool novo** (um memfd novo), que morre com
//! ela: um pool nunca passa de um monitor ou tamanho para outro, porque o
//! SlotPool do SCTK reaproveita regiões sem zerar e cresce dobrando.
//!
//! O desenho é incremental: só as regiões com dano são refeitas. Isso exige
//! saber se o buffer escolhido tem o último quadro enviado ("em dia"). O
//! normal é um buffer só: o Hyprland copia o SHM para a textura no commit e
//! devolve o buffer logo. Se ele ainda estiver ocupado, entra um segundo
//! buffer, limpo e redesenhado inteiro antes do primeiro commit; um buffer
//! que ficou para trás também é redesenhado inteiro antes de voltar a uso.

use smithay_client_toolkit::reexports::client::protocol::wl_shm;
use smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};

/// No máximo três buffers por superfície (o terceiro só quando os outros
/// dois ainda estão com o compositor, por exemplo ao esconder).
const MAX_BUFFERS: usize = 3;

struct Folha {
    buffer: Buffer,
    /// Tem o conteúdo do último quadro enviado.
    em_dia: bool,
}

pub struct Lona {
    pool: SlotPool,
    folhas: Vec<Folha>,
    pub largura: i32,
    pub altura: i32,
}

/// Um buffer pronto para desenhar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vez {
    pub indice: usize,
    /// O buffer não tem o último quadro: redesenhe tudo.
    pub redesenhar_tudo: bool,
}

/// Qual buffer usar no próximo quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escolha {
    Usar(Vez),
    /// Um buffer novo, zerado e redesenhado inteiro.
    Criar,
    /// Todos ainda estão com o compositor e não cabe mais nenhum.
    Nenhum,
}

/// Escolha pura, dado `(em_dia, livre)` de cada buffer: um livre e em dia
/// (desenho incremental); senão um livre (redesenhado inteiro); senão um
/// novo, se couber.
pub fn escolher(folhas: &[(bool, bool)], max: usize) -> Escolha {
    let achar = |cond: fn(&(bool, bool)) -> bool| folhas.iter().position(cond);
    if let Some(indice) = achar(|&(em_dia, livre)| em_dia && livre) {
        return Escolha::Usar(Vez {
            indice,
            redesenhar_tudo: false,
        });
    }
    if let Some(indice) = achar(|&(_, livre)| livre) {
        return Escolha::Usar(Vez {
            indice,
            redesenhar_tudo: true,
        });
    }
    if folhas.len() < max {
        Escolha::Criar
    } else {
        Escolha::Nenhum
    }
}

impl Lona {
    /// Pool novo do tamanho exato de um buffer `largura`x`altura`.
    pub fn nova(shm: &Shm, largura: i32, altura: i32) -> Result<Lona, String> {
        let tamanho = largura as usize * altura as usize * 4;
        let pool = SlotPool::new(tamanho, shm).map_err(|e| format!("pool SHM: {e}"))?;
        Ok(Lona {
            pool,
            folhas: Vec::new(),
            largura,
            altura,
        })
    }

    /// Escolhe o buffer do próximo quadro: um livre e em dia; senão um
    /// livre (que será redesenhado inteiro); senão um novo, zerado.
    pub fn pegar(&mut self) -> Result<Vez, String> {
        let estados: Vec<(bool, bool)> = self
            .folhas
            .iter()
            .map(|f| (f.em_dia, f.buffer.canvas(&mut self.pool).is_some()))
            .collect();
        match escolher(&estados, MAX_BUFFERS) {
            Escolha::Usar(vez) => return Ok(vez),
            Escolha::Nenhum => return Err("todos os buffers ainda estão com o compositor".into()),
            Escolha::Criar => {}
        }
        let passo = self.largura * 4;
        let (buffer, tela) = self
            .pool
            .create_buffer(self.largura, self.altura, passo, wl_shm::Format::Argb8888)
            .map_err(|e| format!("buffer SHM: {e}"))?;
        // O pool pode ter reaproveitado uma região: nunca confie em zeros.
        tela.fill(0);
        self.folhas.push(Folha {
            buffer,
            em_dia: false,
        });
        if self.folhas.len() > 1 {
            depurar!("buffer extra criado ({} no total)", self.folhas.len());
        }
        Ok(Vez {
            indice: self.folhas.len() - 1,
            redesenhar_tudo: true,
        })
    }

    /// Os pixels do buffer `indice` (só enquanto ele está livre).
    pub fn tela(&mut self, indice: usize) -> Option<&mut [u8]> {
        self.folhas[indice].buffer.canvas(&mut self.pool)
    }

    /// Anexa o buffer à superfície; ele passa a ser o único em dia.
    pub fn anexar(&mut self, indice: usize, superficie: &WlSurface) -> Result<(), String> {
        self.folhas[indice]
            .buffer
            .attach_to(superficie)
            .map_err(|e| format!("anexar buffer: {e}"))?;
        for (i, folha) in self.folhas.iter_mut().enumerate() {
            folha.em_dia = i == indice;
        }
        Ok(())
    }

    /// Bytes de SHM do pool (para o estado e o orçamento de memória).
    pub fn bytes(&self) -> usize {
        self.pool.len()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const LIVRE_EM_DIA: (bool, bool) = (true, true);
    const LIVRE_VELHO: (bool, bool) = (false, true);
    const OCUPADO_EM_DIA: (bool, bool) = (true, false);
    const OCUPADO_VELHO: (bool, bool) = (false, false);

    fn usar(indice: usize, redesenhar_tudo: bool) -> Escolha {
        Escolha::Usar(Vez {
            indice,
            redesenhar_tudo,
        })
    }

    #[test]
    fn primeiro_quadro_cria_um_buffer() {
        assert_eq!(escolher(&[], MAX_BUFFERS), Escolha::Criar);
    }

    #[test]
    fn buffer_devolvido_e_em_dia_desenha_so_o_dano() {
        // O normal no Hyprland: o SHM volta logo depois do commit.
        assert_eq!(escolher(&[LIVRE_EM_DIA], MAX_BUFFERS), usar(0, false));
        assert_eq!(
            escolher(&[LIVRE_VELHO, LIVRE_EM_DIA], MAX_BUFFERS),
            usar(1, false),
            "o em dia ganha do velho"
        );
    }

    #[test]
    fn buffer_que_ficou_para_tras_e_redesenhado_inteiro() {
        assert_eq!(
            escolher(&[OCUPADO_EM_DIA, LIVRE_VELHO], MAX_BUFFERS),
            usar(1, true)
        );
    }

    #[test]
    fn todos_ocupados_cria_ate_o_teto() {
        assert_eq!(escolher(&[OCUPADO_EM_DIA], MAX_BUFFERS), Escolha::Criar);
        assert_eq!(
            escolher(&[OCUPADO_EM_DIA, OCUPADO_VELHO, OCUPADO_VELHO], MAX_BUFFERS),
            Escolha::Nenhum
        );
    }
}
