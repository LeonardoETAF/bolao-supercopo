//! Testes de integração das rotas HTTP (via `oneshot`, sem subir servidor).
//!
//! Eles tocam o banco real, então exigem `DATABASE_URL` definido — caso
//! contrário são **pulados** (não falham). Para rodar:
//!
//! ```bash
//! DATABASE_URL=postgres://copa:copa_secret@localhost:5432/copa \
//!   cargo test -p server --test integration
//! ```

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower::ServiceExt; // oneshot

use server::config::Config;
use server::state::AppState;

/// Constrói um AppState de teste (creds admin conhecidas). Retorna `None`
/// se `DATABASE_URL` não estiver definido (teste é pulado).
async fn setup() -> Option<AppState> {
    let database_url = std::env::var("DATABASE_URL").ok()?;
    let db = server::db::create_pool(&database_url).await.ok()?;
    let (ranking_tx, _) = tokio::sync::broadcast::channel(100);
    let config = Config {
        database_url,
        jwt_secret: "segredo_de_teste".to_string(),
        admin_user: "admin".to_string(),
        admin_pass: "admin".to_string(),
        viewer_user: "leads".to_string(),
        viewer_pass: "leads".to_string(),
        port: 0,
    };
    Some(AppState {
        db,
        config,
        ranking_tx,
        palpite_limiter: server::ratelimit::novo(),
        login_limiter: server::ratelimit::novo(),
    })
}

/// Faz uma requisição no app e devolve (status, corpo JSON).
async fn req(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let mut request = builder.body(body).unwrap();
    // Injeta o IP do cliente que o rate-limit espera (ConnectInfo).
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 9000))));

    let resp = app.clone().oneshot(request).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let valor = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, valor)
}

#[tokio::test]
async fn fluxo_completo() {
    let Some(state) = setup().await else {
        eprintln!("DATABASE_URL não definido — pulando fluxo_completo");
        return;
    };
    let app = server::montar_app(state.clone());

    // E-mail de teste improvável de colidir com dados reais.
    const EMAIL: &str = "teste_integracao@email.com";

    // Limpa qualquer resíduo de execuções anteriores.
    let _ = sqlx::query("DELETE FROM usuarios WHERE email = $1")
        .bind(EMAIL)
        .execute(&state.db)
        .await;

    // 1. Login admin.
    let (st, v) = req(
        &app,
        "POST",
        "/admin/login",
        None,
        Some(json!({"usuario": "admin", "senha": "admin"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "login deveria dar 200");
    let token = v["token"].as_str().unwrap().to_string();

    // Login errado -> 401.
    let (st, _) = req(
        &app,
        "POST",
        "/admin/login",
        None,
        Some(json!({"usuario": "admin", "senha": "errada"})),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // 2. Cadastrar jogo ativo no futuro.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time_a = format!("TEST_A_{ts}");
    let time_b = format!("TEST_B_{ts}");
    let (st, jv) = req(
        &app,
        "POST",
        "/admin/jogos",
        Some(&token),
        Some(json!({
            "time_a": time_a, "time_b": time_b,
            "data_jogo": "2030-01-01T00:00:00Z", "ativo": true
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let jogo_id = jv["id"].as_str().unwrap().to_string();

    // 3. E-mail inválido -> 400.
    let (st, _) = req(
        &app,
        "POST",
        "/api/palpite",
        None,
        Some(json!({
            "nome": "Teste Int", "telefone": "11999990000",
            "email": "sem-arroba", "jogo_id": jogo_id,
            "gols_time_a": 1, "gols_time_b": 0
        })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // 4. Palpite válido -> 200 + cupom de participação (percentual configurável).
    let palpite = json!({
        "nome": "Teste Int", "telefone": "11999990000",
        "email": EMAIL, "jogo_id": jogo_id,
        "gols_time_a": 2, "gols_time_b": 1
    });
    let (st, pv) = req(&app, "POST", "/api/palpite", None, Some(palpite.clone())).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(pv["sucesso"], json!(true));
    let desconto_participacao = pv["cupom"]["desconto"].as_str().unwrap().to_string();
    assert!(desconto_participacao.ends_with('%'), "desconto deve ser um percentual");

    // 5. Palpite duplicado -> 409.
    let (st, _) = req(&app, "POST", "/api/palpite", None, Some(palpite)).await;
    assert_eq!(st, StatusCode::CONFLICT);

    // 6. Meus cupons por e-mail -> contém o cupom de participação gerado.
    let (st, cv) = req(
        &app,
        "GET",
        &format!("/api/meus-cupons?email={EMAIL}"),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(cv
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["tipo"] == json!(desconto_participacao)));

    // 7. Informar resultado exato -> pontua e gera cupom de 30%.
    let (st, rv) = req(
        &app,
        "PUT",
        &format!("/admin/jogos/{jogo_id}/resultado"),
        Some(&token),
        Some(json!({"gols_time_a": 2, "gols_time_b": 1})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(rv["cupons_30"], json!(1));

    // Limpeza: remove o jogo (cascata em palpites) e o usuário de teste.
    let _ = sqlx::query("DELETE FROM jogos WHERE id = $1::uuid")
        .bind(&jogo_id)
        .execute(&state.db)
        .await;
    let _ = sqlx::query("DELETE FROM usuarios WHERE email = $1")
        .bind(EMAIL)
        .execute(&state.db)
        .await;
}

#[tokio::test]
async fn admin_exige_token() {
    let Some(state) = setup().await else {
        eprintln!("DATABASE_URL não definido — pulando admin_exige_token");
        return;
    };
    let app = server::montar_app(state);

    let (st, _) = req(&app, "GET", "/admin/metricas", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}
