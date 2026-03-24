# Tuning database configuration

This document explains how configuration is loaded, how it maps to TOML, and how to tune each subsystem. It reflects the structs and validation in this module (`DbConfig` and nested configs).

## How configuration is loaded

1. **Config file path** — Set `CONFIG_PATH` to a TOML file, or the engine uses `./default_config.toml`.
2. **Required keys** — `root_dir` must be present in the TOML file.
3. **Defaults** — `get_dynamic_defaults` supplies baseline values derived from `root_dir` and `sstable_root_dir` (including paths such as `wal.wal_dir` when not overridden).
4. **Merge order** — Serialized defaults are merged first, then your TOML file; later values win.
5. **`sstable_root_dir`** — If omitted, it defaults to `{root_dir}/sstable`.
6. **Validation** — After merge, `DbConfig::validate()` runs. Fix any reported `ConfigError` before expecting a clean start.

### Cross-cutting rule

- **`compaction.root_dir` and `cleaning.root_dir` must be identical.** The validator rejects mismatched directories. In practice, both should match your SSTable root (often the same as top-level `sstable_root_dir`).

---

## Top-level paths

| Field | Role |
|--------|------|
| `root_dir` | Database root; WAL and other layout often hang off this path in defaults. |
| `sstable_root_dir` | Where SSTables live; feeds compaction/cleaner defaults when those sections use the same path. |

TOML keys use snake_case and match the Rust field names.

---

## `[wal]` — `WALConfig`

Durability vs throughput is controlled mainly by sync policy and segment sizing.

| Field | Meaning |
|--------|---------|
| `wal_dir` | Directory for WAL files. |
| `variant` | Implementation (`default` ). |
| `wal_file_size_in_bytes` | Rotate WAL after this size. |
| `wal_max_payload_len_in_bytes` | Max size of a single WAL record. |
| `wal_sync_variant` | `nosync`, `always`, or grouped sync (Explained Later). |

**Validation**

- `wal_max_payload_len_in_bytes` must be **> 0**.
- `wal_file_size_in_bytes` must be **at least 10×** `wal_max_payload_len_in_bytes`.

**TOML examples for `wal_sync_variant`**

```toml
wal_sync_variant = "nosync"
wal_sync_variant = "always"
wal_sync_variant = { groupsync = 100 }
```

`GroupSync(0)` does not fail validation but logs a warning; prefer `nosync` if you do not want grouped sync.

**Tuning**

- **Throughput:** `nosync` (risk of losing recent writes on crash).
- **Durability:** `always` (more `fsync` system calls; lower write throughput).
- **Middle ground:** `groupsync` with a batch size tuned to your workload; larger batches amortize sync cost and limit data loss to group size.
- **Larger `wal_file_size_in_bytes`:** Fewer rotations, larger recovery/replay units on crash.
- **Payload limit:** Set high enough for your largest logical write, but keep the 10× rule vs file size.

---

## `[memtable]` — `MemtableConfig`

| Field | Meaning |
|--------|---------|
| `variant` | `vector`, `btree`, or `hash` |
| `manager_variant` | `default` |
| `max_memtable_size_in_mega_bytes` | Flush trigger size in **MB**. |

**Validation:** `max_memtable_size_in_mega_bytes` must be at least **1** (smaller values are rejected).

**Tuning**

- **Larger memtable:** Fewer flushes and often better write batching; more heap use and longer flush/compaction spikes when a table is frozen.
- **`vector`:** Fast inserts; sorted on flush.
- **`btree`:** Sorted in memory; better if you rely on ordered iteration before flush.
- **`hash`:** Fast point writes; still needs sort before SSTable output.

Align expectations with compaction (`level_base_size` and other params).

---

## `[bloom]` — `BloomConfig`

| Field | Meaning |
|--------|---------|
| `variant` | `default`|
| `bits_per_key` | Bloom bits per key; drives false-positive rate vs memory. |

**Validation:** `bits_per_key` must be **> 0**.

**Tuning**

- **More bits (e.g. 10–12):** Fewer false positives → fewer unnecessary SSTable/block reads; more filter memory per table.
- **Fewer bits:** Less memory; more read amplification from “maybe present” when the key is absent.

