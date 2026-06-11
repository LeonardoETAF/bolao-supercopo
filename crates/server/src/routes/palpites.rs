use crate::errors::AppError;
use crate::models::{gerar_codigo, CriarPalpiteRequest, CupomInfo, Jogo, PalpiteResponse, Usuario};
use crate::ratelimit;
use crate::state::AppState;
use crate::validacao::{somente_digitos, validar_email};
use axum::extract::{ConnectInfo, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// POST /api/palpite — registra um palpite e gera o cupom de participação.
pub async fn enviar_palpite(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<CriarPalpiteRequest>,
) -> Result<Json<PalpiteResponse>, AppError> {
    // 0. Anti-spam por IP.
    ratelimit::checar(&state.palpite_limiter, addr.ip())?;

    // 0b. Bolão encerrado não aceita novos palpites.
    if crate::bolao::esta_encerrado(&state.db).await? {
        return Err(AppError::BolaoEncerrado);
    }

    // 1. Validações de campos.
    let nome = req.nome.trim().to_string();
    if nome.chars().count() < 3 {
        return Err(AppError::Validacao(
            "Nome deve ter no mínimo 3 caracteres".to_string(),
        ));
    }

    let telefone = somente_digitos(&req.telefone);
    if telefone.len() < 10 || telefone.len() > 11 {
        return Err(AppError::Validacao("Telefone inválido".to_string()));
    }

    if !(0..=20).contains(&req.gols_time_a) || !(0..=20).contains(&req.gols_time_b) {
        return Err(AppError::Validacao(
            "Placar deve estar entre 0 e 20".to_string(),
        ));
    }

    // 2. Validação de e-mail.
    let email = validar_email(&req.email)?;

    // 3. Verificar se há um jogo ativo correspondente.
    let jogo = sqlx::query_as::<_, Jogo>(
        "SELECT * FROM jogos WHERE id = $1 AND ativo = TRUE AND status = 'ativo'",
    )
    .bind(req.jogo_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::JogoNaoAtivo)?;

    // 3b. Não aceitar palpites após o horário de início do jogo.
    if jogo.data_jogo <= chrono::Utc::now() {
        return Err(AppError::JogoEncerrado);
    }

    // 3c. E-mail e telefone só podem ser usados uma vez por jogo.
    let ja_usado: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM palpites p
         JOIN usuarios u ON u.id = p.usuario_id
         WHERE p.jogo_id = $1 AND (u.email = $2 OR u.telefone = $3)",
    )
    .bind(jogo.id)
    .bind(&email)
    .bind(&telefone)
    .fetch_one(&state.db)
    .await?;

    if ja_usado > 0 {
        return Err(AppError::PalpiteDuplicado);
    }

    // 4. Buscar ou criar o usuário pelo e-mail (chave única).
    let usuario = sqlx::query_as::<_, Usuario>(
        "INSERT INTO usuarios (nome, telefone, email) VALUES ($1, $2, $3)
         ON CONFLICT (email) DO UPDATE SET nome = EXCLUDED.nome, telefone = EXCLUDED.telefone
         RETURNING *",
    )
    .bind(&nome)
    .bind(&telefone)
    .bind(&email)
    .fetch_one(&state.db)
    .await?;

    // 5. Inserir o palpite.
    sqlx::query(
        "INSERT INTO palpites (usuario_id, jogo_id, gols_time_a, gols_time_b)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(usuario.id)
    .bind(jogo.id)
    .bind(req.gols_time_a)
    .bind(req.gols_time_b)
    .execute(&state.db)
    .await?;

    // 7. Gerar cupom de participação — desconto configurado no painel,
    // código aleatório/único e vinculado a este jogo.
    let cfg = crate::landing::carregar(&state.db).await?;
    let desconto = format!("{}%", cfg.cupom_participacao_desconto);
    let codigo = gerar_codigo();
    sqlx::query("INSERT INTO cupons (usuario_id, jogo_id, tipo, codigo) VALUES ($1, $2, $3, $4)")
        .bind(usuario.id)
        .bind(jogo.id)
        .bind(&desconto)
        .bind(&codigo)
        .execute(&state.db)
        .await?;

    // 8. Notificar o ranking ao vivo (SSE).
    let _ = state.ranking_tx.send("atualizar".to_string());

    tracing::info!(usuario = %usuario.id, jogo = %jogo.id, "palpite registrado");

    Ok(Json(PalpiteResponse {
        sucesso: true,
        mensagem: "Palpite registrado com sucesso! 🎉".to_string(),
        cupom: Some(CupomInfo { codigo, desconto }),
    }))
}

/// GET /api/jogo-ativo — retorna o jogo atualmente aberto para palpites (ou null).
/// Só considera jogos ativos cujo horário ainda não chegou (consistente com a
/// home e com a validação de envio de palpite).
pub async fn jogo_ativo(
    State(state): State<AppState>,
) -> Result<Json<Option<Jogo>>, AppError> {
    let jogo = sqlx::query_as::<_, Jogo>(
        "SELECT * FROM jogos
         WHERE ativo = TRUE AND status = 'ativo' AND data_jogo > NOW()
         ORDER BY data_jogo ASC LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(jogo))
}

#[derive(Debug, Deserialize)]
pub struct MeusCuponsParams {
    pub email: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CupomPublico {
    pub tipo: String,
    pub codigo: String,
    pub utilizado: bool,
}

/// GET /api/meus-cupons?email=... — lista os cupons de um participante pelo e-mail.
pub async fn meus_cupons(
    State(state): State<AppState>,
    Query(params): Query<MeusCuponsParams>,
) -> Result<Json<Vec<CupomPublico>>, AppError> {
    let email = validar_email(&params.email)?;

    let cupons = sqlx::query_as::<_, CupomPublico>(
        "SELECT c.tipo, c.codigo, c.utilizado
         FROM cupons c
         JOIN usuarios u ON u.id = c.usuario_id
         WHERE u.email = $1
         ORDER BY c.criado_em DESC",
    )
    .bind(&email)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(cupons))
}
