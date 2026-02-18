use std::collections::VecDeque;
use std::time::Instant;

use hdrhistogram::Histogram;
use parking_lot::Mutex;
use serde::Serialize;

use super::percentiles::PercentileSet;
use super::Sample;

// ─── Configuration ───────────────────────────────────────────────

/// How many individual request records we keep for the live feed
const MAX_RECENT_SAMPLES: usize = 200;

/// Aggregate timeline resolution (one point per window)
const TIMELINE_WINDOW_MS: u64 = 500;

/// HdrHistogram range: 1 μs → 60 s, 3 significant figures
const HIST_LOW: u64 = 1;
const HIST_HIGH: u64 = 60_000_000;
const HIST_SIGFIG: u8 = 3;

// ─── Public types ────────────────────────────────────────────────

/// Thread-safe metrics engine.
/// Handlers call `record()`, the SSE stream calls `snapshot()`.
pub struct MetricsCollector {
    inner: Mutex<Inner>,
}

/// A single entry in the live request feed.
#[derive(Debug, Clone, Serialize)]
pub struct SampleRecord {
    pub timestamp_ms: u64,
    pub endpoint: String,
    pub redis_us: u64,
    pub rust_us: u64,
    pub total_us: u64,
    pub is_read: bool,
    pub success: bool,
}

/// One aggregated point on the timeline chart (per 500 ms window).
#[derive(Debug, Clone, Serialize)]
pub struct TimelinePoint {
    pub timestamp_ms: u64,
    pub avg_redis_us: f64,
    pub avg_rust_us: f64,
    pub avg_total_us: f64,
    pub count: u64,
}

/// A bucket in the latency distribution histogram.
#[derive(Debug, Clone, Serialize)]
pub struct DistBucket {
    pub range_start_us: u64,
    pub range_end_us: u64,
    pub count: u64,
}

/// Complete snapshot shipped to the dashboard on every SSE tick.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    // Percentile breakdowns per measurement layer
    pub redis_read: PercentileSet,
    pub redis_write: PercentileSet,
    pub rust_overhead: PercentileSet,
    pub e2e: PercentileSet,

    // Counters
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_reads: u64,
    pub total_writes: u64,
    pub requests_per_sec: f64,
    pub elapsed_secs: f64,

    // Visual data
    pub recent_samples: Vec<SampleRecord>,
    pub timeline: Vec<TimelinePoint>,
    pub distribution: Vec<DistBucket>,
}

// ─── Internal state ──────────────────────────────────────────────

struct Inner {
    // One HdrHistogram per measurement layer
    redis_read_hist: Histogram<u64>,
    redis_write_hist: Histogram<u64>,
    rust_overhead_hist: Histogram<u64>,
    e2e_hist: Histogram<u64>,

    // Counters
    total_requests: u64,
    total_errors: u64,
    total_reads: u64,
    total_writes: u64,

    // Rolling window of recent individual requests
    recent_samples: VecDeque<SampleRecord>,

    // Timeline aggregation
    timeline: Vec<TimelinePoint>,
    current_window: Option<WindowAccumulator>,

    // Wall-clock anchor for elapsed time
    start_time: Option<Instant>,
}

/// Running totals for the current 500 ms timeline window.
struct WindowAccumulator {
    window_start_ms: u64,
    redis_sum: u64,
    rust_sum: u64,
    total_sum: u64,
    count: u64,
}

// ─── MetricsCollector impl ───────────────────────────────────────

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::new()),
        }
    }

    /// Record a single request observation. Called from every handler.
    pub fn record(&self, sample: Sample) {
        self.inner.lock().record(sample);
    }

    /// Wipe all data — called when a new benchmark run starts.
    pub fn reset(&self) {
        *self.inner.lock() = Inner::new();
    }

    /// Produce a read-only snapshot for the dashboard.
    pub fn snapshot(&self) -> MetricsSnapshot {
        self.inner.lock().snapshot()
    }
}

