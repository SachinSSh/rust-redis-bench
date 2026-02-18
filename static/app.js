/* ═══════════════════════════════════════════════════════════════
   RUST ↔ REDIS LATENCY OBSERVATORY — Dashboard Client
   ═══════════════════════════════════════════════════════════════ */

// ─── State ───────────────────────────────────────────────────

let evtSource = null;
let benchRunning = false;

// ─── Chart.js defaults ──────────────────────────────────────

Chart.defaults.color = '#8b8fa3';
Chart.defaults.borderColor = '#2a2d3a';
Chart.defaults.font.family =
  "'Inter', -apple-system, BlinkMacSystemFont, sans-serif";
Chart.defaults.font.size = 11;
Chart.defaults.animation.duration = 300;

// ─── Colours ────────────────────────────────────────────────

const C = {
  blue:    'rgba(59, 130, 246, 1)',
  blueA:   'rgba(59, 130, 246, 0.15)',
  orange:  'rgba(245, 158, 11, 1)',
  orangeA: 'rgba(245, 158, 11, 0.15)',
  green:   'rgba(34, 197, 94, 1)',
  greenA:  'rgba(34, 197, 94, 0.15)',
  red:     'rgba(239, 68, 68, 1)',
  purple:  'rgba(168, 85, 247, 1)',
  cyan:    'rgba(6, 182, 212, 1)',
  cyanA:   'rgba(6, 182, 212, 0.25)',
};

// ═════════════════════════════════════════════════════════════
// 1. TIMELINE CHART — Line chart (E2E, Redis, Rust over time)
// ═════════════════════════════════════════════════════════════

const timelineCtx = document.getElementById('timelineChart').getContext('2d');
const timelineChart = new Chart(timelineCtx, {
  type: 'line',
  data: {
    labels: [],
    datasets: [
      {
        label: 'E2E RTT (μs)',
        data: [],
        borderColor: C.blue,
        backgroundColor: C.blueA,
        fill: true,
        tension: 0.3,
        pointRadius: 0,
        borderWidth: 2,
      },
      {
        label: 'Redis (μs)',
        data: [],
        borderColor: C.orange,
        backgroundColor: C.orangeA,
        fill: true,
        tension: 0.3,
        pointRadius: 0,
        borderWidth: 2,
      },
      {
        label: 'Rust Overhead (μs)',
        data: [],
        borderColor: C.green,
        backgroundColor: C.greenA,
        fill: true,
        tension: 0.3,
        pointRadius: 0,
        borderWidth: 2,
      },
    ],
  },
  options: {
    responsive: true,
    maintainAspectRatio: false,
    interaction: { mode: 'index', intersect: false },
    plugins: {
      legend: { position: 'top', labels: { usePointStyle: true, padding: 20 } },
      tooltip: {
        callbacks: {
          label: (ctx) => `${ctx.dataset.label}: ${Math.round(ctx.raw)} μs`,
        },
      },
    },
    scales: {
      x: {
        title: { display: true, text: 'Time (s)' },
        grid: { display: false },
      },
      y: {
        title: { display: true, text: 'Latency (μs)' },
        beginAtZero: true,
        grid: { color: '#1f2233' },
      },
    },
  },
});

// ═════════════════════════════════════════════════════════════
// 2. PERCENTILE BAR CHART
// ═════════════════════════════════════════════════════════════

const percCtx = document.getElementById('percentileChart').getContext('2d');
const percentileChart = new Chart(percCtx, {
  type: 'bar',
  data: {
    labels: ['P50', 'P95', 'P99'],
    datasets: [
      {
        label: 'Redis Read',
        data: [0, 0, 0],
        backgroundColor: C.orange,
        borderRadius: 4,
      },
      {
        label: 'Redis Write',
        data: [0, 0, 0],
        backgroundColor: C.red,
        borderRadius: 4,
      },
      {
        label: 'Rust Overhead',
        data: [0, 0, 0],
        backgroundColor: C.green,
        borderRadius: 4,
      },
      {
        label: 'E2E',
        data: [0, 0, 0],
        backgroundColor: C.blue,
        borderRadius: 4,
      },
    ],
  },
  options: {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { position: 'top', labels: { usePointStyle: true, padding: 12 } },
      tooltip: {
        callbacks: {
          label: (ctx) => `${ctx.dataset.label}: ${ctx.raw} μs`,
        },
      },
    },
    scales: {
      x: { grid: { display: false } },
      y: {
        title: { display: true, text: 'μs' },
        beginAtZero: true,
        grid: { color: '#1f2233' },
      },
    },
  },
});

