# Benchmark results snapshot (`bench_result/2026-03-23`)

This document describes the performance testing of the cuendillar
---
In this section we will be discussing the rocks_db_compatible benchmarks, you can run the benchmark yourself following instructions in [`docs/ROCKS_DB_BENCHMARK.md`](docs/ROCKS_DB_BENCHMARK.md).

The following result is based on the benchmarks ran on the Macbook M1 machine with 8gb of RAM and 75GB of free SSD.

you can find the actual result in the [`Result`](bench_result/2026-03-23/rd_report.txt) file.

For these runs, the relevant phases are:

- **`fillrandom`** — Random writes over a key space of size **`num`**: each key is `rng.next_u64() % num`, with fixed key and value lengths (below).
- **`readrandom`** — **`1,000,000`** point lookups (`--reads=1_000_000` or default), each key drawn as `rng.next_u64() % num` over the same **`num`** key space as the preceding load.

So **`num`** is both the number of writes in `fillrandom` and the logical key universe for `readrandom`.

---

## Fixed parameters for these runs

| Parameter | Value |
|-----------|--------|
| **Key size** | **16 bytes** (`--key_size=16`) |
| **Value size** | **100 bytes** (`--value_size=100`) |
| **Engine configuration** | **Default** — `DbConfig` from **`./default_config.toml`** (or whatever **`CONFIG_PATH`** pointed to if you overrode it; the on-disk snapshot is documented here as default-config runs). |
| **Read phase** | **1,000,000** random `get` operations per `readrandom` block in the report. |

---

## Important note: time between write and read

Between the **write** phase (`fillrandom`) and the **read** phase (`readrandom`), **enough wall time was allowed** for background work—especially **compaction**—to make progress or settle, so the read numbers are not measuring a database still in the middle of the same burst of flushes as the tail end of the load. Exact idle duration was not recorded in `rd_report.txt`; treat this as an intentional methodology choice when comparing to other systems or to same-day runs without a pause. 

---

## Report

Each block is a **Cuendillar Benchmark Report** / **General Report** with:

- **Total Ops / Total Runtime / AVG Throughput** — Aggregates for that single appended section (one benchmark phase).
- **Name** — `fillrandom` or `readrandom`.
- **Count** — Number of operations in that phase (writes for fill, reads for read).
- **Duration** — Wall time for that phase (seconds, integer in the file).
- **p50, p95, p99, p99.9, Max** — Per-operation latency in **microseconds (µs)** from an HDR Histogram (see `benches/db_bench_rocksdb_compatible/report.rs`).

Higher percentiles and **Max** often reflect compaction, I/O stalls, or lock contention, not steady-state microsecond behavior.

---

## Data summarized from `rd_report.txt`

The system is benchmarked across multiple dataset sizes: **100M, 50M, 30M, 10M, and 1M keys**.

For each dataset size `x`, the benchmark performs:
1. Start with an empty database
2. Insert `x` keys (`fillrandom`)
3. Allow a cooldown period for background processes (e.g., compaction)
4. Execute 1,000,000 random reads over the same keyspace (`readrandom`)

Each Benchmark is explained below


### 100M key space 

| Phase | Count | Wall time (reported) | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|----------------------|----------------|-------------------------------------|
| fillrandom | 100,000,000 | ~336 s | ~296,890 ops/s | 3 / 3 / 6 / 20 / 243,199 |
| readrandom | 1,000,000 | ~97 s | ~10,316 ops/s | 105 / 143 / 161 / 225 / 14,543 |

### 50M key space

| Phase | Count | Wall time | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|-----------|----------------|-------------------------------------|
| fillrandom | 50,000,000 | ~168 s | ~297,486 ops/s | 3 / 3 / 6 / 23 / 340,735 |
| readrandom | 1,000,000 | ~77 s | ~12937 ops/s | 103 / 144 / 162 / 211 / 77,19 |

### 30M key space

| Phase | Count | Wall time | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|-----------|----------------|-------------------------------------|
| fillrandom | 30,000,000 | ~97 s | ~308,027 ops/s | 2 / 3 / 5 / 12 / 234,111 |
| readrandom | 1,000,000 | ~8 s | ~115,069 ops/s | 4 / 8 / 114 / 143 / 1325 |

### 10M key space

| Phase | Count | Wall time | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|-----------|----------------|-------------------------------------|
| fillrandom | 10,000,000 | ~32 s | ~307,067 ops/s | 3 / 3 / 5 / 10 / 18,399 |
| readrandom | 1,000,000 | ~6.71 s | ~149,009 ops/s | 3 / 5 / 101 / 128 / 1476 |

### 1M key space

| Phase | Count | Wall time | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|-----------|----------------|-------------------------------------|
| fillrandom | 1,000,000 | ~2.93 s | ~340,149 ops/s | 2 / 3 / 5 / 9 / 5,907 |
| readrandom | 1,000,000 | ~1.79 s | ~557,846 ops/s | 2 / 3 / 3 / 4 / 211 |

### Config Used
The same rd_report.txt file contain the config which was used during the benchmarking
---

## Interpretation (high level)

- **Write throughput (`fillrandom`)** scales in the **hundreds of thousands of ops/s** in these traces, with median latencies of **a few µs**; **Max** in hundreds of ms is consistent with occasional **memtable rotation, flush, or compaction** spikes.
- **Read throughput (`readrandom`)** depends strongly on **dataset size**, **SSTable / L0 layout**, **caching**, and **whether compaction had time to reduce read amplification**—which is why the note about **waiting between write and read** matters.
- **Smaller `num` (10M / 30M)** in this log shows **much faster** read phases than **100M** for the paired runs, which is expected for a smaller key space and fewer files to probe.

---

## Reproducing

Example shape (adjust `num` and ensure your process or shell waits between benchmarks if you want to mirror the compaction-settle methodology):

```bash
cargo bench --bench db_bench_rocksdb_compatible -- \
  --benchmarks=fillrandom \
  --num=10000000 \
  --key_size=16 \
  --value_size=100 \
  --seed=<fixed>

# After sufficient idle time for background compaction:

cargo bench --bench db_bench_rocksdb_compatible -- \
  --benchmarks=readrandom \
  --num=10000000 \
  --reads=1000000 \
  --key_size=16 \
  --use_existing_db=true \
  --seed=<same as fill>
```

Use **`--use_existing_db=true`** on the read leg so the data directory from the fill is preserved (see README). Reports append under **`bench_result/<YYYY-MM-DD>/rd_report.txt`**.

For more detials you can refer to run the benchmark yourself following instructions in [`docs/ROCKS_DB_BENCHMARK.md`](docs/ROCKS_DB_BENCHMARK.md).

