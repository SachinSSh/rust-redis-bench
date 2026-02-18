use hdrhistogram::Histogram;
use serde::Serialize;

/// A complete percentile breakdown for one measurement layer.
/// Serialized straight into the SSE JSON and into the summary table.
#[derive(Debug, Clone, Serialize)]
pub struct PercentileSet {
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub p999: u64,
    pub count: u64,
}

impl PercentileSet {
    /// Extract a full percentile set from an HdrHistogram.
    /// Returns zeroed values if the histogram is empty.
    pub fn from_histogram(hist: &Histogram<u64>) -> Self {
        if hist.len() == 0 {
            return Self::empty();
        }

        Self {
            min: hist.min(),
            max: hist.max(),
            mean: hist.mean(),
            p50: hist.value_at_percentile(50.0),
            p95: hist.value_at_percentile(95.0),
            p99: hist.value_at_percentile(99.0),
            p999: hist.value_at_percentile(99.9),
            count: hist.len(),
        }
    }

    /// All-zero placeholder used before any samples are recorded.
    pub fn empty() -> Self {
        Self {
            min: 0,
            max: 0,
            mean: 0.0,
            p50: 0,
            p95: 0,
            p99: 0,
            p999: 0,
            count: 0,
        }
    }

    /// Convenience: is this set backed by at least one observation?
    pub fn has_data(&self) -> bool {
        self.count > 0
    }
}