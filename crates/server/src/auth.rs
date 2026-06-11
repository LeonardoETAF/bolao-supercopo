use crate::errors::AppError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

fn papel_padrao() -> String {
    "admin".to_string()
}

/// Claims do token JWT do administrador.
#[derive(Debug, Serialize, Deserialize)]
pub struct AdminClaims {
    pub sub: String,
    /// Papel: "admin" (acesso total) ou "viewer" (somente leitura de Métricas/Leads).
    #[serde(default = "papel_padrao")]
    pub role: String,
    pub exp: usize,
}

impl AdminClaims {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// Gera um token JWT válido por 24h, com o papel do usuário.
pub fn gerar_token(usuario: &str, role: &str, secret: &str) -> anyhow::Result<String> {
    let exp = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;
    let claims = AdminClaims {
        sub: usuario.to_string(),
        role: role.to_string(),
        exp,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

/// Extrator que exige um `Authorization: Bearer <jwt>` válido.
/// Usado como argumento nas rotas administrativas protegidas.
#[axum::async_trait]
impl FromRequestParts<AppState> for AdminClaims {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::NaoAutorizado)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::NaoAutorizado)?;

        let dados = decode::<AdminClaims>(
            token,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::NaoAutorizado)?;

        Ok(dados.claims)
    }
}

/// Extrator que exige um admin com **acesso total** (role = "admin").
/// Usuários "viewer" recebem 401 nessas rotas.
pub struct AdminFull(pub AdminClaims);

#[axum::async_trait]
impl FromRequestParts<AppState> for AdminFull {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = AdminClaims::from_request_parts(parts, state).await?;
        if !claims.is_admin() {
            return Err(AppError::NaoAutorizado);
        }
        Ok(AdminFull(claims))
    }
}
