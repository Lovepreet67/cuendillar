# Cuendillar

Cuendillar is an embedded, persistent key–value storage engine written in Rust.  
It is designed to preserve application state safely and predictably across time, without requiring an external database.

Inspired by *cuendillar* (heartstone) — a material that cannot be broken or degraded — the project focuses on durability, immutability, and crash safety.

---

## Motivation

Many applications need reliable local state:
- Checkpoints and offsets
- Persistent caches
- Offline-first or embedded applications

Cuendillar targets these use cases by providing a lightweight, embeddable storage engine with a simple API .

---

## Design Overview

Cuendillar follows an **LSM-tree–based architecture** optimized for fast writes and durable storage.

Key components include:

- **Memtable** — in-memory structure for recent writes  
- **Write-Ahead Log (WAL)** — append-only durability layer  
- **SSTables** — immutable sorted files on disk  
- **Compaction** — background merge process  
- **Crash Recovery** — deterministic rebuild from WAL + SSTables  

---

## Features

- Durable writes with configurable WAL sync modes
- Pluggable memtable implementations (btree, vector, hash)
- Bloom filters for read optimization
- Sorted iteration and range scans
- Background compaction and cleaning
- Configurable LSM-tree layout
- RocksDB-style benchmarking support

---

## Quick Start

```rust
use cuendillar::{Database, DbConfig};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = DbConfig::get_config()?;
    let db = Database::new(config)?;

    db.put(b"key", b"value")?;
    
    if let Some(entry) = db.get(b"key")? {
        println!("{:?}", entry);
    }

    Ok(())
}
```
---

## Cuendillar library guide (third-party entry points)

This section describes the **stable surface** exposed from `lib.rs` for applications and bindings.

For engine tuning, see [`docs/CONFIG_TUNING.md`](docs/CONFIG_TUNING.md). For benchmarks, see [`BENCHMARK.md`](BENCHMARK.md).


### Add as a dependency

**Path (local development)**

```toml
[dependencies]
cuendillar = { path = "../cuendillar" }
```

**Crates.io** — use the published name and version when the crate is released.



### Crate root re-exports


The following names are available directly under `cuendillar::`:

| Name | Role |
|------|------|
| **`Database`** | Main handle: open, get, put, delete, range iterator. |
| **`DbConfig`** | Full engine configuration (paths, WAL, memtable, bloom, index, compaction, cleaner, version manager). |
| **`EngineError`** | Error type returned by `Database` operations and `Database::new`. |
| **`OwnedEntry`** | Owned key–value or tombstone returned by `get` and iterators. |
| **`DatabaseIterator`** | Trait implemented by the boxed iterator from `Database::iter`. |
| **`config`** | Module re-export; same as `cuendillar::config` for nested config types (`wal_config`, `memtable_config`, …). |

Submodules such as `database::db_engine` remain **crate-private**; depend only on the items above unless you fork the crate.

---

### Configuration

1. **File** — By default, `DbConfig::get_config()` reads `./default_config.toml`. Override with the **`CONFIG_PATH`** environment variable.
2. **Programmatic defaults** — `DbConfig::get_dynamic_defaults(root_dir, sstable_root_dir)` fills in path-dependent defaults; merge with your own `Figment` / `serde` layer if you do not use a TOML file.
3. **Validation** — Call `config.validate()` before use, or rely on `get_config()` which validates after merge.

```rust
use cuendillar::{Database, DbConfig};
use std::sync::Arc;

let config = DbConfig::get_config()?;
let db = Database::new(config)?;
```

---

### Database

`Database` is **`Clone`**; clones share the same underlying engine (`Arc` + `RwLock`).

### Opening

- **`Database::new(config: Arc<DbConfig>) -> Result<Self, EngineError>`**  
  Opens or creates storage under the configured directories, replays the WAL, and starts background workers as implemented by the engine.

### `Reads and writes`

| Method | Signature (simplified) | Notes |
|--------|-------------------------|--------|
| **`get`** | `fn get(&self, key: &[u8]) -> Result<Option<OwnedEntry>, EngineError>` | Shared read lock on the engine. |
| **`put`** | `fn put(&self, key: &[u8], value: &[u8]) -> Result<u64, EngineError>` | WAL + memtable; returns a sequence number. **Empty `value` is a tombstone** (logical delete). |
| **`delete`** | `fn delete(&self, key: &[u8]) -> Result<u64, EngineError>` | Writes a tombstone (same as `put` with empty value). |
| **`iter`** | `fn iter(&self, start: Option<&[u8]>, end: Option<&[u8]>) -> Result<Box<dyn DatabaseIterator>, EngineError>` | Inclusive start, **exclusive** end. Full range: `iter(None, None)`. If both bounds are `Some` and `start > end`, returns **`EngineError::InvalidRange`**. The read lock is held only while building the iterator. |

### `Tombstones and deletes`

- **`put(key, &[])`** and **`delete(key)`** both record deletion markers; physical removal happens during compaction.
- **`get`** returns **`Some(OwnedEntry::Tombstone { .. })`** when the latest visible version for that key is a tombstone, **`Some(OwnedEntry::Row { .. })`** when the key has a value, and **`None`** when the key is absent. Application code usually treats tombstones like a missing key for business logic.

