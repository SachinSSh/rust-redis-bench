# rust-redis-bench

A high-performance Redis benchmarking tool built with Rust. Seeds realistic user and product data into Redis and runs configurable load tests via a REST API or live dashboard.


![Demo](assets/demo.gif)

# Getting Started

### 1. Start Redis

```bash
redis-server
```

### 2. Run the Project

```bash
cd rust-redis-bench
cargo run --release
```

**Expected output:**

```
Seeding 10,000 users and 500 products into redis
Seed complete in 1.2s
Server running at http://localhost:3000
Dashboard at http://localhost:3000
Metrics SSE at http://localhost:3000/api/metrics/stream
```

---

## Running a Benchmark

### Option A — Dashboard

Open your browser and navigate to:

```
http://localhost:3000
```

### Option B — curl

```bash
curl -X POST http://localhost:3000/api/benchmark/start \
  -H "Content-Type: application/json" \
  -d '{
    "concurrency": 20,
    "duration_secs": 60,
    "read_pct": 70
  }'
```

## 
<table>
  <thead>
    <tr>
      <th>Parameter usage</th>
      <th>Description</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Connection pooling</td>
      <td>Redis connection manager with configurable pool size</td>
    </tr>
    <tr>
      <td>Realistic payloads</td>
      <td>JSON bodies with nested fields</td>
    </tr>
    <tr>
      <td>Mixed workload</td>
      <td>Configurable read/write ratio, multiple entity types</td>
    </tr>
    <tr>
      <td>Accurate percentiles</td>
      <td>HdrHistogram (same lib used by Prometheus)</td>
    </tr>
    <tr>
      <td>Middleware-based timing</td>
      <td>Tower layer, same pattern as production Axum apps</td>
    </tr>
     <tr>
      <td>Concurrent load</td>
      <td>Spawns N tokio tasks hittinf the server simulateneously</td>
    </tr>
     <tr>
      <td>Rolling windows</td>
      <td>metrics decay over time</td>
    </tr>
     <tr>
      <td>Error tracking</td>
      <td>displays redis timeouts</td>
    </tr>
  </tbody>
</table>






