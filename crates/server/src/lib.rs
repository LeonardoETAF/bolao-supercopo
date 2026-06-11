pub mod auth;
pub mod bolao;
pub mod config;
pub mod db;
pub mod landing;
pub mod errors;
pub mod models;
pub mod ratelimit;
pub mod routes;
pub mod state;
pub mod validacao;

use axum::http::header::CACHE_CONTROL;
use axum::http::HeaderValue;
use axum::routing::{delete, get, post, put};
use axum::Router;
use state::AppState;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// Monta o `Router` da aplicação com todas as rotas e camadas.
/// Separado do `main` para permitir testes de integração (oneshot).
pub fn montar_app(state: AppState) -> Router {
    Router::new()
        // Páginas
        .route("/", get(routes::paginas::index))
        .route("/ranking", get(routes::paginas::ranking_page))
        .route("/admin", get(routes::paginas::painel))
        // API pública
        .route("/api/palpite", post(routes::palpites::enviar_palpite))
        .route("/api/jogo-ativo", get(routes::palpites::jogo_ativo))
        .route("/api/meus-cupons", get(routes::palpites::meus_cupons))
        .route("/api/ranking", get(routes::ranking::ranking_completo))
        .route("/api/ranking/stream", get(routes::sse::ranking_stream))
        // Admin (protegido por JWT, exceto o login)
        .route("/admin/login", post(routes::admin::login))
        .route(
            "/admin/jogos",
            post(routes::admin::cadastrar_jogo).get(routes::admin::listar_jogos),
        )
        .route(
            "/admin/jogos/:id",
            delete(routes::admin::deletar_jogo).put(routes::admin::editar_jogo),
        )
        .route("/admin/jogos/:id/ativar", put(routes::admin::ativar_jogo))
        .route("/admin/jogos/:id/desativar", put(routes::admin::desativar_jogo))
        .route("/admin/bandeiras", get(routes::admin::listar_bandeiras))
        .route(
            "/admin/jogos/:id/resultado",
            put(routes::admin::informar_resultado),
        )
        .route("/admin/metricas", get(routes::admin::metricas))
        .route("/admin/cupons", get(routes::admin::listar_cupons))
        .route("/admin/cupons/:id/utilizar", put(routes::admin::marcar_cupom))
        .route(
            "/admin/landing",
            get(routes::admin::obter_landing).put(routes::admin::salvar_landing),
        )
        // Estáticos (sem cache agressivo: o navegador revalida sempre)
        .nest_service(
            "/static",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    CACHE_CONTROL,
                    HeaderValue::from_static("no-cache"),
                ))
                .service(ServeDir::new("static")),
        )
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}
