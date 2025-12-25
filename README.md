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

Cuendillar targets these use cases by providing a lightweight, embeddable storage engine with a simple API and deterministic behavior.

---

## Design Overview

Cuendillar follows an **LSM-tree–based architecture** optimized for fast writes and durable storage.

Key components include:

- **Memtable**  
  In-memory structure for recent writes.

- **Write-Ahead Log (WAL)**  
  Append-only log to ensure durability and enable crash recovery.

- **SSTables**  
  Immutable, sorted on-disk tables generated from flushed memtables.

- **Compaction**  
  Background process that merges SSTables to reduce read amplification and reclaim space.

- **Crash Recovery**  
  Deterministic reconstruction of state from WAL and SSTables on startup.

---

## Goals

- Strong durability guarantees
- Predictable performance characteristics
- Simple, embeddable API
- Rust-native implementation with minimal dependencies
- Easy operational reasoning and failure recovery

---

## Non-Goals

Cuendillar is not intended to be:

- A SQL or relational database
- A distributed or replicated data store
- A replacement for large-scale storage engines
---
## Use Cases

Cuendillar is well-suited for:
- Local-first applications
- Persistent caches
- CLI tools requiring durable state
- Embedded agents and long-running services

---

## Project Status

🚧 **Early development**

Cuendillar is under active development.

---

## Roadmap (High-Level)

- [ ] Write-ahead log and crash recovery
- [ ] Memtable and SSTable formats
- [ ] Basic compaction strategy
- [ ] Iterators and range scans
- [ ] Configuration options
- [ ] Metrics and observability hooks

---

## Philosophy

Cuendillar favors correctness and clarity over complexity.  
Every component will be designed to understandable, inspectable, and reliable — especially in failure scenarios.
