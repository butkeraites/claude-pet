//! `cargo xtask carga --segundos N`: carga de repintura **invisível**, para a
//! medição de custo (`scripts/medir-custo.sh`).
//!
//! Um cliente Wayland do host cria uma camada OVERLAY transparente (1 pixel
//! de alfa 0 esticado pelo viewport, invisível e sem área clicável), do
//! tamanho do monitor focado, com namespace
//! `claude-pet-carga` e região de input vazia, e faz commit a cada frame
//! callback, no ritmo do monitor. Tem de ser OVERLAY: uma camada BACKGROUND
//! fica tapada pela janela em tela cheia, e o Hyprland não manda frame
//! callback para superfície tapada (medido: 2,6 commits/s em vez de 60).
//! No Hyprland 0.56 cada commit de camada
//! repinta o monitor inteiro (decisão 0005): é a carga de um vídeo em tela
//! cheia sem nada visível mudar. Com ela rodando, a diferença entre o pet
//! escondido e o pet parado mede quanto a camada do pet, sempre mapeada, custa
//! em cada repintura causada pelos outros — o custo que decide entre a camada
//! única e o plano B (revisão do M1).
//!
//! Só fala o protocolo Wayland (nenhum IPC do Hyprland) e sai sozinho depois
//! de N segundos, destruindo a camada. Com a tela apagada não há frame
//! callback: a carga faz um commit só e espera.

use std::io::ErrorKind;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};
use smithay_client_toolkit as sctk;
use smithay_client_toolkit::compositor::{
    CompositorHandler, CompositorState, FrameCallbackData, Region,
};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::client::backend::WaylandError;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_shm, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle, delegate_noop};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{delegate_registry, registry_handlers};

pub const NAMESPACE: &str = "claude-pet-carga";
/// Teto da duração, para um esquecido não ficar repintando a tela.
const MAX_SEGUNDOS: u64 = 600;

struct Carga {
    registro: RegistryState,
    saidas: OutputState,
    shm: Shm,
    camada: LayerSurface,
    viewport: WpViewport,
    _pool: SlotPool,
    buffer: Buffer,
    qh: QueueHandle<Carga>,
    configurada: bool,
    fechada: bool,
    commits: u64,
}

impl Carga {
    /// Anexa o 1x1 transparente (se o compositor já o devolveu), marca dano,
    /// pede o próximo frame callback e faz commit.
    fn commit(&mut self) {
        let superficie = self.camada.wl_surface();
        // O Hyprland devolve o SHM logo depois de copiar; se ainda não
        // devolveu, o commit sem buffer novo repinta do mesmo jeito.
        let _ = self.buffer.attach_to(superficie);
        superficie.damage_buffer(0, 0, 1, 1);
        superficie.frame(&self.qh, FrameCallbackData(superficie.clone()));
        self.camada.commit();
        self.commits += 1;
    }
}

fn segundos(args: &[String]) -> Result<u64, String> {
    match args {
        [] => Ok(30),
        [flag, valor] if flag == "--segundos" => valor
            .parse::<u64>()
            .ok()
            .filter(|s| (1..=MAX_SEGUNDOS).contains(s))
            .ok_or_else(|| format!("--segundos precisa ser inteiro entre 1 e {MAX_SEGUNDOS}")),
        _ => Err("uso: cargo xtask carga [--segundos N]".into()),
    }
}

