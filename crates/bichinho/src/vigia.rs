//! Vigia: se o laço principal parar de bater por mais de 60 s, aborta o
//! processo e deixa o `restart: unless-stopped` do Docker trazê-lo de volta.
//!
//! O batimento vem de um timer que roda sempre, independente de desenhar,
//! para o sono profundo (zero commits) e a tela apagada não contarem como
//! travamento.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::estado::Compartilhado;

pub const LIMITE: Duration = Duration::from_secs(60);
const INTERVALO: Duration = Duration::from_secs(5);

pub fn iniciar(comp: Arc<Compartilhado>) {
    let criada = thread::Builder::new().name("vigia".into()).spawn(move || {
        loop {
            thread::sleep(INTERVALO);
            let idade = comp.idade_batimento();
            if idade > LIMITE {
                erro!(
                    "laço principal sem batimento há {} s; abortando para o Docker reiniciar",
                    idade.as_secs()
                );
                std::process::abort();
            }
        }
    });
    if let Err(e) = criada {
        aviso!("vigia não iniciou: {e}");
    }
}
