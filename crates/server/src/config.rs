use anyhow::Context;

/// Configuração lida das variáveis de ambiente (.env).
#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub admin_user: String,
    pub admin_pass: String,
    /// Usuário de acesso restrito (somente leitura de Métricas e Leads).
    pub viewer_user: String,
    pub viewer_pass: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL não está definida")?,
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev_secret_inseguro".to_string()),
            admin_user: std::env::var("ADMIN_USER").unwrap_or_else(|_| "admin".to_string()),
            admin_pass: std::env::var("ADMIN_PASS").unwrap_or_else(|_| "admin".to_string()),
            viewer_user: std::env::var("VIEWER_USER").unwrap_or_else(|_| "leads".to_string()),
            viewer_pass: std::env::var("VIEWER_PASS").unwrap_or_else(|_| "leads".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
        })
    }
}
