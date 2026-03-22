# RocksDB-style `db_bench` compatible benchmark

This binary reproduces key ideas from RocksDB’s `db_bench` (random key generation, `GenerateKeyFromInt`-style keys, similar phase names) so you can compare Cuendillar against RocksDB using aligned parameters.

**Run it**

```bash
cargo bench --bench db_bench_rocksdb_compatible -- [OPTIONS...]
```

`cargo bench` injects `--bench`; the parser ignores it. Arguments after `--` are passed to this program.

**Database settings** come from `DbConfig::get_config()` (same as the main engine): `CONFIG_PATH` or `./default_config.toml`. The benchmark does not override compaction, WAL, or memtable tuning; change those in your TOML for apples-to-apples comparisons.

---

## How it works

### Key generation

- A **Mersenne Twister (`mt19937_64`)** drives randomness (RocksDB-style 64-bit PRNG).
- Keys are built with **`generate_key_from_int`**: the first up-to-8 bytes encode the integer in big-endian order; any remaining key length is padded with ASCII `'0'`. This matches the spirit of RocksDB’s `GenerateKeyFromInt` when `keys_per_prefix_ == 0`.
- **`--seed`**: if non-zero, used as the PRNG seed; if **zero**, the seed is **current time in microseconds** (so runs are non-reproducible unless you set a fixed seed).

### Lifecycle and disk

- Unless **`--use_existing_db=true`**, the configured **`root_dir` is deleted once** at startup (`remove_dir_all`).
- Each benchmark phase creates a **new** `Database` handle (`Database::new`), then drops it when the phase ends. Phases still share the **same on-disk directory**, so a prior phase’s persisted data is visible to later phases in the same process (e.g. `fillrandom` then `readrandom`).
- If **`--destroy_db_after=true`**, `root_dir` is removed **after all benchmarks finish**.

### Phases (`--benchmarks`)

| Name | What it does |
|------|----------------|
| **`fillrandom`** | `num` times: draw `key_rand = rng.next_u64() % num`, build key of `key_size`, value is `value_size` bytes (`b'v'` repeated), `put`. Records per-op latency in an HDR histogram. |
| **`readrandom`** | `reads` times: same key draw modulo **`key_space` = `num`**, `get`. Prints how many values were found. |
| **`iteratorscan`** | `reads` times (here: **number of scans**): pick random start in `[0, num)`, end is `min(start + 10000, num)` — **fixed window of 10_000 keys** in code. Opens an iterator over `[start, end]`, walks to end; histogram records **total time for that full scan** as one sample. `found` in the report is **total entries iterated** across all scans. |
| **`recovery`** | `num` times: time `Database::new(config)` (open/recover), then a probe `get(b"testing_key_may_not_exist")`, drop DB. Each open is one histogram sample. |

Default **`--benchmarks`** is `fillrandom,readrandom` (see `opts_parser.rs`).

### Output

- **Stdout**: human-readable line per phase (micros/op, ops/sec, count, p50/p95/p99/p99.9/max, and optional found count).
- **Report file**: appends to **`bench_result/<UTC-date>/rd_report.txt`** (e.g. `bench_result/2026-03-22/rd_report.txt`).

---

## CLI parameters

All options use `name=value` (no space around `=`). Boolean values: `true` / `false`, `1` / `0`, or common case variants (`True`, `FALSE`, etc.).

| Option | Default | Description |
|--------|---------|-------------|
| **`--benchmarks`** | `fillrandom,readrandom` | Comma-separated list: `fillrandom`, `readrandom`, `iteratorscan`, `recovery`. |
| **`--num`** | `1000000` | **`fillrandom`:** number of writes. **`readrandom` / `iteratorscan`:** key space upper bound (keys are `rng % num`). **`recovery`:** number of open/drop cycles. Must be **> 0** for `fillrandom` / `readrandom` / `iteratorscan` (modulo by zero would panic). |
| **`--reads`** | `1000000` | **`readrandom`:** number of point reads. **`iteratorscan`:** number of range scans. If set to **`0`**, it is replaced with **`num`**. |
| **`--key_size`** | `16` | Key length in bytes (must be **> 0**). |
| **`--value_size`** | `100` | Value length for **`fillrandom`** only. |
| **`--seed`** | `0` | PRNG seed; `0` ⇒ time-based (microseconds). |
| **`--use_existing_db`** | `false` | If `false`, delete `root_dir` before any benchmark. If `true`, keep existing data (useful for read-only phases or reruns). |
| **`--destroy_db_after`** | `false` | If `true`, delete `root_dir` after all phases complete. |

**`-h` / `--help`** prints a short usage summary and exits successfully.

---

## Examples

Small sanity run (reproducible seed):

```bash
cargo bench --bench db_bench_rocksdb_compatible -- \
  --benchmarks=fillrandom,readrandom \
  --num=100000 \
  --reads=100000 \
  --key_size=16 \
  --value_size=100 \
  --seed=1
```

Include iterator scans (note: `--reads` is the **number of scans**; each scan covers up to 10 000 keys in the current implementation):

```bash
cargo bench --bench db_bench_rocksdb_compatible -- \
  --benchmarks=fillrandom,readrandom,iteratorscan \
  --num=1000000 \
  --reads=10000 \
  --seed=1
```

Recovery micro-benchmark:

```bash
cargo bench --bench db_bench_rocksdb_compatible -- \
  --benchmarks=recovery \
  --num=50 \
  --seed=1
```

---

## Comparing with RocksDB

Use the same **`num`**, **`reads`**, **`key_size`**, **`value_size`**, and **`seed`**. Run on comparable hardware and align durability/sync settings via your Cuendillar TOML vs RocksDB options.

---

## Source layout

| File | Role |
|------|------|
| `main.rs` | CLI driver, phase dispatch, DB directory cleanup. |
| `opts_parser.rs` | Argument parsing and defaults. |
| `runners.rs` | PRNG, key generation, `fillrandom` / `readrandom` / `iteratorscan` / `recovery`. |
| `report.rs` | HDR histogram aggregation and `bench_result/.../rd_report.txt`. |
| `constants.rs` | MT19937-64 constants. |