---

### OwnedEntry

Enum of:

- **`Row { seq_no, key, value }`** — live key–value.
- **`Tombstone { seq_no, key }`** — deleted key at that sequence.

Helpers include **`get_key()`**, **`get_seq_no()`**, **`encode` / `decode`** for a binary record layout, and **`Debug`**.

---

### DatabaseIterator

Returned as **`Box<dyn DatabaseIterator>`**. The trait provides:

- **`peek`**, **`next_owned`**, **`first_entry`**, **`last_entry`** (see `database/iterator` for slice vs owned semantics).
- **`as_iterator()`** — adapter to `Iterator<Item = OwnedEntry>`.

`Box<dyn DatabaseIterator>` also implements **`Iterator<Item = OwnedEntry>`** (delegating to **`next_owned`**), for example:

```rust
let mut it = db.iter(Some(b"a"), Some(b"z"))?;
while let Some(entry) = it.next() {
    let _key = entry.get_key();
}
```

---

### EngineError

```text
General
Internal(String)
PosionError          // RwLock poisoned
IoError(std::io::Error)
InvalidRange         // bad iterator bounds
```

Implements **`Debug`** (not **`Error`** / **`Display`** ). For interoperability, map with **`format!("{:?}", err)`** or wrap in your application error type.

---

## Threading and async

The handle is designed for **shared access across threads** via `Clone` and interior mutability on the engine. Individual method contracts (e.g. how much true concurrency you get on writes) follow the current `RwLock` usage inside the engine. There is **no async API** in the public crate root; run blocking calls on a thread pool if needed.

---

## Stability

Public types and methods on **`Database`** and the re-exports listed above are the intended integration surface. Internal modules may change between versions. For reproducible workloads and CLI-style benchmarks, see the **`db_bench_rocksdb_compatible`** bench and `benches/doc.md`.

## Example application (path dependency)

The workspace member **`examples/cuendillar_example_kv`** is an **interactive `kv>` shell** (and optional one-shot subcommands) that depends on **`cuendillar`** like an external crate (`path = "../.."`). It covers config loading, CRUD, scans, and bulk load. See [`/examples/cuendillar_example_kv/README.md`](/examples/cuendillar_example_kv/README.md).

## Benchmarks

Cuendillar provides three benchmarking binaries:

| Benchmark | Purpose |
|----------|--------|
| `db_workload_operation` | Trace replay with latency histograms |
| `db_workload_operation_summerize` | Lightweight summary report |
| `db_bench_rocksdb_compatible` | RocksDB-style benchmarks |

### Example

```bash
cargo bench --bench db_bench_rocksdb_compatible --   --benchmarks=fillrandom,readrandom   --num=1000000   --seed=1
```

### Benchmark Snapshot (2026-03-23)

| Dataset | Write Throughput | Read Throughput |
|--------|-----------------|-----------------|
| 100M | ~296K ops/s | ~10K ops/s |
| 50M  | ~297K ops/s | ~12K ops/s |
| 30M  | ~308K ops/s | ~115K ops/s |
| 10M  | ~307K ops/s | ~149K ops/s |
| 1M   | ~340K ops/s | ~557K ops/s |


For more details you can see [`FULL_REPORT`](./Benchmark.md),[`BENCHMARK_DETAILS`](./docs/BENCHMARK.md) and [`ROCKS_DB_BENCHMARK_DETAILS`](./docs/ROCKS_DB_BENCHMARK.md).



## Testing

Run integration tests (single-threaded due to shared DB directory):

```bash
cargo test -- --test-threads=1
```

Or specific:

```bash
cargo test --test db_engine -- --test-threads=1
```
For more details you can refer to [`INTEGRATION_TEST`](./docs/DB_ENGINE_INTEGERATION_TEST.md).

---

## Configuration Highlights

Important rules:

- `compaction.root_dir == cleaning.root_dir`
- WAL file size ≥ 10× max payload
- memtable ≥ 1MB

Key tunables:

| Area | Impact |
|------|--------|
| WAL sync | durability vs performance |
| Memtable size | write batching vs memory |
| Bloom bits | memory vs read amplification |

See full guide: [`docs/CONFIG_TUNING.md`](./docs/CONFIG_TUNING.md)

---

## Project Structure

```
src/                 # Core engine
benches/             # Benchmark implementations
tests/               # Integration tests
docs/                # Documentation
configs/             # Example configs
bench_result/        # Benchmark outputs
examples/            # Demo applications
```

---

## Use Cases

- Embedded systems
- Local-first apps
- Persistent caches
- CLI tools
- Background agents

---
## Documentation

- Configuration guide → [`docs/CONFIG_TUNING.md`](./docs/CONFIG_TUNING.md)
- Benchmark details → [`docs/BENCHMARK.md`](./docs/BENCHMARK.md)
- Benchmark Snapshot → [`BENCHMARK.md`](BENCHMARK.md)
- RocksDB comparison → [`docs/ROCKS_DB_BENCHMARK.md`](./docs/ROCKS_DB_BENCHMARK.md)
- Integration tests → [`docs/DB_ENGINE_INTEGRATION_TEST.md`](./docs/DB_ENGINE_INTEGRATION_TEST.md)