// ═════════════════════════════════════════════════════════════
// 3. DISTRIBUTION HISTOGRAM (bar chart)
// ═════════════════════════════════════════════════════════════

const distCtx = document.getElementById('distChart').getContext('2d');
const distChart = new Chart(distCtx, {
  type: 'bar',
  data: {
    labels: [],
    datasets: [
      {
        label: 'Requests',
        data: [],
        backgroundColor: C.cyanA,
        borderColor: C.cyan,
        borderWidth: 1,
        borderRadius: 3,
      },
    ],
  },
  options: {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
      tooltip: {
        callbacks: {
          title: (items) => `${items[0].label} μs`,
          label: (ctx) => `${ctx.raw} requests`,
        },
      },
    },
    scales: {
      x: {
        title: { display: true, text: 'Latency (μs)' },
        grid: { display: false },
      },
      y: {
        title: { display: true, text: 'Count' },
        beginAtZero: true,
        grid: { color: '#1f2233' },
      },
    },
  },
});

// ═════════════════════════════════════════════════════════════
// 4. DONUT CHART — Redis vs Rust time breakdown
// ═════════════════════════════════════════════════════════════

const donutCtx = document.getElementById('donutChart').getContext('2d');
const donutChart = new Chart(donutCtx, {
  type: 'doughnut',
  data: {
    labels: ['Redis Read', 'Redis Write', 'Rust Overhead'],
    datasets: [
      {
        data: [1, 1, 1],
        backgroundColor: [C.orange, C.red, C.green],
        borderColor: '#1a1d29',
        borderWidth: 3,
        hoverOffset: 8,
      },
    ],
  },
  options: {
    responsive: true,
    maintainAspectRatio: false,
    cutout: '60%',
    plugins: {
      legend: {
        position: 'bottom',
        labels: { usePointStyle: true, padding: 16 },
      },
      tooltip: {
        callbacks: {
          label: (ctx) => {
            const total = ctx.dataset.data.reduce((a, b) => a + b, 0);
            const pct = total > 0 ? ((ctx.raw / total) * 100).toFixed(1) : 0;
            return `${ctx.label}: ${Math.round(ctx.raw)} μs avg (${pct}%)`;
          },
        },
      },
    },
  },
});

// ═════════════════════════════════════════════════════════════
// SSE CONNECTION
// ═════════════════════════════════════════════════════════════

function connectSSE() {
  if (evtSource) {
    evtSource.close();
  }

  evtSource = new EventSource('/api/metrics/stream');

  evtSource.onmessage = (event) => {
    try {
      const snap = JSON.parse(event.data);
      updateDashboard(snap);
    } catch (e) {
      console.error('SSE parse error:', e);
    }
  };

  evtSource.onerror = () => {
    console.warn('SSE connection lost, reconnecting in 2s...');
    setTimeout(connectSSE, 2000);
  };
}

// Start SSE immediately on page load
connectSSE();

// ═════════════════════════════════════════════════════════════
// DASHBOARD UPDATE — called on every SSE tick (~500ms)
// ═════════════════════════════════════════════════════════════

function updateDashboard(snap) {
  // ── KPI cards ─────────────────────────────────────────────
  document.getElementById('kpiTotal').textContent =
    snap.total_requests.toLocaleString();

  document.getElementById('kpiRps').innerHTML =
    `${Math.round(snap.requests_per_sec).toLocaleString()} <small>req/s</small>`;

  document.getElementById('kpiRW').textContent =
    `${snap.total_reads.toLocaleString()} / ${snap.total_writes.toLocaleString()}`;

  const errEl = document.getElementById('kpiErrors');
  errEl.textContent = snap.total_errors.toLocaleString();
  errEl.classList.toggle('has-errors', snap.total_errors > 0);

  document.getElementById('kpiElapsed').textContent =
    `${snap.elapsed_secs.toFixed(1)}s`;

  // ── Status indicator ──────────────────────────────────────
  const isActive = snap.total_requests > 0 && snap.requests_per_sec > 5;
  const dot = document.getElementById('statusDot');
  const txt = document.getElementById('statusText');
  if (benchRunning || isActive) {
    dot.className = 'status-dot running';
    txt.textContent = 'RUNNING';
  } else {
    dot.className = 'status-dot idle';
    txt.textContent = snap.total_requests > 0 ? 'FINISHED' : 'IDLE';
  }

  // ── Timeline chart ────────────────────────────────────────
  updateTimeline(snap.timeline);

  // ── Percentile chart ──────────────────────────────────────
  updatePercentiles(snap);

  // ── Distribution chart ────────────────────────────────────
  updateDistribution(snap.distribution);

  // ── Donut chart ───────────────────────────────────────────
  updateDonut(snap);

  // ── Summary table ─────────────────────────────────────────
  updateSummaryTable(snap);

  // ── Live feed ─────────────────────────────────────────────
  updateFeed(snap.recent_samples);
}

