use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Usuario {
    pub id: Uuid,
    pub nome: String,
    pub telefone: String,
    pub email: String,
    pub data_cadastro: DateTime<Utc>,
}

/// Payload do formulário de palpite (inclui os dados do usuário).
#[derive(Debug, Deserialize)]
pub struct CriarPalpiteRequest {
    pub nome: String,
    pub telefone: String,
    pub email: String,
    pub jogo_id: Uuid,
    pub gols_time_a: i16,
    pub gols_time_b: i16,
}
