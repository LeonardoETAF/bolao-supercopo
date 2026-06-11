use crate::state::AppState;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use futures::StreamExt;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;

/// GET /api/ranking/stream — fluxo SSE que avisa os clientes quando o ranking muda.
pub async fn ranking_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.ranking_tx.subscribe();

    let stream = BroadcastStream::new(rx).map(|msg| {
        let dado = msg.unwrap_or_else(|_| "atualizar".to_string());
        Ok(Event::default().data(dado))
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
