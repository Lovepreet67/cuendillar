# Cuendillar library guide (third-party entry points)

Cuendillar is an embedded **key–value store** built around an LSM-tree (memtable, WAL, SSTables, compaction). This document describes the **stable surface** exposed from `lib.rs` for applications and bindings.

For engine tuning, see [`database/config/CONFIG_TUNING.md`](database/config/CONFIG_TUNING.md). For benchmarks, see [`../benches/doc.md`](../benches/doc.md).

---

## Add as a dependency

**Path (local development)**

```toml
[dependencies]
cuendillar = { path = "../cuendillar" }
```

**Crates.io** — use the published name and version when the crate is released.

---

## Crate root re-exports

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

## Configuration

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

## `Database`

`Database` is **`Clone`**; clones share the same underlying engine (`Arc` + `RwLock`).

### Opening

- **`Database::new(config: Arc<DbConfig>) -> Result<Self, EngineError>`**  
  Opens or creates storage under the configured directories, replays the WAL, and starts background workers as implemented by the engine.

### Reads and writes

| Method | Signature (simplified) | Notes |
|--------|-------------------------|--------|
| **`get`** | `fn get(&self, key: &[u8]) -> Result<Option<OwnedEntry>, EngineError>` | Shared read lock on the engine. |
| **`put`** | `fn put(&self, key: &[u8], value: &[u8]) -> Result<u64, EngineError>` | WAL + memtable; returns a sequence number. **Empty `value` is a tombstone** (logical delete). |
| **`delete`** | `fn delete(&self, key: &[u8]) -> Result<u64, EngineError>` | Writes a tombstone (same as `put` with empty value). |
| **`iter`** | `fn iter(&self, start: Option<&[u8]>, end: Option<&[u8]>) -> Result<Box<dyn DatabaseIterator>, EngineError>` | Inclusive start, **exclusive** end. Full range: `iter(None, None)`. If both bounds are `Some` and `start > end`, returns **`EngineError::InvalidRange`**. The read lock is held only while building the iterator. |

### Tombstones and deletes

- **`put(key, &[])`** and **`delete(key)`** both record deletion markers; physical removal happens during compaction.
- **`get`** returns **`Some(OwnedEntry::Tombstone { .. })`** when the latest visible version for that key is a tombstone, **`Some(OwnedEntry::Row { .. })`** when the key has a value, and **`None`** when the key is absent. Application code usually treats tombstones like a missing key for business logic.

---

## `OwnedEntry`

Enum of:

- **`Row { seq_no, key, value }`** — live key–value.
- **`Tombstone { seq_no, key }`** — deleted key at that sequence.

Helpers include **`get_key()`**, **`get_seq_no()`**, **`encode` / `decode`** for a binary record layout, and **`Debug`**.

---

## `DatabaseIterator`

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

## `EngineError`

```text
General
Internal(String)
PosionError          // RwLock poisoned
IoError(std::io::Error)
InvalidRange         // bad iterator bounds
```

Implements **`Debug`** (not **`Error`** / **`Display`** today). For interoperability, map with **`format!("{:?}", err)`** or wrap in your application error type.

---

## Threading and async

The handle is designed for **shared access across threads** via `Clone` and interior mutability on the engine. Individual method contracts (e.g. how much true concurrency you get on writes) follow the current `RwLock` usage inside the engine. There is **no async API** in the public crate root; run blocking calls on a thread pool if needed.

---

## Stability

Public types and methods on **`Database`** and the re-exports listed above are the intended integration surface. Internal modules may change between versions. For reproducible workloads and CLI-style benchmarks, see the **`db_bench_rocksdb_compatible`** bench and `benches/doc.md`.

## Example application (path dependency)

The workspace member **`examples/cuendillar_example_kv`** is an **interactive `kv>` shell** (and optional one-shot subcommands) that depends on **`cuendillar`** like an external crate (`path = "../.."`). It covers config loading, CRUD, scans, and bulk load. See [`../examples/cuendillar_example_kv/README.md`](../examples/cuendillar_example_kv/README.md).
