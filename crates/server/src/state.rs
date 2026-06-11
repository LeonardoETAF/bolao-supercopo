use crate::config::Config;
use crate::ratelimit::Limiter;
use sqlx::PgPool;
use tokio::sync::broadcast;

/// Estado compartilhado entre todas as rotas.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    /// Canal de notificação para o SSE do ranking. Qualquer mudança que afete
    /// a classificação envia uma mensagem aqui para os clientes recarregarem.
    pub ranking_tx: broadcast::Sender<String>,
    /// Limitador de envio de palpites por IP (anti-spam).
    pub palpite_limiter: Limiter,
}
