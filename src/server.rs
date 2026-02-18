use axum::{
    middleware as axum_mw,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::handlers;
use crate::metrics::stream;
use crate::middleware::timing;
use crate::AppState;

/// Builds the full Axum `Router` with all routes, middleware, and static serving.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // ── User endpoints ──────────────────────────────────────
        .route("/api/users/:id", get(handlers::users::get_user))
        .route("/api/users", post(handlers::users::create_user))
        // ── Session endpoints ───────────────────────────────────
        .route(
            "/api/sessions/:id",
            get(handlers::sessions::get_session),
        )
        .route("/api/sessions", post(handlers::sessions::create_session))
        // ── Product endpoints ───────────────────────────────────
        .route(
            "/api/products/:id",
            get(handlers::products::get_product),
        )
        // ── Benchmark control ───────────────────────────────────
        .route(
            "/api/benchmark/start",
            post(handlers::benchmark::start_benchmark),
        )
        .route(
            "/api/benchmark/stop",
            post(handlers::benchmark::stop_benchmark),
        )
        .route(
            "/api/benchmark/status",
            get(handlers::benchmark::benchmark_status),
        )
        // ── Metrics ─────────────────────────────────────────────
        .route("/api/metrics", get(stream::get_metrics))
        .route("/api/metrics/stream", get(stream::metrics_stream))
        // ── Provide shared state to all routes above ────────────
        .with_state(state)
        // ── Serve static/ directory for the dashboard ───────────
        .fallback_service(ServeDir::new("static"))
        // ── Global middleware (applied bottom-up) ───────────────
        .layer(axum_mw::from_fn(timing::timing_middleware))
        .layer(CorsLayer::permissive())
}