---

## `[index]` — `IndexConfig`

| Field | Meaning |
|--------|---------|
| `variant` | `default`  |
| `index_block_min_size` | Minimum accumulated entry size (bytes) before starting a new index block. |

**Validation:** must be **> 0**.

**Tuning**

- **Larger blocks:** Smaller index footprint; slightly coarser block boundaries.
- **Smaller blocks:** Finer block targeting on read; more index metadata.

---

## `[compaction]` — `CompactionConfig`

| Field | Meaning |
|--------|---------|
| `variant` | `leveled` |
| `root_dir` | SSTable directory for compaction I/O (must match `[cleaning].root_dir`). |
| `compaction_interval` | Worker wake interval (**milliseconds**). |
| `min_l0_file_count` | Minimum L0 file count before compaction is considered. |
| `max_l0_file_count_per_cycle` | Cap on how many L0 files are pulled into one compaction cycle (memory: merged inputs may be held in memory during work). |
| `base_entries_per_table` | Base entry; ties into per-level growth (It should be at least equal to the entries in the memtable)|
| `level_entries_growth_factor` | Multiplier for entry capacity across levels (must be greater than 1). |
| `level_base_size` | Base size for leveled layout. This will be used while chekcing if the current level (>0) needs compaction or not|
| `level_size_growth_factor` | Size growth between levels (use a factor greater than 1 in line with leveled LSM practice). |
| `max_level_count` | Number of levels (**1–10** inclusive). |

**Validation (what `CompactionConfig::validate` enforces)**

- `compaction_interval` non-zero  
- `min_l0_file_count` non-zero  
- `base_entries_per_table` greater than 1  
- `level_entries_growth_factor` greater than 1  
- `level_base_size` greater than 1  
- `max_level_count` between **1 and 10** inclusive  

`max_l0_file_count_per_cycle` and `level_size_growth_factor` are not validated here; tune them with workload and memory in mind.

**Tuning**

- **Lower `compaction_interval`:** More frequent checks; can clear L0 backlog faster at the cost of CPU.
- **Higher `min_l0_file_count`:** Fewer early compactions; risk of read amplification if L0 grows too large.
- **`max_l0_file_count_per_cycle`:** Balance against available RAM — large values merge more L0 files per cycle.
- **Growth factors and `max_level_count`:** Control total depth and size progression; deeper trees can store more data with higher read amplification.

---

## `[cleaning]` — `CleanerConfig`

| Field | Meaning |
|--------|---------|
| `root_dir` | Same SSTable root as compaction. |
| `cleaning_interval` | Poll/sleep interval (**milliseconds**) |

**Validation:** `cleaning_interval` must be non-zero.

**Tuning**

- **Lower interval:** Obsolete files removed sooner; more wakeups.
- **Higher interval:** Less background activity; delayed reclaim of disk space.

---

## `[version_manager]` — `VersionManagerConfig`

| Field | Meaning |
|--------|---------|
| `version_manager_sync_mode` | How aggressively manifest/metadata writes are synced: `nosync`, `always`, or grouped (`groupsync` with a count). |

**TOML (same pattern as WAL)**

```toml
version_manager_sync_mode = "nosync"
version_manager_sync_mode = "always"
version_manager_sync_mode = { groupsync = 100 }
```

**Tuning**

- Mirrors the WAL durability trade-off: `always` for strongest metadata durability, `nosync` for performance, `groupsync` as a compromise.

The WAL can be aligned with version-manager sync policy elsewhere in the engine; keep both in mind when you require end-to-end durability.

---


## Quick reference: enum string forms

Serde uses **lowercase** names for these enums in TOML:

- `WALVariant` / `BloomVariant` / `IndexVariant` / `CompactionVariant`: e.g. `default`, `leveled`.
- `MemtableVariant`: `vector`, `btree`, `hash`.
- `MemtableMangerVariant`: `default`.
- Unit sync modes: `nosync`, `always`.
- Tuple variants: `{ groupsync = <u64> }` for grouped sync (see examples above).

When in doubt, match the variant names and shapes used in `configs/group_sync_config.toml` and `configs/always_sync_config.toml` at the repository root.
