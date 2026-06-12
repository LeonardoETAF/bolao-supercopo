use crate::errors::AppError;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RankingParams {
    pub page: Option<i64>,
    /// Busca opcional por nome do participante.
    pub q: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct RankRow {
    nome: String,
    total_pontos: i64,
    total_palpites: i64,
    acertos_exatos: i64,
    ultimo_palpite: Option<String>,
    posicao: i64,
}

#[derive(Debug, Serialize)]
pub struct Participante {
    pub posicao: i64,
    pub nome: String,
    pub total_pontos: i64,
    pub total_palpites: i64,
    pub acertos_exatos: i64,
    /// Placar do palpite mais recente, ex.: "2x1" (ou null se nunca palpitou).
    pub ultimo_palpite: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RankingResponse {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
    pub participantes: Vec<Participante>,
}

/// GET /api/ranking?page=N&q=texto — ranking paginado (20 por página) com busca por nome.
/// A posição é sempre a do ranking global (não muda ao filtrar pela busca).
/// Ordenação: maior pontuação primeiro; em empate, quem palpitou por ÚLTIMO
/// (palpite mais recente) fica à frente; por fim, nome (desempate estável).
pub async fn ranking_completo(
    State(state): State<AppState>,
    Query(params): Query<RankingParams>,
) -> Result<Json<RankingResponse>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = 20i64;
    let offset = (page - 1) * per_page;
    let busca = params.q.unwrap_or_default().trim().to_string();

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM usuarios u
         WHERE ($1 = ''
                OR u.nome ILIKE '%' || $1 || '%'
                OR u.telefone ILIKE '%' || $1 || '%'
                OR u.email ILIKE '%' || $1 || '%')",
    )
    .bind(&busca)
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query_as::<_, RankRow>(
        r#"
        WITH classificados AS (
            SELECT
                u.nome,
                u.telefone,
                u.email,
                COALESCE(SUM(p.pontuacao), 0)::BIGINT                   AS total_pontos,
                COUNT(p.id)::BIGINT                                     AS total_palpites,
                COUNT(*) FILTER (WHERE p.pontuacao = 10)::BIGINT        AS acertos_exatos,
                (SELECT pp.gols_time_a::text || 'x' || pp.gols_time_b::text
                   FROM palpites pp WHERE pp.usuario_id = u.id
                   ORDER BY pp.criado_em DESC LIMIT 1)                  AS ultimo_palpite,
                ROW_NUMBER() OVER (
                    ORDER BY
                        COALESCE(SUM(p.pontuacao), 0) DESC,
                        MAX(p.criado_em) DESC NULLS LAST,
                        u.nome ASC
                )::BIGINT                                               AS posicao
            FROM usuarios u
            LEFT JOIN palpites p ON p.usuario_id = u.id
            GROUP BY u.id, u.nome, u.telefone, u.email
        )
        SELECT nome, total_pontos, total_palpites, acertos_exatos, ultimo_palpite, posicao
        FROM classificados
        WHERE ($1 = ''
               OR nome ILIKE '%' || $1 || '%'
               OR telefone ILIKE '%' || $1 || '%'
               OR email ILIKE '%' || $1 || '%')
        ORDER BY posicao
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&busca)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let participantes = rows
        .into_iter()
        .map(|r| Participante {
            posicao: r.posicao,
            nome: r.nome,
            total_pontos: r.total_pontos,
            total_palpites: r.total_palpites,
            acertos_exatos: r.acertos_exatos,
            ultimo_palpite: r.ultimo_palpite,
        })
        .collect();

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as i64;

    Ok(Json(RankingResponse {
        page,
        per_page,
        total,
        total_pages: total_pages.max(1),
        participantes,
    }))
}
