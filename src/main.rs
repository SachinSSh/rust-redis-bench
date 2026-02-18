use std::sync::atomic::AtomicBool;
use std::sync::Arc;

mod handlers;
mod load_generator;
mod metrics;
mod middleware;
mod mock_data;
mod redis_client;
mod server;

/// Shared application state available to every handler via `State<Arc<AppState>>`.
pub struct AppState {
    /// Cloneable async Redis connection (auto-reconnects).
    pub redis: redis::aio::ConnectionManager,

    /// Central metrics engine — handlers push samples, SSE reads snapshots.
    pub metrics: Arc<metrics::MetricsCollector>,

    /// Flag checked by every load-generator worker on each iteration.
    pub load_running: Arc<AtomicBool>,

    /// Handle to the spawned load-generator task so we can await clean shutdown.
    pub load_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

#[tokio::main]
async fn main() {
    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║   🔬  RUST ↔ REDIS LATENCY OBSERVATORY          ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    // ── 1. Connect to Redis ──────────────────────────────────────
    println!("🔌 Connecting to Redis at 127.0.0.1:6379...");
    let redis_conn = redis_client::connect("redis://127.0.0.1:6379/").await;
    println!("   ✓ connected");

    // ── 2. Seed mock data ────────────────────────────────────────
    mock_data::seed(&redis_conn).await;

    // ── 3. Build shared state ────────────────────────────────────
    let state = Arc::new(AppState {
        redis: redis_conn,
        metrics: Arc::new(metrics::MetricsCollector::new()),
        load_running: Arc::new(AtomicBool::new(false)),
        load_handle: tokio::sync::Mutex::new(None),
    });

    // ── 4. Build Axum router ─────────────────────────────────────
    let app = server::create_router(state);

    // ── 5. Bind & serve ──────────────────────────────────────────
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port 3000 — is it already in use?");

    println!();
    println!("Server listening on http://localhost:3000");
    println!("Dashboard       → http://localhost:3000");
    println!("Metrics SSE     → http://localhost:3000/api/metrics/stream");
    println!("Metrics JSON    → http://localhost:3000/api/metrics");
    println!();

    axum::serve(listener, app)
        .await
        .expect("Server exited with error");
}
