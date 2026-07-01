use crate::bolao;
use crate::errors::AppError;
use crate::models::Jogo;
use crate::state::AppState;
use askama::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Html;
use frontend::{
    AdminTemplate, IndexTemplate, JogoView, PodioView, PrivacidadeTemplate, RankingTemplate,
    RedeView, RegrasTemplate, TermosTemplate,
};

fn render<T: Template>(tpl: T) -> Result<Html<String>, AppError> {
    let html = tpl
        .render()
        .map_err(|e| AppError::Interno(anyhow::anyhow!("falha ao renderizar template: {e}")))?;
    Ok(Html(html))
}

/// Monta a URL base absoluta (ex.: `https://dominio`) a partir dos cabeçalhos da
/// requisição. Necessária para a imagem do Open Graph (WhatsApp não aceita URL
/// relativa). Respeita `X-Forwarded-Host`/`X-Forwarded-Proto` (proxy reverso) e
/// assume `https` fora de localhost.
fn base_url(headers: &HeaderMap) -> String {
    let header_str = |nome: &str| headers.get(nome).and_then(|v| v.to_str().ok());

    let host = header_str("x-forwarded-host")
        .or_else(|| header_str("host"))
        .unwrap_or("localhost:3000");

    let proto = header_str("x-forwarded-proto").unwrap_or({
        if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
            "http"
        } else {
            "https"
        }
    });

    format!("{proto}://{host}")
}

/// Mapeia o nome do time (PT ou EN) para a bandeira disponível em static/img.
fn bandeira(nome: &str) -> Option<String> {
    let arquivo = match nome.trim().to_lowercase().as_str() {
        "brasil" | "brazil" => "Flag_of_Brazil.svg",
        "marrocos" | "morocco" | "maroc" => "Flag_of_Morocco.svg",
        "haiti" | "haïti" | "haíti" => "Flag_of_Haiti.svg",
        "escócia" | "escocia" | "scotland" => "Flag_of_Scotland.svg",
        "japão" | "japao" | "japan" => "Flag_of_Japan.svg",
        "noruega" | "norway" => "Flag_of_Norway.svg",
        _ => return None,
    };
    Some(format!("/static/img/{arquivo}"))
}

fn to_view(j: Jogo) -> JogoView {
    // Bandeira escolhida no cadastro tem prioridade; senão, mapeia pelo nome.
    let bandeira_a = j
        .bandeira_a
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| bandeira(&j.time_a));
    let bandeira_b = j
        .bandeira_b
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| bandeira(&j.time_b));
    JogoView {
        id: j.id.to_string(),
        time_a: j.time_a,
        time_b: j.time_b,
        data_jogo_iso: j.data_jogo.to_rfc3339(),
        bandeira_a,
        bandeira_b,
    }
}

#[derive(sqlx::FromRow)]
struct PodioRow {
    nome: String,
    total_pontos: i64,
}

