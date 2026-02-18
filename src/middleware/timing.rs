use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;

/// Tower-compatible middleware that adds two response headers:
///
///   X-Response-Time-Us  — total handler wall time in microseconds
///   Server-Timing       — same value in the standard Server-Timing format
///
/// Also prints a coloured one-liner to stdout for development.
pub async fn timing_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();

    let start = Instant::now();
    let mut response = next.run(req).await;
    let elapsed = start.elapsed();
    let us = elapsed.as_micros();

    // ── Inject response headers ─────────────────────────────────
    if let Ok(val) = us.to_string().parse() {
        response.headers_mut().insert("X-Response-Time-Us", val);
    }

    let server_timing =
        format!("total;dur={:.3}", elapsed.as_secs_f64() * 1000.0);
    if let Ok(val) = server_timing.parse() {
        response.headers_mut().insert("Server-Timing", val);
    }

    // ── Console log ─────────────────────────────────────────────
    let status = response.status().as_u16();
    let colour = match status {
        200..=299 => "\x1b[32m", // green
        400..=499 => "\x1b[33m", // yellow
        _ => "\x1b[31m",        // red
    };
    // Skip noisy static-file / SSE requests
    if path.starts_with("/api/") && !path.contains("/stream") {
        println!(
            "  {colour}{status}\x1b[0m  {method:<5} {path:<35} {us:>7}μs"
        );
    }

    response
}