use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Configuração da landing page editável pelo painel `/config`.
/// Guardada como um único blob JSON na tabela `config` (chave `landing_config`).

/// Link de uma rede social: ligado/desligado + URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeSocial {
    pub ativo: bool,
    pub url: String,
}

impl RedeSocial {
    fn novo(ativo: bool, url: &str) -> Self {
        Self {
            ativo,
            url: url.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingConfig {
    // Confronto
    pub time1_nome: String,
    pub time2_nome: String,
    pub data: String,    // ex.: "20/06/2026"
    pub horario: String, // ex.: "16h00"

    /// Jogo da tabela `jogos` vinculado a este confronto (cadastro do jogo).
    /// Preenchido pelo servidor ao salvar; usado para atualizar o mesmo jogo.
    #[serde(default)]
    pub jogo_id: Option<String>,

    // Resultado oficial
    pub placar_time1: i32,
    pub placar_time2: i32,
    #[serde(default)]
    pub divulgar_resultado: bool,

    // Cupons — apenas o percentual de desconto; o código é gerado
    // aleatoriamente e único por cupom no momento da emissão.
    pub cupom_participacao_desconto: i32,
    pub cupom_acerto_desconto: i32,

    // Redes sociais
    pub instagram: RedeSocial,
    pub facebook: RedeSocial,
    pub tiktok: RedeSocial,
    pub whatsapp: RedeSocial,
    pub youtube: RedeSocial,
}

impl Default for LandingConfig {
    fn default() -> Self {
        Self {
            time1_nome: "Brasil".to_string(),
            time2_nome: "Marrocos".to_string(),
            data: String::new(),
            horario: String::new(),
            jogo_id: None,
            placar_time1: 0,
            placar_time2: 0,
            divulgar_resultado: false,
            cupom_participacao_desconto: 10,
            cupom_acerto_desconto: 30,
            instagram: RedeSocial::novo(true, "https://instagram.com/supercopo"),
            facebook: RedeSocial::novo(true, "https://facebook.com/supercopo"),
            tiktok: RedeSocial::novo(false, "https://tiktok.com/@supercopo"),
            whatsapp: RedeSocial::novo(true, "https://wa.me/55..."),
            youtube: RedeSocial::novo(false, "https://youtube.com/@supercopo"),
        }
    }
}

/// Lê a configuração da landing; devolve os defaults se ainda não houver registro.
pub async fn carregar(db: &PgPool) -> Result<LandingConfig, sqlx::Error> {
    let valor: Option<String> =
        sqlx::query_scalar("SELECT valor FROM config WHERE chave = 'landing_config'")
            .fetch_optional(db)
            .await?;
    Ok(valor
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default())
}

/// Grava a configuração da landing (upsert na chave `landing_config`).
pub async fn salvar(db: &PgPool, cfg: &LandingConfig) -> Result<(), sqlx::Error> {
    let json = serde_json::to_string(cfg).unwrap_or_else(|_| "{}".to_string());
    sqlx::query(
        "INSERT INTO config (chave, valor) VALUES ('landing_config', $1)
         ON CONFLICT (chave) DO UPDATE SET valor = EXCLUDED.valor",
    )
    .bind(json)
    .execute(db)
    .await?;
    Ok(())
}
