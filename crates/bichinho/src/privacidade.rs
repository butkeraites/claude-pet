//! O que o processo nunca deixa no disco (decisões 0045 e 0061).
//!
//! O hook tem o prompt na entrada padrão, e o daemon guarda na memória os
//! títulos de janela que o compositor manda (as linhas do socket2 do
//! Hyprland passam pelo buffer do leitor, e os títulos do foreign-toplevel
//! chegam na conexão Wayland e são jogados fora, mas os bytes ficam no heap
//! liberado). Um aborto (o `panic = "abort"` do perfil de release, o vigia que
//! aborta um laço travado, falta de memória) levaria tudo isso ao
//! `systemd-coredump` do host, que grava o core em disco. Sem o `dumpable`, o
//! kernel não faz o core; e o compose ainda põe o limite do core em 0.

/// Tira o `dumpable` do processo: nada de core dump, e o `/proc/<pid>` passa
/// a ser do root (proc(5)). Sem `unsafe`: o `prctl` pelo invólucro seguro do
/// `rustix`. Fora do Linux, nada (o Windows e o macOS chegam no M8).
#[cfg(target_os = "linux")]
pub fn sem_core_dump() {
    use rustix::process::{DumpableBehavior, set_dumpable_behavior};
    let _ = set_dumpable_behavior(DumpableBehavior::NotDumpable);
}

#[cfg(not(target_os = "linux"))]
pub fn sem_core_dump() {}