pub fn executar(args: &[String]) -> Result<(), String> {
    let duracao = Duration::from_secs(segundos(args)?);
    let conexao =
        Connection::connect_to_env().map_err(|e| format!("sem compositor Wayland: {e}"))?;
    let (globais, mut fila) =
        registry_queue_init::<Carga>(&conexao).map_err(|e| format!("registro: {e}"))?;
    let qh = fila.handle();
    let falhou = |nome: &str, e: &dyn std::fmt::Display| format!("{nome}: {e}");
    let compositor =
        CompositorState::bind(&globais, &qh).map_err(|e| falhou("wl_compositor", &e))?;
    let shm = Shm::bind(&globais, &qh).map_err(|e| falhou("wl_shm", &e))?;
    let camadas = LayerShell::bind(&globais, &qh).map_err(|e| falhou("layer shell", &e))?;
    let viewporter = globais
        .bind::<WpViewporter, _, _>(&qh, 1..=1, ())
        .map_err(|e| falhou("wp_viewporter", &e))?;

    let superficie = compositor.create_surface(&qh);
    let viewport = viewporter.get_viewport(&superficie, &qh, ());
    let camada =
        camadas.create_layer_surface(&qh, superficie, Layer::Overlay, Some(NAMESPACE), None);
    camada.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    camada.set_size(0, 0);
    camada.set_exclusive_zone(-1);
    camada.set_keyboard_interactivity(KeyboardInteractivity::None);
    let vazia = Region::new(&compositor).map_err(|e| falhou("região", &e))?;
    camada
        .wl_surface()
        .set_input_region(Some(vazia.wl_region()));
    camada.commit();

    let mut pool = SlotPool::new(64, &shm).map_err(|e| falhou("pool", &e))?;
    let (buffer, tela) = pool
        .create_buffer(1, 1, 4, wl_shm::Format::Argb8888)
        .map_err(|e| falhou("buffer", &e))?;
    tela.fill(0);
    let mut carga = Carga {
        registro: RegistryState::new(&globais),
        saidas: OutputState::new(&globais, &qh),
        shm,
        camada,
        viewport,
        _pool: pool,
        buffer,
        qh,
        configurada: false,
        fechada: false,
        commits: 0,
    };

    let inicio = Instant::now();
    let fim = inicio + duracao;
    while !carga.fechada {
        let agora = Instant::now();
        if agora >= fim {
            break;
        }
        fila.flush().map_err(|e| format!("envio: {e}"))?;
        fila.dispatch_pending(&mut carga)
            .map_err(|e| format!("eventos: {e}"))?;
        let Some(guarda) = fila.prepare_read() else {
            continue;
        };
        let espera = (fim - agora).min(Duration::from_millis(200));
        let espera = Timespec::try_from(espera).map_err(|e| e.to_string())?;
        let prontos = {
            let fd = guarda.connection_fd();
            let mut fds = [PollFd::new(&fd, PollFlags::IN | PollFlags::ERR)];
            match rustix::event::poll(&mut fds, Some(&espera)) {
                Ok(n) => n,
                Err(rustix::io::Errno::INTR) => continue,
                Err(e) => return Err(format!("poll: {e}")),
            }
        };
        if prontos > 0 {
            match guarda.read() {
                Ok(_) => {}
                Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => return Err(format!("leitura: {e}")),
            }
        }
        fila.dispatch_pending(&mut carga)
            .map_err(|e| format!("eventos: {e}"))?;
    }
    let decorrido = inicio.elapsed().as_secs_f64();
    let configurada = carga.configurada;
    let fechada = carga.fechada;
    let commits = carga.commits;
    drop(carga);
    // Destrói a camada e confirma antes de sair (o compositor está vivo:
    // acabou de responder).
    conexao
        .roundtrip()
        .map_err(|e| format!("ida e volta final: {e}"))?;
    if !configurada {
        return Err("a camada de carga nunca recebeu configure".into());
    }
    if fechada {
        return Err("o compositor fechou a camada de carga no meio".into());
    }
    println!(
        "carga: {commits} commits em {decorrido:.1} s ({:.1}/s)",
        commits as f64 / decorrido.max(0.001)
    );
    Ok(())
}

impl Drop for Carga {
    fn drop(&mut self) {
        self.viewport.destroy();
    }
}

impl CompositorHandler for Carga {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        self.commit();
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for Carga {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.fechada = true;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let (largura, altura) = configure.new_size;
        if largura == 0 || altura == 0 {
            return;
        }
        self.viewport.set_destination(largura as i32, altura as i32);
        if !self.configurada {
            self.configurada = true;
            self.commit();
        }
    }
}

impl OutputHandler for Carga {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.saidas
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ShmHandler for Carga {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for Carga {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registro
    }
    registry_handlers![OutputState];
}

delegate_noop!(Carga: WpViewporter);
delegate_noop!(Carga: WpViewport);
delegate_registry!(Carga);
sctk::delegate_dispatch2!(Carga);

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn duracao_padrao_e_limites() {
        let s = |v: &[&str]| segundos(&v.iter().map(|x| x.to_string()).collect::<Vec<_>>());
        assert_eq!(s(&[]), Ok(30));
        assert_eq!(s(&["--segundos", "15"]), Ok(15));
        assert!(s(&["--segundos", "0"]).is_err());
        assert!(s(&["--segundos", "601"]).is_err());
        assert!(s(&["--fps", "3"]).is_err());
    }
}
