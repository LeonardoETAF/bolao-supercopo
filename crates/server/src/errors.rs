use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Erro central da aplicação. Sempre vira uma resposta JSON `{ "erro": "..." }`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validacao(String),

    #[error("Nenhum jogo está ativo para palpites no momento")]
    JogoNaoAtivo,

    #[error("Este jogo já começou; os palpites estão encerrados")]
    JogoEncerrado,

    #[error("O bolão está encerrado; não é possível enviar novos palpites")]
    BolaoEncerrado,

    #[error("Muitas tentativas em pouco tempo. Aguarde um instante e tente novamente")]
    MuitasRequisicoes,

    #[error("Você já enviou um palpite para este jogo")]
    PalpiteDuplicado,

    #[error("E-mail inválido")]
    EmailInvalido,

    #[error("Credenciais inválidas")]
    NaoAutorizado,

    #[error("Recurso não encontrado")]
    NaoEncontrado,

    #[error("Erro no banco de dados")]
    Db(#[from] sqlx::Error),

    #[error("Erro interno do servidor")]
    Interno(#[from] anyhow::Error),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::Validacao(_)
            | AppError::EmailInvalido
            | AppError::JogoNaoAtivo
            | AppError::JogoEncerrado
            | AppError::BolaoEncerrado => StatusCode::BAD_REQUEST,
            AppError::MuitasRequisicoes => StatusCode::TOO_MANY_REQUESTS,
            AppError::PalpiteDuplicado => StatusCode::CONFLICT,
            AppError::NaoAutorizado => StatusCode::UNAUTHORIZED,
            AppError::NaoEncontrado => StatusCode::NOT_FOUND,
            AppError::Db(_) | AppError::Interno(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("erro interno: {:?}", self);
        }
        (status, Json(json!({ "erro": self.to_string() }))).into_response()
    }
}