// ═════════════════════════════════════════════════════════════
// INDIVIDUAL UPDATE FUNCTIONS
// ═════════════════════════════════════════════════════════════

function updateTimeline(timeline) {
  if (!timeline || timeline.length === 0) return;

  const labels = timeline.map((p) => (p.timestamp_ms / 1000).toFixed(1));
  const e2e   = timeline.map((p) => Math.round(p.avg_total_us));
  const redis = timeline.map((p) => Math.round(p.avg_redis_us));
  const rust  = timeline.map((p) => Math.round(p.avg_rust_us));

  timelineChart.data.labels = labels;
  timelineChart.data.datasets[0].data = e2e;
  timelineChart.data.datasets[1].data = redis;
  timelineChart.data.datasets[2].data = rust;
  timelineChart.update('none');
}

function updatePercentiles(snap) {
  const rr = snap.redis_read;
  const rw = snap.redis_write;
  const ro = snap.rust_overhead;
  const e  = snap.e2e;

  percentileChart.data.datasets[0].data = [rr.p50, rr.p95, rr.p99];
  percentileChart.data.datasets[1].data = [rw.p50, rw.p95, rw.p99];
  percentileChart.data.datasets[2].data = [ro.p50, ro.p95, ro.p99];
  percentileChart.data.datasets[3].data = [e.p50,  e.p95,  e.p99];
  percentileChart.update('none');
}

function updateDistribution(distribution) {
  if (!distribution || distribution.length === 0) return;

  const labels = distribution.map(
    (b) => `${fmtUs(b.range_start_us)}–${fmtUs(b.range_end_us)}`
  );
  const data = distribution.map((b) => b.count);

  distChart.data.labels = labels;
  distChart.data.datasets[0].data = data;
  distChart.update('none');
}

function updateDonut(snap) {
  const readMean  = snap.redis_read.count > 0  ? snap.redis_read.mean  : 0;
  const writeMean = snap.redis_write.count > 0 ? snap.redis_write.mean : 0;
  const rustMean  = snap.rust_overhead.count > 0 ? snap.rust_overhead.mean : 0;

  // Only update if we have data (prevents donut collapsing to zero)
  if (readMean + writeMean + rustMean > 0) {
    donutChart.data.datasets[0].data = [
      Math.round(readMean),
      Math.round(writeMean),
      Math.round(rustMean),
    ];
    donutChart.update('none');
  }
}

function updateSummaryTable(snap) {
  fillRow('rowRedisRead',    snap.redis_read);
  fillRow('rowRedisWrite',   snap.redis_write);
  fillRow('rowRustOverhead', snap.rust_overhead);
  fillRow('rowE2E',          snap.e2e);
}

function fillRow(rowId, ps) {
  const row = document.getElementById(rowId);
  if (!row) return;

  const cells = row.querySelectorAll('td');
  if (ps.count === 0) {
    for (let i = 1; i < cells.length; i++) cells[i].textContent = '–';
    return;
  }

  cells[1].textContent = fmtUs(ps.min);
  cells[2].textContent = fmtUs(Math.round(ps.mean));
  cells[3].textContent = fmtUs(ps.p50);
  cells[4].textContent = fmtUs(ps.p95);
  cells[5].textContent = fmtUs(ps.p99);
  cells[6].textContent = fmtUs(ps.max);
  cells[7].textContent = ps.count.toLocaleString();
}

// ── Live feed ───────────────────────────────────────────────

let lastFeedLen = 0;