/// Ícone SVG (viewBox 24) de cada rede social, inserido com `|safe` no template.
fn icone_rede(slug: &str) -> &'static str {
    match slug {
        "instagram" => r#"<svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" aria-hidden="true"><path d="M12 2.16c3.2 0 3.58.01 4.85.07 3.25.15 4.77 1.69 4.92 4.92.06 1.27.07 1.65.07 4.85s-.01 3.58-.07 4.85c-.15 3.23-1.66 4.77-4.92 4.92-1.27.06-1.65.07-4.85.07s-3.58-.01-4.85-.07c-3.26-.15-4.77-1.7-4.92-4.92C2.17 15.58 2.16 15.2 2.16 12s.01-3.58.07-4.85C2.38 3.92 3.9 2.38 7.15 2.23 8.42 2.17 8.8 2.16 12 2.16ZM12 0C8.74 0 8.33.01 7.05.07 2.7.27.27 2.69.07 7.05.01 8.33 0 8.74 0 12s.01 3.67.07 4.95c.2 4.36 2.62 6.78 6.98 6.98C8.33 23.99 8.74 24 12 24s3.67-.01 4.95-.07c4.35-.2 6.78-2.62 6.98-6.98.06-1.28.07-1.69.07-4.95s-.01-3.67-.07-4.95c-.2-4.35-2.62-6.78-6.98-6.98C15.67.01 15.26 0 12 0Zm0 5.84a6.16 6.16 0 1 0 0 12.32 6.16 6.16 0 0 0 0-12.32Zm0 10.16a4 4 0 1 1 0-8 4 4 0 0 1 0 8Zm6.41-11.85a1.44 1.44 0 1 0 0 2.88 1.44 1.44 0 0 0 0-2.88Z"/></svg>"#,
        "facebook" => r#"<svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" aria-hidden="true"><path d="M24 12.07C24 5.4 18.63 0 12 0S0 5.4 0 12.07c0 6 4.39 10.95 10.13 11.85v-8.38H7.08v-3.47h3.05V9.43c0-3.01 1.79-4.67 4.53-4.67 1.31 0 2.69.24 2.69.24v2.95h-1.51c-1.49 0-1.96.93-1.96 1.87v2.25h3.33l-.53 3.47h-2.8v8.38C19.61 23.02 24 18.07 24 12.07Z"/></svg>"#,
        "tiktok" => r#"<svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" aria-hidden="true"><path d="M12.53.02C13.84 0 15.14.01 16.44 0c.08 1.53.63 3.09 1.75 4.17 1.12 1.11 2.7 1.62 4.24 1.79v4.03c-1.44-.05-2.89-.35-4.2-.97-.57-.26-1.1-.59-1.62-.93-.01 2.92.01 5.84-.02 8.75-.08 1.4-.54 2.79-1.35 3.94-1.31 1.92-3.58 3.17-5.91 3.21-1.43.08-2.86-.31-4.08-1.03-2.02-1.19-3.44-3.37-3.65-5.71-.02-.5-.03-1-.01-1.49.18-1.9 1.12-3.72 2.58-4.96 1.66-1.44 3.98-2.13 6.15-1.72.02 1.48-.04 2.96-.04 4.44-.99-.32-2.15-.23-3.02.37-.63.41-1.11 1.04-1.36 1.75-.21.51-.15 1.07-.14 1.61.24 1.64 1.82 3.02 3.5 2.87 1.12-.01 2.19-.66 2.77-1.61.19-.33.4-.67.41-1.06.1-1.79.06-3.57.07-5.36.01-4.03-.01-8.05.02-12.07Z"/></svg>"#,
        "whatsapp" => r#"<svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" aria-hidden="true"><path d="M17.47 14.38c-.3-.15-1.76-.87-2.03-.97-.27-.1-.47-.15-.67.15-.2.3-.77.96-.94 1.16-.17.2-.35.22-.64.07-.3-.15-1.26-.46-2.39-1.48-.88-.79-1.48-1.76-1.65-2.06-.17-.3-.02-.46.13-.6.13-.14.3-.35.45-.52.15-.18.2-.3.3-.5.1-.2.05-.37-.02-.52-.08-.15-.67-1.61-.92-2.21-.24-.58-.49-.5-.67-.51h-.57c-.2 0-.52.07-.79.37-.27.3-1.04 1.02-1.04 2.48 0 1.46 1.06 2.88 1.21 3.07.15.2 2.1 3.2 5.08 4.49.71.3 1.26.49 1.69.62.71.23 1.36.2 1.87.12.57-.09 1.76-.72 2-1.41.25-.7.25-1.29.18-1.41-.08-.13-.27-.2-.57-.35Zm-5.42 7.4h-.01a9.87 9.87 0 0 1-5.03-1.38l-.36-.21-3.74.98 1-3.65-.24-.38a9.86 9.86 0 0 1-1.51-5.26C2.16 6.43 6.6 2 12.05 2c2.64 0 5.12 1.03 6.99 2.9a9.82 9.82 0 0 1 2.89 6.99c0 5.45-4.44 9.89-9.88 9.89Zm8.41-18.3A11.81 11.81 0 0 0 12.05 0C5.5 0 .16 5.34.16 11.89c0 2.1.55 4.14 1.59 5.95L.06 24l6.3-1.65a11.88 11.88 0 0 0 5.68 1.45h.01c6.55 0 11.89-5.34 11.89-11.89 0-3.18-1.24-6.17-3.48-8.42Z"/></svg>"#,
        "youtube" => r#"<svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor" aria-hidden="true"><path d="M23.5 6.2a3.02 3.02 0 0 0-2.12-2.14C19.5 3.55 12 3.55 12 3.55s-7.5 0-9.38.51A3.02 3.02 0 0 0 .5 6.2 31.5 31.5 0 0 0 0 12a31.5 31.5 0 0 0 .5 5.8 3.02 3.02 0 0 0 2.12 2.14C4.5 20.45 12 20.45 12 20.45s7.5 0 9.38-.51a3.02 3.02 0 0 0 2.12-2.14A31.5 31.5 0 0 0 24 12a31.5 31.5 0 0 0-.5-5.8ZM9.6 15.57V8.43L15.82 12 9.6 15.57Z"/></svg>"#,
        _ => "",
    }
}

