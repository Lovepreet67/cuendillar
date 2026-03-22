# Benchmark results snapshot (`bench_result/2026-03-22`)

This document describes how the numbers in **`bench_result/2026-03-22/rd_report.txt`** were produced, what they mean, and how to read them.

---

## Which benchmark produced this file?

The append-only report **`rd_report.txt`** is written by the **`db_bench_rocksdb_compatible`** binary (see [`benches/db_bench_rocksdb_compatible/README.md`](benches/db_bench_rocksdb_compatible/README.md) and [`benches/doc.md`](benches/doc.md)).

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

## How to read the report lines

Each block is a **Cuendillar Benchmark Report** / **General Report** with:

- **Total Ops / Total Runtime / AVG Throughput** — Aggregates for that single appended section (one benchmark phase).
- **Name** — `fillrandom` or `readrandom`.
- **Count** — Number of operations in that phase (writes for fill, reads for read).
- **Duration** — Wall time for that phase (seconds, integer in the file).
- **p50, p95, p99, p99.9, Max** — Per-operation latency in **microseconds (µs)** from an HDR Histogram (see `benches/db_bench_rocksdb_compatible/report.rs`).

Higher percentiles and **Max** often reflect compaction, I/O stalls, or lock contention, not steady-state microsecond behavior.

---

## Data summarized from `rd_report.txt`

The source file includes inline comments (`// 100M keys as base`, etc.) grouping runs. Below is a concise summary of each **fillrandom** + **readrandom** pair where both appear in sequence for a given key-space comment.

### 100M key space (first block in file)

| Phase | Count | Wall time (reported) | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|----------------------|----------------|-------------------------------------|
| fillrandom | 100,000,000 | ~343 s | ~291,519 ops/s | 3 / 4 / 6 / 21 / 463,871 |
| readrandom | 1,000,000 | ~69 s | ~14,462 ops/s | 91 / 147 / 181 / 392 / 11,055 |

### 50M key space

| Phase | Count | Wall time | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|-----------|----------------|-------------------------------------|
| fillrandom | 50,000,000 | ~164 s | ~304,817 ops/s | 2 / 4 / 6 / 13 / 182,015 |
| readrandom | 1,000,000 | ~22 s | ~45,476 ops/s | 3 / 115 / 142 / 170 / 1,331 |

### 30M key space

| Phase | Count | Wall time | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|-----------|----------------|-------------------------------------|
| fillrandom | 30,000,000 | ~100 s | ~298,510 ops/s | 3 / 4 / 6 / 12 / 160,255 |
| readrandom | 1,000,000 | ~2.3 s | ~436,983 ops/s | 2 / 4 / 5 / 8 / 970 |

### 10M key space

| Phase | Count | Wall time | Avg throughput | p50 / p95 / p99 / p99.9 / Max (µs) |
|--------|------:|-----------|----------------|-------------------------------------|
| fillrandom | 10,000,000 | ~31 s | ~321,909 ops/s | 2 / 3 / 6 / 9 / 14,831 |
| readrandom | 1,000,000 | ~2.25 s | ~443,468 ops/s | 2 / 4 / 4 / 6 / 69 |

### Additional rows in the same file (same day, appended later)

The log continues with another **100M fillrandom** (~340 s, similar microsecond percentiles) and **several** **readrandom** entries (1M reads each) with **different** throughputs and latencies—for example one run near **~4,211 ops/s** with much higher p50/p99, and others near **~19k–23k ops/s**. Those are **separate** read passes (or different sessions / machine state) against a **large** populated dataset; they are **not** paired in the file with a fresh fill immediately above each read in every case. Use them as additional samples, not as a single controlled A/B row without matching load metadata.

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
