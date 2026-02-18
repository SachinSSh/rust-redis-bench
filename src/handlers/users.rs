use axum::{
    extract::{Path, State},
    Json,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::metrics::Sample;
use crate::AppState;

use super::{AppError, RequestTiming, TimedResponse};

// ─── Domain types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub prefs: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default = "default_prefs")]
    pub prefs: String,
}

fn default_role() -> String {
    "viewer".into()
}
fn default_prefs() -> String {
    r#"{"theme":"light","lang":"en","notifications":true}"#.into()
}

// ─── GET /api/users/:id ──────────────────────────────────────────

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TimedResponse<User>>, AppError> {
    let t0 = Instant::now();

    // Rust work: build key
    let key = format!("user:{id}");

    // ── Redis READ ──────────────────────────────────────────────
    let t_redis = Instant::now();
    let mut conn = state.redis.clone();
    let map: HashMap<String, String> = conn
        .hgetall(&key)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let redis_us = t_redis.elapsed().as_micros() as u64;
    // ────────────────────────────────────────────────────────────

    if map.is_empty() {
        state.metrics.record(Sample {
            endpoint: "GET /api/users/:id".into(),
            redis_us,
            rust_us: 0,
            total_us: t0.elapsed().as_micros() as u64,
            is_read: true,
            success: false,
        });
        return Err(AppError::NotFound(format!("user '{id}' not found")));
    }

    // Rust work: deserialize hash → struct
    let user = user_from_map(&map);

    let total_us = t0.elapsed().as_micros() as u64;
    let rust_us = total_us.saturating_sub(redis_us);

    state.metrics.record(Sample {
        endpoint: "GET /api/users/:id".into(),
        redis_us,
        rust_us,
        total_us,
        is_read: true,
        success: true,
    });

    Ok(Json(TimedResponse {
        data: user,
        timing: RequestTiming {
            total_us,
            redis_us,
            rust_overhead_us: rust_us,
        },
    }))
}

// ─── POST /api/users ─────────────────────────────────────────────

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<TimedResponse<User>>, AppError> {
    let t0 = Instant::now();

    // Rust work: build entity
    let user = User {
        id: format!("usr_{}", &uuid::Uuid::new_v4().to_string()[..8]),
        name: req.name,
        email: req.email,
        role: req.role,
        prefs: req.prefs,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let key = format!("user:{}", user.id);

    // ── Redis WRITE ─────────────────────────────────────────────
    let t_redis = Instant::now();
    let mut conn = state.redis.clone();
    let mut cmd = redis::cmd("HSET");
    cmd.arg(&key)
        .arg("id")
        .arg(&user.id)
        .arg("name")
        .arg(&user.name)
        .arg("email")
        .arg(&user.email)
        .arg("role")
        .arg(&user.role)
        .arg("prefs")
        .arg(&user.prefs)
        .arg("created_at")
        .arg(&user.created_at);
    let _: () = cmd
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Redis(e.to_string()))?;
    let redis_us = t_redis.elapsed().as_micros() as u64;
    // ────────────────────────────────────────────────────────────

    let total_us = t0.elapsed().as_micros() as u64;
    let rust_us = total_us.saturating_sub(redis_us);

    state.metrics.record(Sample {
        endpoint: "POST /api/users".into(),
        redis_us,
        rust_us,
        total_us,
        is_read: false,
        success: true,
    });

    Ok(Json(TimedResponse {
        data: user,
        timing: RequestTiming {
            total_us,
            redis_us,
            rust_overhead_us: rust_us,
        },
    }))
}

// ─── Helpers ─────────────────────────────────────────────────────

fn user_from_map(map: &HashMap<String, String>) -> User {
    User {
        id: map.get("id").cloned().unwrap_or_default(),
        name: map.get("name").cloned().unwrap_or_default(),
        email: map.get("email").cloned().unwrap_or_default(),
        role: map.get("role").cloned().unwrap_or_default(),
        prefs: map.get("prefs").cloned().unwrap_or_default(),
        created_at: map.get("created_at").cloned().unwrap_or_default(),
    }
}