use axum::{
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::AppState;

use super::AppError;

// ─── Request / response types ────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkConfig {
    /// Number of concurrent Tokio tasks generating load
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,

    /// How long the benchmark runs (seconds)
    #[serde(default = "default_duration")]
    pub duration_secs: u64,

    /// Percentage of operations that are reads (0–100)
    #[serde(default = "default_read_pct")]
    pub read_pct: u8,
}

fn default_concurrency() -> u32 {
    10
}
fn default_duration() -> u64 {
    30
}
fn default_read_pct() -> u8 {
    70
}

#[derive(Debug, Serialize)]
pub struct BenchmarkStatus {
    pub running: bool,
    pub message: String,
}

// ─── POST /api/benchmark/start ───────────────────────────────────

pub async fn start_benchmark(
    State(state): State<Arc<AppState>>,
    Json(config): Json<BenchmarkConfig>,
) -> Result<Json<BenchmarkStatus>, AppError> {
    // Guard: only one benchmark at a time
    if state.load_running.load(Ordering::SeqCst) {
        return Err(AppError::AlreadyRunning);
    }

    // Validate inputs
    if config.concurrency == 0 || config.concurrency > 500 {
        return Err(AppError::BadRequest(
            "concurrency must be between 1 and 500".into(),
        ));
    }
    if config.duration_secs == 0 || config.duration_secs > 300 {
        return Err(AppError::BadRequest(
            "duration_secs must be between 1 and 300".into(),
        ));
    }
    if config.read_pct > 100 {
        return Err(AppError::BadRequest(
            "read_pct must be between 0 and 100".into(),
        ));
    }

    // Reset metrics for a clean run
    state.metrics.reset();

    // Flip the flag BEFORE spawning so workers see it immediately
    state.load_running.store(true, Ordering::SeqCst);

    // Capture values for the status message before the move
    let msg = format!(
        "Started: {} workers × {}s, {}% reads / {}% writes",
        config.concurrency,
        config.duration_secs,
        config.read_pct,
        100u8.saturating_sub(config.read_pct),
    );

    // Capture clones for the spawned task
    let running = state.load_running.clone();
    let metrics = state.metrics.clone();
    let redis = state.redis.clone();
    let concurrency = config.concurrency;
    let duration_secs = config.duration_secs;
    let read_pct = config.read_pct;

    let handle = tokio::spawn(async move {
        crate::load_generator::run(
            running,
            metrics,
            redis,
            concurrency,
            duration_secs,
            read_pct,
        )
        .await;
    });

    // Stash the handle so `stop` can await clean shutdown
    let mut guard = state.load_handle.lock().await;
    *guard = Some(handle);

    Ok(Json(BenchmarkStatus {
        running: true,
        message: msg,
    }))
}

// ─── POST /api/benchmark/stop ────────────────────────────────────

pub async fn stop_benchmark(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BenchmarkStatus>, AppError> {
    if !state.load_running.load(Ordering::SeqCst) {
        return Ok(Json(BenchmarkStatus {
            running: false,
            message: "No benchmark is running".into(),
        }));
    }

    // Signal all workers to stop
    state.load_running.store(false, Ordering::SeqCst);

    // Await the load-generator task so we know it's fully stopped
    let mut guard = state.load_handle.lock().await;
    if let Some(handle) = guard.take() {
        // Ignore JoinError — the task may have already finished
        let _ = handle.await;
    }

    Ok(Json(BenchmarkStatus {
        running: false,
        message: "Benchmark stopped".into(),
    }))
}

// ─── GET /api/benchmark/status ───────────────────────────────────

pub async fn benchmark_status(
    State(state): State<Arc<AppState>>,
) -> Json<BenchmarkStatus> {
    let running = state.load_running.load(Ordering::SeqCst);
    Json(BenchmarkStatus {
        running,
        message: if running {
            "Benchmark in progress".into()
        } else {
            "Idle".into()
        },
    })
}