use crate::errors::AppError;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Limitador simples em memória: registra os horários das requisições por IP.
pub type Limiter = Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>;

const JANELA: Duration = Duration::from_secs(60);
const MAX_POR_JANELA: usize = 5;

pub fn novo() -> Limiter {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Permite no máximo `MAX_POR_JANELA` chamadas por IP a cada `JANELA`.
/// O lock é segurado apenas em código síncrono (sem `await`), então é seguro.
pub fn checar(limiter: &Limiter, ip: IpAddr) -> Result<(), AppError> {
    let agora = Instant::now();
    let mut mapa = limiter.lock().unwrap();
    let registros = mapa.entry(ip).or_default();
    registros.retain(|t| agora.duration_since(*t) < JANELA);

    if registros.len() >= MAX_POR_JANELA {
        return Err(AppError::MuitasRequisicoes);
    }
    registros.push(agora);
    Ok(())
}
