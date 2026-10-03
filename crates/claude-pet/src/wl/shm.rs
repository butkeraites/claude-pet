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
pub struct Vez {
    pub indice: usize,
    /// O buffer não tem o último quadro: redesenhe tudo.
    pub redesenhar_tudo: bool,
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
        let livre = |pool: &mut SlotPool, folha: &Folha| folha.buffer.canvas(pool).is_some();
        if let Some(i) = (0..self.folhas.len())
            .find(|&i| self.folhas[i].em_dia && livre(&mut self.pool, &self.folhas[i]))
        {
            return Ok(Vez {
                indice: i,
                redesenhar_tudo: false,
            });
        }
        if let Some(i) = (0..self.folhas.len()).find(|&i| livre(&mut self.pool, &self.folhas[i])) {
            return Ok(Vez {
                indice: i,
                redesenhar_tudo: true,
            });
        }
        if self.folhas.len() >= MAX_BUFFERS {
            return Err("todos os buffers ainda estão com o compositor".into());
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