function updateFeed(samples) {
  if (!samples || samples.length === 0) return;

  const container = document.getElementById('feedContainer');

  // Only re-render if new samples arrived
  if (samples.length === lastFeedLen) return;
  lastFeedLen = samples.length;

  // Remove placeholder
  const placeholder = container.querySelector('.feed-placeholder');
  if (placeholder) placeholder.remove();

  // Build HTML for the last 80 samples (keeps DOM light)
  const visible = samples.slice(-80);
  const html = visible
    .map((s) => {
      const parts = s.endpoint.split(' ');
      const method = parts[0] || 'GET';
      const path = parts.slice(1).join(' ') || s.endpoint;
      const methodClass = method.toLowerCase();
      const statusClass = s.success ? 'ok' : 'fail';
      const statusText  = s.success ? 'OK' : 'ERR';

      return `<div class="feed-line">
        <span class="feed-method ${methodClass}">${method}</span>
        <span class="feed-path">${path}</span>
        <span class="feed-time">${fmtUs(s.total_us)}</span>
        <span class="feed-status ${statusClass}">${statusText}</span>
      </div>`;
    })
    .join('');

  container.innerHTML = html;

  // Auto-scroll to bottom
  container.scrollTop = container.scrollHeight;
}

// ═════════════════════════════════════════════════════════════
// BENCHMARK CONTROLS
// ═════════════════════════════════════════════════════════════

async function startBenchmark() {
  const concurrency  = parseInt(document.getElementById('concurrency').value, 10) || 10;
  const durationSecs = parseInt(document.getElementById('duration').value, 10) || 30;
  const readPct      = parseInt(document.getElementById('readPct').value, 10);

  try {
    const res = await fetch('/api/benchmark/start', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        concurrency,
        duration_secs: durationSecs,
        read_pct: readPct,
      }),
    });

    if (!res.ok) {
      const err = await res.json();
      alert(`Failed to start: ${err.error || res.statusText}`);
      return;
    }

    benchRunning = true;
    document.getElementById('btnStart').disabled = true;
    document.getElementById('btnStop').disabled = false;

    // Auto-stop UI after duration + buffer
    setTimeout(() => {
      benchRunning = false;
      document.getElementById('btnStart').disabled = false;
      document.getElementById('btnStop').disabled = true;
    }, (durationSecs + 2) * 1000);

  } catch (e) {
    alert(`Network error: ${e.message}`);
  }
}

async function stopBenchmark() {
  try {
    await fetch('/api/benchmark/stop', { method: 'POST' });
  } catch (e) {
    console.error('Stop error:', e);
  }

  benchRunning = false;
  document.getElementById('btnStart').disabled = false;
  document.getElementById('btnStop').disabled = true;
}

function resetDashboard() {
  // Reset all charts to empty
  timelineChart.data.labels = [];
  timelineChart.data.datasets.forEach((ds) => (ds.data = []));
  timelineChart.update('none');

  percentileChart.data.datasets.forEach((ds) => (ds.data = [0, 0, 0]));
  percentileChart.update('none');

  distChart.data.labels = [];
  distChart.data.datasets[0].data = [];
  distChart.update('none');

  donutChart.data.datasets[0].data = [1, 1, 1];
  donutChart.update('none');

  // Reset KPIs
  document.getElementById('kpiTotal').textContent = '0';
  document.getElementById('kpiRps').innerHTML = '0 <small>req/s</small>';
  document.getElementById('kpiRW').textContent = '0 / 0';
  document.getElementById('kpiErrors').textContent = '0';
  document.getElementById('kpiElapsed').textContent = '0.0s';

  // Reset table
  ['rowRedisRead', 'rowRedisWrite', 'rowRustOverhead', 'rowE2E'].forEach((id) => {
    const cells = document.getElementById(id).querySelectorAll('td');
    for (let i = 1; i < cells.length; i++) cells[i].textContent = '–';
  });

  // Reset feed
  document.getElementById('feedContainer').innerHTML =
    '<div class="feed-placeholder">Waiting for benchmark to start...</div>';
  lastFeedLen = 0;

  // Reset status
  document.getElementById('statusDot').className = 'status-dot idle';
  document.getElementById('statusText').textContent = 'IDLE';
}

// ═════════════════════════════════════════════════════════════
// UTILITY
// ═════════════════════════════════════════════════════════════

/**
 * Format microseconds into a human-readable string.
 *   < 1000       → "142μs"
 *   1000–999999  → "1.42ms"
 *   >= 1000000   → "1.42s"
 */
function fmtUs(us) {
  if (us === undefined || us === null) return '–';
  if (us < 1000) return `${us}μs`;
  if (us < 1_000_000) return `${(us / 1000).toFixed(2)}ms`;
  return `${(us / 1_000_000).toFixed(2)}s`;
}