pub mod benchmark;
pub mod products;
pub mod sessions;
pub mod users;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

// ─── Shared response envelope ────────────────────────────────────

/// Every API response is wrapped with timing metadata so the
/// dashboard can show per-request latency without parsing headers.
#[derive(Debug, Clone, Serialize)]
pub struct TimedResponse<T: Serialize> {
    pub data: T,
    pub timing: RequestTiming,
}

/// Microsecond-precision breakdown of where wall-clock time was spent.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct RequestTiming {
    /// Total handler wall time (μs)
    pub total_us: u64,
    /// Time spent inside the Redis round-trip (μs)
    pub redis_us: u64,
    /// Rust serialization / validation / routing overhead (μs)
    pub rust_overhead_us: u64,
}

// ─── Unified error type ──────────────────────────────────────────

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Redis(String),
    BadRequest(String),
    Internal(String),
    AlreadyRunning,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::Redis(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Redis: {msg}"))
            }
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::AlreadyRunning => {
                (StatusCode::CONFLICT, "Benchmark already running".into())
            }
        };

        let body = serde_json::json!({
            "error":  message,
            "status": status.as_u16(),
        });

        (status, Json(body)).into_response()
    }
}