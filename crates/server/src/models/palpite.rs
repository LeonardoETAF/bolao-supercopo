use super::cupom::CupomInfo;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Palpite {
    pub id: Uuid,
    pub usuario_id: Uuid,
    pub jogo_id: Uuid,
    pub gols_time_a: i16,
    pub gols_time_b: i16,
    pub pontuacao: i16,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PalpiteResponse {
    pub sucesso: bool,
    pub mensagem: String,
    pub cupom: Option<CupomInfo>,
}
