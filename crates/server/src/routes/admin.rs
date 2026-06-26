use crate::auth::{gerar_token, AdminClaims, AdminFull};
use crate::errors::AppError;
use crate::landing::LandingConfig;
use crate::models::{gerar_codigo, CriarJogoRequest, Cupom, Jogo, Palpite, ResultadoRequest};
use crate::routes::calcular_pontos;
use crate::ratelimit;
use crate::state::AppState;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub usuario: String,
    pub senha: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    /// "admin" (acesso total) ou "viewer" (somente leitura).
    pub role: String,
}

/// POST /admin/login — autentica com as credenciais do .env e devolve um JWT (24h).
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // Anti força bruta: limita tentativas de login por IP.
    ratelimit::checar_login(&state.login_limiter, addr.ip())?;

    let role = if req.usuario == state.config.admin_user && req.senha == state.config.admin_pass {
        "admin"
    } else if req.usuario == state.config.viewer_user && req.senha == state.config.viewer_pass {
        "viewer"
    } else {
        return Err(AppError::NaoAutorizado);
    };
    let token = gerar_token(&req.usuario, role, &state.config.jwt_secret)?;
    Ok(Json(TokenResponse {
        token,
        role: role.to_string(),
    }))
}

/// POST /admin/jogos — cadastra um novo jogo.
pub async fn cadastrar_jogo(
    State(state): State<AppState>,
    _claims: AdminFull,
    Json(req): Json<CriarJogoRequest>,
) -> Result<Json<Jogo>, AppError> {
    let status = if req.ativo { "ativo" } else { "agendado" };

    // Vários jogos podem ficar abertos ao mesmo tempo.
    let jogo = sqlx::query_as::<_, Jogo>(
        "INSERT INTO jogos (time_a, time_b, data_jogo, status, ativo, bandeira_a, bandeira_b)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
    )
    .bind(&req.time_a)
    .bind(&req.time_b)
    .bind(req.data_jogo)
    .bind(status)
    .bind(req.ativo)
    .bind(&req.bandeira_a)
    .bind(&req.bandeira_b)
    .fetch_one(&state.db)
    .await?;

    let _ = state.ranking_tx.send("atualizar".to_string());
    Ok(Json(jogo))
}

/// PUT /admin/jogos/:id/ativar — torna o jogo o único ativo para palpites.
pub async fn ativar_jogo(
    State(state): State<AppState>,
    _claims: AdminFull,
    Path(jogo_id): Path<Uuid>,
) -> Result<Json<Jogo>, AppError> {
    let jogo = sqlx::query_as::<_, Jogo>(
        "UPDATE jogos SET ativo = TRUE, status = 'ativo' WHERE id = $1 RETURNING *",
    )
    .bind(jogo_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NaoEncontrado)?;

    Ok(Json(jogo))
}

/// PUT /admin/jogos/:id/desativar — tira o jogo do ar (não aceita mais palpites).
/// Não mexe em jogos já encerrados.
pub async fn desativar_jogo(
    State(state): State<AppState>,
    _claims: AdminFull,
    Path(jogo_id): Path<Uuid>,
) -> Result<Json<Jogo>, AppError> {
    let jogo = sqlx::query_as::<_, Jogo>(
        "UPDATE jogos SET ativo = FALSE,
                status = CASE WHEN status = 'encerrado' THEN status ELSE 'agendado' END
         WHERE id = $1 RETURNING *",
    )
    .bind(jogo_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NaoEncontrado)?;

    let _ = state.ranking_tx.send("atualizar".to_string());
    Ok(Json(jogo))
}

#[derive(Debug, Serialize)]
pub struct ResultadoResponse {
    pub processados: usize,
    pub cupons_30: usize,
}

