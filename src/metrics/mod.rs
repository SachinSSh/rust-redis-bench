pub mod collector;
pub mod percentiles;
pub mod stream;

pub use collector::{MetricsCollector, MetricsSnapshot};

/// A single timing observation recorded by a handler.
/// This is the "write" side — handlers create these and push them in.
#[derive(Debug, Clone)]
pub struct Sample {
    /// e.g. "GET /api/users/:id"
    pub endpoint: String,
    /// Microseconds spent inside the Redis round-trip
    pub redis_us: u64,
    /// Microseconds of Rust overhead (serialization, validation, etc.)
    pub rust_us: u64,
    /// Total handler wall time in microseconds
    pub total_us: u64,
    /// true = read operation, false = write operation
    pub is_read: bool,
    /// false when the request hit a not-found or Redis error
    pub success: bool,
}