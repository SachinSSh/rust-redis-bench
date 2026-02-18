use axum::{
    extract::{Path, State},
    Json,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::metrics::Sample;
use crate::AppState;

use super::{AppError, RequestTiming, TimedResponse};

// ─── Domain types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub ip: String,
    pub created_at: String,
    pub ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub user_id: String,
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,
}

fn default_ip() -> String {
    "127.0.0.1".into()
}
fn default_ttl() -> u64 {
    300
}

// ─── GET /api/sessions/:id ───────────────────────────────────────

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TimedResponse<Session>>, AppError> {
    let t0 = Instant::now();

    let key = format!("session:{id}");

    // ── Redis READ ──────────────────────────────────────────────
    let t_redis = Instant::now();
    let mut conn = state.redis.clone();
    let maybe_json: Option<String> = conn
        .get(&key)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let redis_us = t_redis.elapsed().as_micros() as u64;
    // ────────────────────────────────────────────────────────────

    let json_str = match maybe_json {
        Some(v) => v,
        None => {
            state.metrics.record(Sample {
                endpoint: "GET /api/sessions/:id".into(),
                redis_us,
                rust_us: 0,
                total_us: t0.elapsed().as_micros() as u64,
                is_read: true,
                success: false,
            });
            return Err(AppError::NotFound(format!(
                "session '{id}' not found or expired"
            )));
        }
    };

    // Rust work: deserialize JSON blob
    let session: Session = serde_json::from_str(&json_str)
        .map_err(|e| AppError::Internal(format!("corrupt session data: {e}")))?;

    let total_us = t0.elapsed().as_micros() as u64;
    let rust_us = total_us.saturating_sub(redis_us);

    state.metrics.record(Sample {
        endpoint: "GET /api/sessions/:id".into(),
        redis_us,
        rust_us,
        total_us,
        is_read: true,
        success: true,
    });

    Ok(Json(TimedResponse {
        data: session,
        timing: RequestTiming {
            total_us,
            redis_us,
            rust_overhead_us: rust_us,
        },
    }))
}

// ─── POST /api/sessions ──────────────────────────────────────────

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<TimedResponse<Session>>, AppError> {
    let t0 = Instant::now();

    // Rust work: build entity + serialize to JSON
    let session = Session {
        id: format!("sess_{}", &uuid::Uuid::new_v4().to_string()[..8]),
        user_id: req.user_id,
        token: format!("tok_{}", uuid::Uuid::new_v4()),
        ip: req.ip,
        created_at: chrono::Utc::now().to_rfc3339(),
        ttl_secs: req.ttl_secs,
    };

    let key = format!("session:{}", session.id);
    let json_str = serde_json::to_string(&session)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // ── Redis WRITE (with TTL) ──────────────────────────────────
    let t_redis = Instant::now();
    let mut conn = state.redis.clone();
    let mut cmd = redis::cmd("SET");
    cmd.arg(&key)
        .arg(&json_str)
        .arg("EX")
        .arg(session.ttl_secs);
    let _: () = cmd
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let redis_us = t_redis.elapsed().as_micros() as u64;
    // ────────────────────────────────────────────────────────────

    let total_us = t0.elapsed().as_micros() as u64;
    let rust_us = total_us.saturating_sub(redis_us);

    state.metrics.record(Sample {
        endpoint: "POST /api/sessions".into(),
        redis_us,
        rust_us,
        total_us,
        is_read: false,
        success: true,
    });

    Ok(Json(TimedResponse {
        data: session,
        timing: RequestTiming {
            total_us,
            redis_us,
            rust_overhead_us: rust_us,
        },
    }))
}