// ─── Inner impl ──────────────────────────────────────────────────

impl Inner {
    fn new() -> Self {
        Self {
            redis_read_hist: Histogram::<u64>::new_with_bounds(
                HIST_LOW, HIST_HIGH, HIST_SIGFIG,
            )
            .expect("histogram creation"),
            redis_write_hist: Histogram::<u64>::new_with_bounds(
                HIST_LOW, HIST_HIGH, HIST_SIGFIG,
            )
            .expect("histogram creation"),
            rust_overhead_hist: Histogram::<u64>::new_with_bounds(
                HIST_LOW, HIST_HIGH, HIST_SIGFIG,
            )
            .expect("histogram creation"),
            e2e_hist: Histogram::<u64>::new_with_bounds(
                HIST_LOW, HIST_HIGH, HIST_SIGFIG,
            )
            .expect("histogram creation"),
            total_requests: 0,
            total_errors: 0,
            total_reads: 0,
            total_writes: 0,
            recent_samples: VecDeque::with_capacity(MAX_RECENT_SAMPLES + 1),
            timeline: Vec::with_capacity(1024),
            current_window: None,
            start_time: None,
        }
    }

    fn record(&mut self, sample: Sample) {
        // Lazily set the anchor on the very first sample
        let start = *self.start_time.get_or_insert_with(Instant::now);
        let elapsed_ms = start.elapsed().as_millis() as u64;

        // ── Counters ────────────────────────────────────────────
        self.total_requests += 1;
        if !sample.success {
            self.total_errors += 1;
        }

        // ── Histograms (clamp to ≥ 1 μs) ───────────────────────
        let redis_us = sample.redis_us.max(1);
        let rust_us = sample.rust_us.max(1);
        let total_us = sample.total_us.max(1);

        if sample.is_read {
            self.total_reads += 1;
            let _ = self.redis_read_hist.record(redis_us);
        } else {
            self.total_writes += 1;
            let _ = self.redis_write_hist.record(redis_us);
        }
        let _ = self.rust_overhead_hist.record(rust_us);
        let _ = self.e2e_hist.record(total_us);

        // ── Timeline aggregation ────────────────────────────────
        self.push_to_timeline(elapsed_ms, redis_us, rust_us, total_us);

        // ── Live request feed ───────────────────────────────────
        self.recent_samples.push_back(SampleRecord {
            timestamp_ms: elapsed_ms,
            endpoint: sample.endpoint,
            redis_us: sample.redis_us,
            rust_us: sample.rust_us,
            total_us: sample.total_us,
            is_read: sample.is_read,
            success: sample.success,
        });
        if self.recent_samples.len() > MAX_RECENT_SAMPLES {
            self.recent_samples.pop_front();
        }
    }

    /// Bucket the sample into the current 500 ms window, or roll over.
    fn push_to_timeline(
        &mut self,
        elapsed_ms: u64,
        redis_us: u64,
        rust_us: u64,
        total_us: u64,
    ) {
        let window_start = (elapsed_ms / TIMELINE_WINDOW_MS) * TIMELINE_WINDOW_MS;

        match &mut self.current_window {
            // Same window — accumulate
            Some(w) if w.window_start_ms == window_start => {
                w.redis_sum += redis_us;
                w.rust_sum += rust_us;
                w.total_sum += total_us;
                w.count += 1;
            }
            // New window — finalize the old one, start fresh
            Some(_) => {
                let old = self.current_window.take().unwrap();
                self.finalize_window(old);
                self.current_window = Some(WindowAccumulator {
                    window_start_ms: window_start,
                    redis_sum: redis_us,
                    rust_sum: rust_us,
                    total_sum: total_us,
                    count: 1,
                });
            }
            // Very first sample
            None => {
                self.current_window = Some(WindowAccumulator {
                    window_start_ms: window_start,
                    redis_sum: redis_us,
                    rust_sum: rust_us,
                    total_sum: total_us,
                    count: 1,
                });
            }
        }
    }

