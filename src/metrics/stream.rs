use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use super::collector::MetricsSnapshot;
use crate::AppState;

// ─── GET /api/metrics ────────────────────────────────────────────
/// Returns a single JSON snapshot — useful for curl / debugging.

pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
) -> Json<MetricsSnapshot> {
    Json(state.metrics.snapshot())
}

// ─── GET /api/metrics/stream ─────────────────────────────────────
/// Server-Sent Events endpoint.
/// Pushes a full `MetricsSnapshot` as JSON every 500 ms.
/// The browser's `EventSource` connects here and feeds the charts.

pub async fn metrics_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    // Tick every 500 ms → 2 updates per second to the dashboard
    let interval = tokio::time::interval(Duration::from_millis(500));

    let stream = IntervalStream::new(interval).map(move |_| {
        let snapshot = state.metrics.snapshot();
        let json = serde_json::to_string(&snapshot).unwrap_or_default();
        Ok(Event::default().data(json))
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}