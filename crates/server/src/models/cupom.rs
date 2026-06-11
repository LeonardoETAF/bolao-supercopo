use chrono::{DateTime, Utc};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Cupom {
    pub id: Uuid,
    pub usuario_id: Uuid,
    pub tipo: String,
    pub codigo: String,
    pub utilizado: bool,
    pub criado_em: DateTime<Utc>,
    /// Jogo do palpite que gerou o cupom (nulo para cupons antigos).
    pub jogo_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CupomInfo {
    pub codigo: String,
    pub desconto: String,
}

/// Gera um código de cupom único no formato `COPA-XXXXXXXXXXXX`.
pub fn gerar_codigo() -> String {
    let sufixo: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect::<String>()
        .to_uppercase();
    format!("COPA-{}", sufixo)
}
