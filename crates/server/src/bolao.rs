use sqlx::PgPool;

/// Estado do bolão — calculado automaticamente a partir da data/horário dos jogos.
///
/// O bolão encerra sozinho quando o jogo cadastrado chega na data/horário: fica
/// "aberto" enquanto existe um jogo ativo (não encerrado) com data ainda no
/// futuro; quando esse prazo passa (ou o jogo é encerrado), a home passa a
/// exibir o pódio e os palpites deixam de ser aceitos. Cadastrar um novo jogo
/// futuro reabre o bolão automaticamente.
pub async fn esta_encerrado(db: &PgPool) -> Result<bool, sqlx::Error> {
    // Sem nenhum jogo cadastrado ainda não há bolão a encerrar.
    let tem_jogo: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM jogos)")
        .fetch_one(db)
        .await?;
    if !tem_jogo {
        return Ok(false);
    }

    let tem_aberto: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM jogos
             WHERE ativo = TRUE AND status <> 'encerrado' AND data_jogo > NOW()
         )",
    )
    .fetch_one(db)
    .await?;

    Ok(!tem_aberto)
}