/// Monta as redes sociais ativas (rodapé) e o link de WhatsApp (modal de sucesso).
async fn redes_e_whatsapp(state: &AppState) -> (Vec<RedeView>, Option<String>) {
    let cfg = crate::landing::carregar(&state.db).await.unwrap_or_default();
    let itens = [
        ("instagram", "Instagram", &cfg.instagram),
        ("facebook", "Facebook", &cfg.facebook),
        ("tiktok", "TikTok", &cfg.tiktok),
        ("whatsapp", "WhatsApp", &cfg.whatsapp),
        ("youtube", "YouTube", &cfg.youtube),
    ];
    let redes = itens
        .iter()
        .filter(|(_, _, r)| r.ativo && !r.url.trim().is_empty())
        .map(|(slug, nome, r)| RedeView {
            nome: nome.to_string(),
            url: r.url.clone(),
            svg: icone_rede(slug).to_string(),
        })
        .collect();
    let whatsapp_url = if cfg.whatsapp.ativo && !cfg.whatsapp.url.trim().is_empty() {
        Some(cfg.whatsapp.url.clone())
    } else {
        None
    };
    (redes, whatsapp_url)
}

/// GET / — landing page. Se o bolão estiver encerrado, mostra o pódio;
/// caso contrário, mostra os jogos abertos (1 destaque + lista).
pub async fn index(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Html<String>, AppError> {
    let base = base_url(&headers);
    // Bolão encerrado: exibe o pódio FINAL (top 3 entre os elegíveis ao prêmio).
    // Elegibilidade (regras 6.2/7.1/7.2): só concorre ao prêmio final quem
    // conquistou pelo menos um cupom e UTILIZOU TODOS eles. Quem deixou cupom
    // sem usar não entra no pódio — o que implementa o repasse ao próximo
    // colocado elegível (7.2). O 1º elegível é o Palpiteiro Campeão (5.1).
    if bolao::esta_encerrado(&state.db).await? {
        let linhas = sqlx::query_as::<_, PodioRow>(
            r#"
            SELECT u.nome, COALESCE(SUM(p.pontuacao), 0)::BIGINT AS total_pontos
            FROM usuarios u
            LEFT JOIN palpites p ON p.usuario_id = u.id
            WHERE EXISTS (SELECT 1 FROM cupons c WHERE c.usuario_id = u.id)
              AND NOT EXISTS (
                  SELECT 1 FROM cupons c
                  WHERE c.usuario_id = u.id AND c.utilizado = FALSE
              )
            GROUP BY u.id, u.nome
            HAVING COALESCE(SUM(p.pontuacao), 0) > 0
            ORDER BY total_pontos DESC,
                     MAX(p.criado_em) DESC NULLS LAST,
                     u.nome ASC
            LIMIT 3
            "#,
        )
        .fetch_all(&state.db)
        .await?;

        // Maior pontuação entre os elegíveis (primeira linha, ordenada DESC).
        // Todos que empatam nessa pontuação são campeões (co-campeões).
        let max_pontos = linhas.first().map(|r| r.total_pontos).unwrap_or(0);

        // Posição compartilhada em caso de empate (RANK): co-campeões ficam todos em 1º.
        let mut podio: Vec<PodioView> = Vec::with_capacity(linhas.len());
        let mut posicao = 0i64;
        let mut pontos_anterior: Option<i64> = None;
        for (i, r) in linhas.into_iter().enumerate() {
            if pontos_anterior != Some(r.total_pontos) {
                posicao = i as i64 + 1;
                pontos_anterior = Some(r.total_pontos);
            }
            podio.push(PodioView {
                posicao,
                campeao: r.total_pontos == max_pontos,
                nome: r.nome,
                total_pontos: r.total_pontos,
            });
        }

        let (redes, whatsapp_url) = redes_e_whatsapp(&state).await;
        let cfg = crate::landing::carregar(&state.db).await.unwrap_or_default();
        return render(IndexTemplate {
            encerrado: true,
            podio,
            destaque: None,
            outros: Vec::new(),
            redes,
            whatsapp_url,
            desconto_participacao: cfg.cupom_participacao_desconto,
            desconto_acerto: cfg.cupom_acerto_desconto,
            base_url: base,
        });
    }

    // Bolão aberto: lista os jogos abertos (futuros) — o 1º vira destaque.
    let abertos = sqlx::query_as::<_, Jogo>(
        "SELECT * FROM jogos
         WHERE ativo = TRUE AND status = 'ativo' AND data_jogo > NOW()
         ORDER BY data_jogo ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let mut views: Vec<JogoView> = abertos.into_iter().map(to_view).collect();
    let destaque = if views.is_empty() {
        None
    } else {
        Some(views.remove(0))
    };

    let (redes, whatsapp_url) = redes_e_whatsapp(&state).await;
    let cfg = crate::landing::carregar(&state.db).await.unwrap_or_default();
    render(IndexTemplate {
        encerrado: false,
        podio: Vec::new(),
        destaque,
        outros: views,
        redes,
        whatsapp_url,
        desconto_participacao: cfg.cupom_participacao_desconto,
        desconto_acerto: cfg.cupom_acerto_desconto,
        base_url: base,
    })
}

/// GET /ranking — página de ranking completo (dados carregados via JS/SSE).
pub async fn ranking_page(headers: HeaderMap) -> Result<Html<String>, AppError> {
    render(RankingTemplate {
        base_url: base_url(&headers),
    })
}

/// GET /regras — tela com as regras escritas do bolão.
pub async fn regras_page(headers: HeaderMap) -> Result<Html<String>, AppError> {
    render(RegrasTemplate {
        base_url: base_url(&headers),
    })
}

/// GET /admin — painel administrativo (login + gestão via JS/fetch).
pub async fn painel(headers: HeaderMap) -> Result<Html<String>, AppError> {
    render(AdminTemplate {
        base_url: base_url(&headers),
    })
}

/// GET /termos — Termos de Uso.
pub async fn termos_page(headers: HeaderMap) -> Result<Html<String>, AppError> {
    render(TermosTemplate {
        base_url: base_url(&headers),
    })
}

/// GET /privacidade — Política de Privacidade.
pub async fn privacidade_page(headers: HeaderMap) -> Result<Html<String>, AppError> {
    render(PrivacidadeTemplate {
        base_url: base_url(&headers),
    })
}