/// PUT /admin/jogos/:id/resultado — informa o placar e recalcula a pontuação de todos
/// os palpites do jogo, gerando cupom de 30% para quem acertou o placar exato.
pub async fn informar_resultado(
    State(state): State<AppState>,
    _claims: AdminFull,
    Path(jogo_id): Path<Uuid>,
    Json(req): Json<ResultadoRequest>,
) -> Result<Json<ResultadoResponse>, AppError> {
    // 0. Valida o placar informado (mesma faixa dos palpites).
    if !(0..=20).contains(&req.gols_time_a) || !(0..=20).contains(&req.gols_time_b) {
        return Err(AppError::Validacao(
            "Placar deve estar entre 0 e 20".to_string(),
        ));
    }

    let cfg = crate::landing::carregar(&state.db).await?;
    let desconto_acerto = format!("{}%", cfg.cupom_acerto_desconto);
    let desconto_participacao = format!("{}%", cfg.cupom_participacao_desconto);

    // Tudo numa transação: ou o resultado é aplicado por completo, ou nada muda.
    let mut tx = state.db.begin().await?;

    // 1. Atualiza o placar e encerra o jogo.
    let atualizado = sqlx::query(
        "UPDATE jogos SET placar_a = $1, placar_b = $2, status = 'encerrado', ativo = FALSE
         WHERE id = $3",
    )
    .bind(req.gols_time_a)
    .bind(req.gols_time_b)
    .bind(jogo_id)
    .execute(&mut *tx)
    .await?;

    if atualizado.rows_affected() == 0 {
        return Err(AppError::NaoEncontrado);
    }

    // 2. Busca os palpites do jogo.
    let palpites = sqlx::query_as::<_, Palpite>("SELECT * FROM palpites WHERE jogo_id = $1")
        .bind(jogo_id)
        .fetch_all(&mut *tx)
        .await?;

    // 3. Reinformar/corrigir o resultado deve ser idempotente: zera tudo para o
    // cupom de participação e, em seguida, promove apenas quem cravou o placar
    // agora — assim quem deixou de cravar (numa correção) não fica com 30%.
    sqlx::query("UPDATE cupons SET tipo = $1 WHERE jogo_id = $2")
        .bind(&desconto_participacao)
        .bind(jogo_id)
        .execute(&mut *tx)
        .await?;

    // 4. Calcula a pontuação de cada palpite. Quem crava o placar (10 pts) tem o
    // cupom de participação PROMOVIDO ao de acerto (substitui o desconto), com o
    // percentual configurado no painel. Quem acerta parcial (5) ou erra (0) mantém
    // o cupom de participação.
    let mut cupons_30 = 0usize;
    for palpite in &palpites {
        let pontos = calcular_pontos(
            palpite.gols_time_a,
            palpite.gols_time_b,
            req.gols_time_a,
            req.gols_time_b,
        );

        sqlx::query("UPDATE palpites SET pontuacao = $1 WHERE id = $2")
            .bind(pontos)
            .bind(palpite.id)
            .execute(&mut *tx)
            .await?;

        if pontos == 10 {
            // Promove o cupom de participação deste jogo ao de acerto.
            let atualizados = sqlx::query(
                "UPDATE cupons SET tipo = $1 WHERE usuario_id = $2 AND jogo_id = $3",
            )
            .bind(&desconto_acerto)
            .bind(palpite.usuario_id)
            .bind(jogo_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

            // Sem cupom de participação registrado (caso raro): cria o de acerto.
            if atualizados == 0 {
                sqlx::query(
                    "INSERT INTO cupons (usuario_id, jogo_id, tipo, codigo) VALUES ($1, $2, $3, $4)",
                )
                .bind(palpite.usuario_id)
                .bind(jogo_id)
                .bind(&desconto_acerto)
                .bind(gerar_codigo())
                .execute(&mut *tx)
                .await?;
            }
            cupons_30 += 1;
        }
    }

    tx.commit().await?;

    // 5. Notifica o ranking ao vivo.
    let _ = state.ranking_tx.send("atualizar".to_string());

    tracing::info!(jogo = %jogo_id, processados = palpites.len(), cupons_30, "resultado processado");

    Ok(Json(ResultadoResponse {
        processados: palpites.len(),
        cupons_30,
    }))
}

#[derive(Debug, Deserialize)]
pub struct EditarJogoRequest {
    pub time_a: String,
    pub time_b: String,
    pub data_jogo: chrono::DateTime<chrono::Utc>,
}

/// PUT /admin/jogos/:id — edita os dados básicos de um jogo.
pub async fn editar_jogo(
    State(state): State<AppState>,
    _claims: AdminFull,
    Path(jogo_id): Path<Uuid>,
    Json(req): Json<EditarJogoRequest>,
) -> Result<Json<Jogo>, AppError> {
    let jogo = sqlx::query_as::<_, Jogo>(
        "UPDATE jogos SET time_a = $1, time_b = $2, data_jogo = $3 WHERE id = $4 RETURNING *",
    )
    .bind(&req.time_a)
    .bind(&req.time_b)
    .bind(req.data_jogo)
    .bind(jogo_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NaoEncontrado)?;

    let _ = state.ranking_tx.send("atualizar".to_string());
    Ok(Json(jogo))
}

/// DELETE /admin/jogos/:id — remove um jogo (e seus palpites em cascata).
pub async fn deletar_jogo(
    State(state): State<AppState>,
    _claims: AdminFull,
    Path(jogo_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let r = sqlx::query("DELETE FROM jogos WHERE id = $1")
        .bind(jogo_id)
        .execute(&state.db)
        .await?;

    if r.rows_affected() == 0 {
        return Err(AppError::NaoEncontrado);
    }
    let _ = state.ranking_tx.send("atualizar".to_string());
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /admin/cupons/:id/utilizar — dá baixa em um cupom (marca como utilizado).
pub async fn marcar_cupom(
    State(state): State<AppState>,
    _claims: AdminFull,
    Path(cupom_id): Path<Uuid>,
) -> Result<Json<Cupom>, AppError> {
    let cupom = sqlx::query_as::<_, Cupom>(
        "UPDATE cupons SET utilizado = TRUE WHERE id = $1 RETURNING *",
    )
    .bind(cupom_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NaoEncontrado)?;

    // Notifica os painéis abertos (LEADS em tempo real, igual ao ranking).
    let _ = state.ranking_tx.send("atualizar".to_string());
    Ok(Json(cupom))
}

/// GET /admin/jogos — lista todos os jogos (mais recentes primeiro).
pub async fn listar_jogos(
    State(state): State<AppState>,
    _claims: AdminFull,
) -> Result<Json<Vec<Jogo>>, AppError> {
    let jogos = sqlx::query_as::<_, Jogo>("SELECT * FROM jogos ORDER BY criado_em DESC")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(jogos))
}

#[derive(Debug, Serialize)]
pub struct Metricas {
    pub total_participantes: i64,
    pub total_palpites: i64,
    pub cupons_gerados: i64,
    pub cupons_utilizados: i64,
    pub jogo_maior_participacao: Option<String>,
    pub bolao_encerrado: bool,
    /// Percentual de palpites que pontuaram, entre os palpites já apurados (jogos encerrados).
    pub taxa_acerto: i64,
}

/// GET /admin/metricas — números gerais para os cards do painel (admin e viewer).
pub async fn metricas(
    State(state): State<AppState>,
    _claims: AdminClaims,
) -> Result<Json<Metricas>, AppError> {
    let total_participantes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usuarios")
        .fetch_one(&state.db)
        .await?;
    let total_palpites: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM palpites")
        .fetch_one(&state.db)
        .await?;
    let cupons_gerados: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cupons")
        .fetch_one(&state.db)
        .await?;
    let cupons_utilizados: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cupons WHERE utilizado = TRUE")
            .fetch_one(&state.db)
            .await?;

    let jogo_maior_participacao: Option<String> = sqlx::query_scalar(
        r#"
        SELECT j.time_a || ' x ' || j.time_b
        FROM jogos j
        JOIN palpites p ON p.jogo_id = j.id
        GROUP BY j.id, j.time_a, j.time_b
        ORDER BY COUNT(p.id) DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&state.db)
    .await?;

    let bolao_encerrado = crate::bolao::esta_encerrado(&state.db).await?;

    // Taxa de acerto: % de palpites que cravaram o placar exato (10 pts / cupom 30%),
    // entre os palpites de jogos já encerrados. Acerto parcial e erro não contam.
    let taxa_acerto: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(ROUND(
            100.0 * COUNT(*) FILTER (WHERE p.pontuacao = 10)
                  / NULLIF(COUNT(*) FILTER (WHERE j.status = 'encerrado'), 0)
        ), 0)::BIGINT
        FROM palpites p
        JOIN jogos j ON j.id = p.jogo_id
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(Metricas {
        total_participantes,
        total_palpites,
        cupons_gerados,
        cupons_utilizados,
        jogo_maior_participacao,
        bolao_encerrado,
        taxa_acerto,
    }))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ClassificacaoFinalRow {
    /// Posição no ranking por pontos (empate na pontuação compartilha a posição).
    pub posicao: i64,
    pub nome: String,
    pub total_pontos: i64,
    pub acertos_exatos: i64,
    pub cupons_total: i64,
    pub cupons_utilizados: i64,
    /// Elegível ao prêmio final (regra 7.1): conquistou ≥1 cupom e usou todos.
    pub elegivel: bool,
    /// Campeão: elegível e com a MAIOR pontuação entre os elegíveis. Em caso de
    /// empate na maior pontuação, todos os empatados são campeões (co-campeões).
    pub campeao: bool,
}

/// GET /admin/classificacao-final — ranking por pontos com o status de
/// elegibilidade ao prêmio final de cada participante. Serve para a organização
/// aplicar as regras 7.1/7.2 (desclassificar quem não usou todos os cupons e
/// repassar ao próximo elegível) com transparência. Acessível a admin e viewer.
pub async fn classificacao_final(
    State(state): State<AppState>,
    _claims: AdminClaims,
) -> Result<Json<Vec<ClassificacaoFinalRow>>, AppError> {
    let rows = sqlx::query_as::<_, ClassificacaoFinalRow>(
        r#"
        WITH base AS (
            SELECT
                u.id,
                u.nome,
                COALESCE(SUM(p.pontuacao), 0)::BIGINT                 AS total_pontos,
                COUNT(*) FILTER (WHERE p.pontuacao = 10)::BIGINT      AS acertos_exatos,
                MAX(p.criado_em)                                      AS ultimo_palpite,
                (SELECT COUNT(*) FROM cupons c
                   WHERE c.usuario_id = u.id)::BIGINT                 AS cupons_total,
                (SELECT COUNT(*) FROM cupons c
                   WHERE c.usuario_id = u.id AND c.utilizado)::BIGINT AS cupons_utilizados
            FROM usuarios u
            LEFT JOIN palpites p ON p.usuario_id = u.id
            GROUP BY u.id, u.nome
        ),
        calc AS (
            SELECT *,
                (cupons_total >= 1 AND cupons_utilizados = cupons_total) AS elegivel
            FROM base
        )
        SELECT
            -- Empate na pontuação compartilha a posição (RANK, não ROW_NUMBER).
            RANK() OVER (ORDER BY total_pontos DESC)::BIGINT          AS posicao,
            nome,
            total_pontos,
            acertos_exatos,
            cupons_total,
            cupons_utilizados,
            elegivel,
            -- Campeão: elegível e com a maior pontuação ENTRE OS ELEGÍVEIS.
            -- Todos que empatam nessa pontuação máxima são campeões.
            (elegivel
             AND total_pontos > 0
             AND total_pontos = (SELECT MAX(total_pontos) FROM calc WHERE elegivel)
            )                                                         AS campeao
        FROM calc
        WHERE total_pontos > 0
        ORDER BY total_pontos DESC, ultimo_palpite DESC NULLS LAST, nome
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CuponsParams {
    /// "utilizados" | "disponiveis" | ausente = todos
    pub filtro: Option<String>,
}

/// GET /admin/cupons?filtro=... — lista cupons (Leads); acessível a admin e viewer.
pub async fn listar_cupons(
    State(state): State<AppState>,
    _claims: AdminClaims,
    Query(params): Query<CuponsParams>,
) -> Result<Json<Vec<Cupom>>, AppError> {
    let sql = match params.filtro.as_deref() {
        Some("utilizados") => "SELECT * FROM cupons WHERE utilizado = TRUE ORDER BY criado_em DESC",
        Some("disponiveis") => {
            "SELECT * FROM cupons WHERE utilizado = FALSE ORDER BY criado_em DESC"
        }
        _ => "SELECT * FROM cupons ORDER BY criado_em DESC",
    };

    let cupons = sqlx::query_as::<_, Cupom>(sql).fetch_all(&state.db).await?;
    Ok(Json(cupons))
}

/// GET /admin/landing — devolve a configuração atual da landing page.
pub async fn obter_landing(
    State(state): State<AppState>,
    _claims: AdminFull,
) -> Result<Json<LandingConfig>, AppError> {
    Ok(Json(crate::landing::carregar(&state.db).await?))
}

/// PUT /admin/landing — salva a configuração da landing e define qual jogo
/// (já cadastrado no card Jogos) é o ativo no site. NÃO cadastra nem edita jogos.
pub async fn salvar_landing(
    State(state): State<AppState>,
    _claims: AdminFull,
    Json(cfg): Json<LandingConfig>,
) -> Result<Json<LandingConfig>, AppError> {
    // O confronto apenas seleciona o jogo atual: ativa o jogo escolhido
    // (sem reabrir encerrados). O cadastro/edição dos jogos é feito no card Jogos.
    if let Some(id) = cfg.jogo_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()) {
        sqlx::query(
            "UPDATE jogos SET ativo = TRUE, status = 'ativo'
             WHERE id = $1 AND status <> 'encerrado'",
        )
        .bind(id)
        .execute(&state.db)
        .await?;
    }

    crate::landing::salvar(&state.db, &cfg).await?;
    let _ = state.ranking_tx.send("atualizar".to_string());
    Ok(Json(cfg))
}

#[derive(Debug, Serialize)]
pub struct BandeiraInfo {
    pub nome: String,
    pub url: String,
}

/// Nome amigável (PT) da bandeira a partir do nome do arquivo.
fn nome_bandeira(arquivo: &str) -> String {
    let l = arquivo.to_lowercase();
    if l.contains("brazil") || l.contains("brasil") {
        return "Brasil".to_string();
    }
    if l.contains("morocco") || l.contains("maroc") {
        return "Marrocos".to_string();
    }
    if l.contains("haiti") {
        return "Haiti".to_string();
    }
    if l.contains("scotland") {
        return "Escócia".to_string();
    }
    if l.contains("japan") {
        return "Japão".to_string();
    }
    arquivo
        .trim_start_matches("Flag_of_")
        .trim_start_matches("Flag_")
        .rsplit_once('.')
        .map(|(n, _)| n)
        .unwrap_or(arquivo)
        .replace('_', " ")
}

/// GET /admin/bandeiras — lista as bandeiras disponíveis em `static/img`.
pub async fn listar_bandeiras(
    _claims: AdminFull,
) -> Result<Json<Vec<BandeiraInfo>>, AppError> {
    let dir = std::fs::read_dir("static/img")
        .map_err(|e| AppError::Interno(anyhow::anyhow!("falha ao ler static/img: {e}")))?;

    let mut bandeiras: Vec<BandeiraInfo> = dir
        .flatten()
        .filter_map(|e| {
            let arquivo = e.file_name().to_string_lossy().to_string();
            let l = arquivo.to_lowercase();
            let img = l.ends_with(".svg") || l.ends_with(".png");
            if l.starts_with("flag") && img {
                Some(BandeiraInfo {
                    nome: nome_bandeira(&arquivo),
                    url: format!("/static/img/{arquivo}"),
                })
            } else {
                None
            }
        })
        .collect();

    bandeiras.sort_by(|a, b| a.nome.cmp(&b.nome));
    Ok(Json(bandeiras))
}