    fn finalize_window(&mut self, w: WindowAccumulator) {
        if w.count == 0 {
            return;
        }
        self.timeline.push(TimelinePoint {
            timestamp_ms: w.window_start_ms,
            avg_redis_us: w.redis_sum as f64 / w.count as f64,
            avg_rust_us: w.rust_sum as f64 / w.count as f64,
            avg_total_us: w.total_sum as f64 / w.count as f64,
            count: w.count,
        });
    }

    /// Build a complete read-only snapshot for the SSE stream.
    fn snapshot(&self) -> MetricsSnapshot {
        let elapsed_secs = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        let rps = if elapsed_secs > 0.0 {
            self.total_requests as f64 / elapsed_secs
        } else {
            0.0
        };

        // Include the current (partial) window in the timeline
        let mut timeline = self.timeline.clone();
        if let Some(w) = &self.current_window {
            if w.count > 0 {
                timeline.push(TimelinePoint {
                    timestamp_ms: w.window_start_ms,
                    avg_redis_us: w.redis_sum as f64 / w.count as f64,
                    avg_rust_us: w.rust_sum as f64 / w.count as f64,
                    avg_total_us: w.total_sum as f64 / w.count as f64,
                    count: w.count,
                });
            }
        }

        MetricsSnapshot {
            redis_read: PercentileSet::from_histogram(&self.redis_read_hist),
            redis_write: PercentileSet::from_histogram(&self.redis_write_hist),
            rust_overhead: PercentileSet::from_histogram(
                &self.rust_overhead_hist,
            ),
            e2e: PercentileSet::from_histogram(&self.e2e_hist),

            total_requests: self.total_requests,
            total_errors: self.total_errors,
            total_reads: self.total_reads,
            total_writes: self.total_writes,
            requests_per_sec: rps,
            elapsed_secs,

            recent_samples: self.recent_samples.iter().cloned().collect(),
            timeline,
            distribution: Self::compute_distribution(&self.e2e_hist),
        }
    }

    // ── Distribution histogram for the bar chart ────────────────

    /// Pre-defined bucket boundaries (μs).  Covers the typical
    /// localhost Redis latency range with good resolution.
    const DIST_BOUNDARIES: &'static [u64] = &[
        25, 50, 100, 150, 200, 300, 400, 500, 750, 1_000, 1_500, 2_000,
        3_000, 5_000, 10_000, 50_000,
    ];

    fn compute_distribution(hist: &Histogram<u64>) -> Vec<DistBucket> {
        if hist.len() == 0 {
            return Vec::new();
        }

        let bounds = Self::DIST_BOUNDARIES;
        let num_buckets = bounds.len() + 1; // +1 for overflow
        let mut counts = vec![0u64; num_buckets];

        // Walk every recorded value in the histogram and bucket it
        for iv in hist.iter_recorded() {
            let val = iv.value_iterated_to();
            let cnt = iv.count_at_value();

            // binary_search gives us the first boundary >= val
            let idx = match bounds.binary_search(&val) {
                Ok(i) => i,        // val == boundary  → bucket i
                Err(i) => i,       // val < boundary[i] → bucket i
            };
            let idx = idx.min(bounds.len()); // clamp for overflow
            counts[idx] += cnt;
        }

        // Convert to output structs, skipping empty buckets
        let mut result = Vec::with_capacity(num_buckets);
        let mut prev = 0u64;
        for (i, &boundary) in bounds.iter().enumerate() {
            if counts[i] > 0 {
                result.push(DistBucket {
                    range_start_us: prev,
                    range_end_us: boundary,
                    count: counts[i],
                });
            }
            prev = boundary;
        }
        // Overflow bucket
        if counts[bounds.len()] > 0 {
            result.push(DistBucket {
                range_start_us: *bounds.last().unwrap(),
                range_end_us: hist.max(),
                count: counts[bounds.len()],
            });
        }

        result
    }
}