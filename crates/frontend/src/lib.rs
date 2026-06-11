use askama::Template;

/// Dados de um jogo exibido na home (destaque ou lista de abertos).
pub struct JogoView {
    pub id: String,
    pub time_a: String,
    pub time_b: String,
    /// Data do jogo em ISO 8601 (consumida pelo countdown.js via data-target).
    pub data_jogo_iso: String,
    /// URL da bandeira de cada time (quando reconhecida), ou None → ícone ⚽.
    pub bandeira_a: Option<String>,
    pub bandeira_b: Option<String>,
}

/// Linha do pódio exibido quando o bolão é encerrado.
pub struct PodioView {
    pub posicao: i64,
    pub nome: String,
    pub total_pontos: i64,
}

/// Rede social ativa exibida no rodapé da home.
pub struct RedeView {
    pub nome: String,
    pub url: String,
    /// Markup SVG (inserido com `|safe`) do ícone da rede.
    pub svg: String,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub encerrado: bool,
    pub podio: Vec<PodioView>,
    pub destaque: Option<JogoView>,
    pub outros: Vec<JogoView>,
    /// Redes sociais ativas (rodapé).
    pub redes: Vec<RedeView>,
    /// Link do WhatsApp configurado (modal de sucesso), quando ativo.
    pub whatsapp_url: Option<String>,
    /// Percentuais de desconto configurados no painel (cupons).
    pub desconto_participacao: i32,
    pub desconto_acerto: i32,
}

#[derive(Template)]
#[template(path = "ranking.html")]
pub struct RankingTemplate {}

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminTemplate {}
