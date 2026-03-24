# Integration tests: `tests/db_engine.rs`

These are **crate integration tests** (separate binary per file under `tests/`). They drive the real **`Database`** with workloads from the **`workload/`** directory and use **`DbConfig::get_config()`** (same as production/benches).

### Concurrency: use a single test thread

These tests all use the same **`root_dir`** from your config (typically **`./table`** in `default_config.toml`). If Cargo runs **multiple tests in parallel**, two tests can **open, write, or `remove_dir_all` the same directory at once**, which causes **race conditions** and flaky failures.

**Run with one test thread** by passing **`--test-threads=1`** after `--` (arguments to the test binary):

```bash
cargo test --test db_engine -- --test-threads=1
```

Use the same flag whenever you run **`db_engine`** tests together with other integration tests that share that data directory, or run the full **`cargo test`** suite with **`-- --test-threads=1`** if needed for your setup.

---

## How to run

From the repository root:

```bash
# All integration tests (every `tests/*.rs` file) — single-threaded if they share root_dir
cargo test -- --test-threads=1

# Only this file’s tests (recommended)
cargo test --test db_engine -- --test-threads=1
```

Run a **single** test by name:

```bash
cargo test --test db_engine db_test -- --test-threads=1
cargo test --test db_engine db_controlled_recovery_test -- --test-threads=1
cargo test --test db_engine db_iterator_full_scan_test -- --test-threads=1
cargo test --test db_engine db_iterator_range_test -- --test-threads=1
```

Show output from `println!` / `info!`:

```bash
cargo test --test db_engine -- --test-threads=1 --nocapture
```

---

## Environment variables and parameters

| Variable | Default | Effect |
|----------|---------|--------|
| **`ACTIVE_WORKLOAD`** | **`100k`** | Base name of the workload file: **`workload/{ACTIVE_WORKLOAD}.txt`**. All four tests read this path. |
| **`CONFIG_PATH`** | *(unset)* | Passed through **`DbConfig::get_config()`** → defaults to **`./default_config.toml`** (see `src/database/config/mod.rs`). |

**Workload file on disk:** The tests **open the file at runtime**; if `workload/100k.txt` does not exist, set an existing file, for example:

```bash
ACTIVE_WORKLOAD=10k cargo test --test db_engine
```

The repository currently includes examples such as **`workload/10k.txt`** and **`workload/100.txt`**; adjust **`ACTIVE_WORKLOAD`** to match a file you actually have (or add **`workload/100k.txt`** if you want the default).

---

## Workload line format

Lines are comma-separated. The parser is in **`get_operation`**:

| Opcode | Format | Notes |
|--------|--------|--------|
| **GET** | `GET,<key>,<HIT\|MISS>[,<expected_value>]` | If the third field is **`HIT`**, the fourth field is the expected value. For **`MISS`**, extra fields are ignored. Assertions use **`OwnedEntry::is_equal`**, which compares **keys and values** for rows and **keys** for tombstones (sequence numbers in the expected struct are not compared for equality). |
| **PUT** | `PUT,<key>,<value>` | Values must not contain commas (same limitation as the bench workload parser). |
| **DEL** | `DEL,<key>` | |

Unknown first field → **panic**.

---

## What each test does

### `db_test`

1. Loads config, opens **`Database::new`** once.
2. Replays **`workload/{ACTIVE_WORKLOAD}.txt`** with **`run_workload`** (every line: GET / PUT / DEL + assertions on GET).
3. Drops the DB, **`remove_dir_all(config.root_dir)`**, then sleeps **10 seconds** (to allow filesystem cleanup after directory removal).

### `db_controlled_recovery_test`

Same workload file and **`get_operation`**, but **periodically closes and reopens** the database to exercise recovery:

- Before handling a line, if **`counter % 99999 == 0`** (with **`counter`** starting at **1**), the test **`drop`s** the current `Database` and calls **`Database::new(config.clone())`** again, then continues the trace.

So WAL / on-disk state must be consistent across reopen while replaying the same workload.

### `db_iterator_full_scan_test`

1. Replays the workload while maintaining an in-memory **`BTreeMap`** of the expected final key → value (PUT sets value, DEL sets “deleted”).
2. **`GET` lines** still run through **`execute_op`** (they affect only assertions, not the `BTreeMap`).
3. Full scan: **`db.iter(None, None)`**, collect all **`OwnedEntry`** values, and compare to the map (rows vs tombstones for deleted keys). Expected entries use placeholder **`seq_no`** values that **`is_equal` ignores** for the fields it compares.

### `db_iterator_range_test`

1. Same replay and **`BTreeMap` ground truth** as the full-scan test.
2. Then runs **five fixed key ranges** (see the `ranges` array in the source: start/end byte strings around `key000000` … `key005000`, plus cases where bounds fall outside stored keys).
3. For each range: **`db.iter(Some(start), Some(end))`** (**end is exclusive**, matching the engine API), checks **sorted keys**, length, and **`is_equal`** against **`expected.range(start..=end)`** built from the map.

---

## Side effects and requirements

- **Disk:** Uses **`root_dir`** (and related paths) from your **TOML config**. Tests **delete `config.root_dir`** after success (via **`remove_dir_all`**), so **do not point production data** at the same path.
- **Time:** `db_test` waits **10 seconds** after teardown; recovery and iterator tests can be heavy for large workloads.
- **Dependencies:** Workloads must be consistent with the assertion rules (especially **GET** expectations for HIT/MISS and tombstones).

---

## Quick examples

```bash
# Use the 10k-line workload with printed progress (single thread — shared root_dir)
ACTIVE_WORKLOAD=10k cargo test --test db_engine -- --test-threads=1 --nocapture

# Custom config + workload name
CONFIG_PATH=./configs/always_sync_config.toml ACTIVE_WORKLOAD=10k cargo test --test db_engine -- --test-threads=1
```

---

## Source

Implementation: [`tests/db_engine.rs`](../tests/db_engine.rs). For a similar trace format in benchmarks (without GET assertions), see [`BENCHMARK.md`](BENCHMARK.md).
