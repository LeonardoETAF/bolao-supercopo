use sqlx::{postgres::PgPoolOptions, PgPool};

/// Cria o pool de conexões e aplica as migrations (embutidas no binário).
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;

    Ok(pool)
}
