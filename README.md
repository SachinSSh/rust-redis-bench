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
      <th>Parameter</th>
      <th>Description</th>
      <th>Example</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>concurrency</td>
      <td>Number of parallel workers</td>
      <td>20</td>
    </tr>
    <tr>
      <td>duration_secs</td>
      <td>How long the benchmark runs (seconds)</td>
      <td>60</td>
    </tr>
    <tr>
      <td>read_pct</td>
      <td>Percentage of operations that are reads</td>
      <td>70</td>
    </tr>
  </tbody>
</table>





