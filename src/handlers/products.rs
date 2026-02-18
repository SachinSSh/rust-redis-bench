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

// ─── Domain type ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub title: String,
    /// Price in cents (e.g. 12999 = $129.99)
    pub price: u64,
    pub stock: u32,
    pub category: String,
    pub description: String,
}

// ─── GET /api/products/:id ───────────────────────────────────────

pub async fn get_product(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TimedResponse<Product>>, AppError> {
    let t0 = Instant::now();

    let key = format!("product:{id}");

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
            endpoint: "GET /api/products/:id".into(),
            redis_us,
            rust_us: 0,
            total_us: t0.elapsed().as_micros() as u64,
            is_read: true,
            success: false,
        });
        return Err(AppError::NotFound(format!("product '{id}' not found")));
    }

    // Rust work: parse hash fields into typed struct
    let product = product_from_map(&map);

    let total_us = t0.elapsed().as_micros() as u64;
    let rust_us = total_us.saturating_sub(redis_us);

    state.metrics.record(Sample {
        endpoint: "GET /api/products/:id".into(),
        redis_us,
        rust_us,
        total_us,
        is_read: true,
        success: true,
    });

    Ok(Json(TimedResponse {
        data: product,
        timing: RequestTiming {
            total_us,
            redis_us,
            rust_overhead_us: rust_us,
        },
    }))
}

// ─── Helpers ─────────────────────────────────────────────────────

fn product_from_map(map: &HashMap<String, String>) -> Product {
    Product {
        id: map.get("id").cloned().unwrap_or_default(),
        title: map.get("title").cloned().unwrap_or_default(),
        price: map
            .get("price")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        stock: map
            .get("stock")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        category: map.get("category").cloned().unwrap_or_default(),
        description: map.get("description").cloned().unwrap_or_default(),
    }
}