use server::{config::Config, db, montar_app, ratelimit, state::AppState};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;
    let db = db::create_pool(&config.database_url).await?;
    tracing::info!("migrations aplicadas; banco pronto");

    let (ranking_tx, _) = tokio::sync::broadcast::channel::<String>(100);
    let port = config.port;
    let state = AppState {
        db,
        config,
        ranking_tx,
        palpite_limiter: ratelimit::novo(),
    };

    let app = montar_app(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    // Abra no navegador por localhost (e não 0.0.0.0, que alguns navegadores
    // tratam de forma diferente e podem bloquear storage/requisições).
    tracing::info!("Servidor rodando em http://localhost:{port}  (admin em /admin)");
    // `into_make_service_with_connect_info` expõe o IP do cliente (ConnectInfo)
    // usado pelo rate-limit de palpites.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
