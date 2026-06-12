use crate::errors::AppError;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Limitador simples em memória: registra os horários das requisições por IP.
pub type Limiter = Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>;

const JANELA: Duration = Duration::from_secs(60);
const MAX_POR_JANELA: usize = 5;

/// Limites do login admin: mais permissivo que palpites (usuário pode errar a
/// senha algumas vezes), porém suficiente para barrar força bruta.
const LOGIN_JANELA: Duration = Duration::from_secs(60);
const LOGIN_MAX_POR_JANELA: usize = 10;

pub fn novo() -> Limiter {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Permite no máximo `MAX_POR_JANELA` chamadas por IP a cada `JANELA` (palpites).
pub fn checar(limiter: &Limiter, ip: IpAddr) -> Result<(), AppError> {
    checar_com(limiter, ip, MAX_POR_JANELA, JANELA)
}

/// Rate-limit do login admin (anti força bruta).
pub fn checar_login(limiter: &Limiter, ip: IpAddr) -> Result<(), AppError> {
    checar_com(limiter, ip, LOGIN_MAX_POR_JANELA, LOGIN_JANELA)
}

/// Núcleo do limitador, parametrizável. O lock é segurado apenas em código
/// síncrono (sem `await`), então é seguro.
///
/// A cada chamada faz uma limpeza oportunista do mapa inteiro, descartando IPs
/// cujos registros já expiraram — assim o `HashMap` não cresce indefinidamente.
pub fn checar_com(
    limiter: &Limiter,
    ip: IpAddr,
    max: usize,
    janela: Duration,
) -> Result<(), AppError> {
    let agora = Instant::now();
    let mut mapa = limiter.lock().unwrap_or_else(|e| e.into_inner());

    // Limpeza: remove registros expirados e IPs que ficaram sem registros.
    mapa.retain(|_, regs| {
        regs.retain(|t| agora.duration_since(*t) < janela);
        !regs.is_empty()
    });

    let registros = mapa.entry(ip).or_default();
    if registros.len() >= max {
        return Err(AppError::MuitasRequisicoes);
    }
    registros.push(agora);
    Ok(())
}
