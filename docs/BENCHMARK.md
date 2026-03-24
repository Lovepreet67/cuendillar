# Benchmarks

Cuendillar declares three Criterion-free binaries (`harness = false` in `Cargo.toml`). Each loads **`DbConfig`** via `DbConfig::get_config()`:

- Config file: **`CONFIG_PATH`** environment variable, or **`./default_config.toml`** if unset.
- See [`CONFIG_TUNING.md`](CONFIG_TUNING.md) for tuning the engine.

---

## Summary

| Cargo bench name | Source | Purpose |
|------------------|--------|---------|
| **`db_workload_operation`** | `benches/db_operation_latency.rs` | Replay a trace file of `GET` / `PUT` / `DEL`; HDR histograms; SVG + CSV + HTML under `bench_result/`. |
| **`db_workload_operation_summerize`** | `benches/db_operation_latency_summerize.rs` | Same trace format; text report with throughput, optional failure capture, panic handling; lighter output. |
| **`db_bench_rocksdb_compatible`** | `benches/db_bench_rocksdb_compatible/main.rs` | RocksDB `db_bench`-style phases (`fillrandom`, `readrandom`, `iteratorscan`, `recovery`); CLI flags after `--`. |

---

## 1. `db_workload_operation`

**Run**

```bash
ACTIVE_WORKLOAD=10k cargo bench --bench db_workload_operation
```

**Parameters**

| Input | Meaning |
|--------|---------|
| **`ACTIVE_WORKLOAD`** | Base name of the workload file. Default: **`10k`**. The program opens **`workload/{ACTIVE_WORKLOAD}.txt`** (e.g. `workload/10k.txt`, `workload/100k.txt`). |

**Workload file format**

- One operation per line, comma-separated.
- **`GET,<key>`** — extra columns after the key (e.g. `HIT`, expected value) are **ignored**; only `parts[1]` is used as the key bytes.
- **`PUT,<key>,<value>`** — key and value are the second and third fields (values must not contain commas, or parsing will split them).
- **`DEL,<key>`**
- Unknown first field: **panic**.

**Behavior**

1. Loads `DbConfig`, opens `Database::new`.
2. **Warm-up:** sleeps **10 seconds** (no operations).
3. Replays the file sequentially; records **per-operation latency in microseconds** in separate HDR histograms for PUT / GET / DEL.
4. Prints p50 / p95 / p99 / max to stdout for each op type.
5. Writes under **`bench_result/<UTC-date>/<ACTIVE_WORKLOAD>/`**:
   - `put.svg`, `get.svg`, `del.svg` (plotters bar-style charts),
   - `put_hist.csv`, `get_hist.csv`, `del_hist.csv` (`latency_us,count`),
   - `report.html` embedding the SVGs.
6. Drops the DB and runs **`remove_dir_all("./table")`** — hardcoded path; use a config whose `root_dir` is `./table` or adjust the code if your data directory differs.

**Tracing:** initializes `tracing_subscriber` at **debug** level.

---

## 2. `db_workload_operation_summerize`

**Run**

```bash
ACTIVE_WORKLOAD=10k cargo bench --bench db_workload_operation_summerize
```

**Parameters**

| Input | Meaning |
|--------|---------|
| **`ACTIVE_WORKLOAD`** | Same as above; reads **`workload/{ACTIVE_WORKLOAD}.txt`**. Default **`10k`**. |
| **`CONFIG_PATH`** | Logged at the top of the text report (for reproducibility). |

**Workload file format**

Same opcode rules as **`db_workload_operation`**. On error, the operation returns `Err`; failed ops **do not** add a latency sample.

**Behavior**

1. Loads config, opens DB.
2. **Warm-up:** **5 seconds** sleep.
3. Runs the workload inside **`catch_unwind`** so panics are caught.
4. On each successful op, increments **`total_ops`**. Every **1000** successful ops, sleeps **3 ms** (artificial pacing).
5. On first **`Err`** from an operation or parse failure, stops and writes failure metadata (op index, line, error) into the report.
6. Writes **`bench_result/<UTC-date>/<ACTIVE_WORKLOAD>/report.txt`**, or **`report_panic.txt`** if the workload run panicked (empty stats in that case).
7. Report includes: config path, workload name, total ops, wall-clock runtime, **throughput (ops/sec)**, and per-op-type histogram summary (count, mean, p50, p95, p99, p999, max). Histogram sections are skipped if that op type had no successful samples.
8. **`remove_dir_all("./table")`** on exit (same hardcoded path caveat).

**Difference vs `db_workload_operation`:** no plots/HTML/CSV; records errors and panics; shorter warm-up; optional throttling every 1k ops.

---

## 3. `db_bench_rocksdb_compatible`

**Run**

```bash
cargo bench --bench db_bench_rocksdb_compatible -- --benchmarks=fillrandom,readrandom --num=10000000 --seed=1
```

**Parameters**

Passed as **`--name=value`** after `--`. Full behavior, defaults, and RocksDB alignment notes live in:

**[`ROCKS_DB_BENCHMARK.md`](ROCKS_DB_BENCHMARK.md)**

Short list: `--benchmarks`, `--num`, `--reads`, `--key_size`, `--value_size`, `--seed`, `--use_existing_db`, `--destroy_db_after`. Appends to **`bench_result/<date>/rd_report.txt`**.

---

## Shared artifacts and paths

| Path | Used by |
|------|---------|
| **`workload/*.txt`** | Both workload replay benches (`ACTIVE_WORKLOAD`). |
| **`bench_result/<YYYY-MM-DD>/...`** | All three (layout differs per bench). |
| **`./table`** | Workload benches **delete this directory** at the end of a run (not read from config). |

---

## Example commands

Workload with the bundled 10k trace and graphs:

```bash
ACTIVE_WORKLOAD=10k cargo bench --bench db_workload_operation
```

Workload summary only:

```bash
ACTIVE_WORKLOAD=10k cargo bench --bench db_workload_operation_summerize
```

Custom config + RocksDB-style bench:

```bash
CONFIG_PATH=./configs/always_sync_config.toml \
  cargo bench --bench db_bench_rocksdb_compatible -- \
  --benchmarks=fillrandom,readrandom --num=50000 --reads=50000 --seed=42
```

