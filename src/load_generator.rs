use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::metrics::{MetricsCollector, Sample};

// ─── Public entry point ──────────────────────────────────────────

/// Spawns `concurrency` Tokio tasks that hammer Redis until the
/// deadline or the `running` flag is set to false.
pub async fn run(
    running: Arc<AtomicBool>,
    metrics: Arc<MetricsCollector>,
    redis: ConnectionManager,
    concurrency: u32,
    duration_secs: u64,
    read_pct: u8,
) {
    let deadline = Instant::now() + Duration::from_secs(duration_secs);

    let mut handles = Vec::with_capacity(concurrency as usize);

    for worker_id in 0..concurrency {
        let running = running.clone();
        let metrics = metrics.clone();
        let conn = redis.clone();

        handles.push(tokio::spawn(async move {
            worker(worker_id, running, metrics, conn, deadline, read_pct).await;
        }));
    }

    // Wait for all workers to finish
    for h in handles {
        let _ = h.await;
    }

    // Mark benchmark as finished
    running.store(false, Ordering::SeqCst);
}

// ─── Worker loop ─────────────────────────────────────────────────

async fn worker(
    id: u32,
    running: Arc<AtomicBool>,
    metrics: Arc<MetricsCollector>,
    mut conn: ConnectionManager,
    deadline: Instant,
    read_pct: u8,
) {
    // Each worker gets its own deterministic RNG seeded uniquely.
    let mut rng = StdRng::seed_from_u64(1000 + id as u64);

    while running.load(Ordering::Relaxed) && Instant::now() < deadline {
        let is_read = rng.gen_range(0u8..100) < read_pct;

        if is_read {
            do_read(&mut rng, &metrics, &mut conn).await;
        } else {
            do_write(&mut rng, &metrics, &mut conn).await;
        }
    }
}

// ─── Read operation ──────────────────────────────────────────────

async fn do_read(
    rng: &mut StdRng,
    metrics: &Arc<MetricsCollector>,
    conn: &mut ConnectionManager,
) {
    let t0 = Instant::now();

    // 60 % user lookups, 40 % product lookups
    let (key, endpoint) = if rng.gen_bool(0.6) {
        let id = rng.gen_range(1..=10_000u32);
        (
            format!("user:usr_{:08}", id),
            "GET /api/users/:id",
        )
    } else {
        let id = rng.gen_range(1..=500u32);
        (
            format!("product:prod_{:04}", id),
            "GET /api/products/:id",
        )
    };

    // ── Redis timed section ─────────────────────────────────────
    let t_redis = Instant::now();
    let result: redis::RedisResult<HashMap<String, String>> =
        conn.hgetall(&key).await;
    let redis_us = t_redis.elapsed().as_micros() as u64;
    // ────────────────────────────────────────────────────────────

    let total_us = t0.elapsed().as_micros() as u64;
    let rust_us = total_us.saturating_sub(redis_us);

    metrics.record(Sample {
        endpoint: endpoint.into(),
        redis_us,
        rust_us,
        total_us,
        is_read: true,
        success: result.is_ok() && result.unwrap().len() > 0,
    });
}

// ─── Write operation ─────────────────────────────────────────────

async fn do_write(
    rng: &mut StdRng,
    metrics: &Arc<MetricsCollector>,
    conn: &mut ConnectionManager,
) {
    let t0 = Instant::now();

    if rng.gen_bool(0.5) {
        // ── Create session (SET with TTL) ───────────────────────
        let sess_id = format!("sess_{:08x}", rng.gen::<u32>());
        let user_id = format!("usr_{:08}", rng.gen_range(1..=10_000u32));
        let key = format!("session:{}", sess_id);

        let json = serde_json::json!({
            "id":         sess_id,
            "user_id":    user_id,
            "token":      format!("tok_{:016x}", rng.gen::<u64>()),
            "ip":         format!("10.0.{}.{}", rng.gen_range(0u8..=255),
                                                 rng.gen_range(1u8..=254)),
            "created_at": "2025-06-19T00:00:00Z",
            "ttl_secs":   300,
        })
        .to_string();

        let t_redis = Instant::now();
        let result: redis::RedisResult<()> = redis::cmd("SET")
            .arg(&key)
            .arg(&json)
            .arg("EX")
            .arg(300u64)
            .query_async(conn)
            .await;
        let redis_us = t_redis.elapsed().as_micros() as u64;

        let total_us = t0.elapsed().as_micros() as u64;
        let rust_us = total_us.saturating_sub(redis_us);

        metrics.record(Sample {
            endpoint: "POST /api/sessions".into(),
            redis_us,
            rust_us,
            total_us,
            is_read: false,
            success: result.is_ok(),
        });
    } else {
        // ── Create user (HSET) ──────────────────────────────────
        let i = rng.gen_range(10_001..=99_999u32);
        let id = format!("usr_{:08}", i);
        let key = format!("user:{}", id);

        let t_redis = Instant::now();
        let result: redis::RedisResult<()> = redis::cmd("HSET")
            .arg(&key)
            .arg("id")
            .arg(&id)
            .arg("name")
            .arg("Bench User")
            .arg("email")
            .arg(format!("bench{}@test.com", i))
            .arg("role")
            .arg("viewer")
            .arg("prefs")
            .arg(r#"{"theme":"dark","lang":"en","notifications":false}"#)
            .arg("created_at")
            .arg("2025-06-19T00:00:00Z")
            .query_async(conn)
            .await;
        let redis_us = t_redis.elapsed().as_micros() as u64;

        let total_us = t0.elapsed().as_micros() as u64;
        let rust_us = total_us.saturating_sub(redis_us);

        metrics.record(Sample {
            endpoint: "POST /api/users".into(),
            redis_us,
            rust_us,
            total_us,
            is_read: false,
            success: result.is_ok(),
        });
    }
}