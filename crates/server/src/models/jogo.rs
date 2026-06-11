use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Jogo {
    pub id: Uuid,
    pub time_a: String,
    pub time_b: String,
    pub data_jogo: DateTime<Utc>,
    pub placar_a: Option<i16>,
    pub placar_b: Option<i16>,
    pub status: String,
    pub ativo: bool,
    pub criado_em: DateTime<Utc>,
    /// Bandeira (arquivo/URL em static/img) escolhida no cadastro; opcional.
    pub bandeira_a: Option<String>,
    pub bandeira_b: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CriarJogoRequest {
    pub time_a: String,
    pub time_b: String,
    pub data_jogo: DateTime<Utc>,
    #[serde(default)]
    pub ativo: bool,
    #[serde(default)]
    pub bandeira_a: Option<String>,
    #[serde(default)]
    pub bandeira_b: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResultadoRequest {
    pub gols_time_a: i16,
    pub gols_time_b: i16,
}
