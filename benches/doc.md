# Benchmarks

This directory contains benchmark binaries for Cuendillar.

## Available Benchmarks

1. `db_workload_operation`
- Path: `benches/db_operation_latency.rs`
- Replays a mixed workload file (`PUT/GET/DEL`) and reports latency histograms.

2. `db_workload_operation_summerize`
- Path: `benches/db_operation_latency_summerize.rs`
- Similar mixed-workload runner with summary-oriented reporting.

3. `db_bench_rocksdb_compatible`
- Path: `benches/db_bench_rocksdb_compatible.rs`
- Implements a RocksDB `db_bench`-compatible flow for:
- `fillrandom`
- `readrandom`
- Designed for apples-to-apples comparison with RocksDB baseline runs.

## RocksDB-Compatible Benchmark

The `db_bench_rocksdb_compatible` benchmark mirrors the following RocksDB behaviors:

1. Key RNG model (`Random64` style based on `mt19937_64`).
2. Key encoding format from `GenerateKeyFromInt` (binary prefix + `'0'` padding).
3. `fillrandom` key selection: `rand.Next() % num`.
4. `readrandom` key selection: `rand.Next() % num`.
5. `seed=0` means time-based seed in microseconds.
6. Read phase runs after write phase on existing DB (engine reopen between phases).

Current scope:

1. Matches default `readrandom` behavior (`read_random_exp_range=0`).
2. Single-process benchmark binary; thread parity with RocksDB multithreaded modes is not implemented yet.

## CLI Options (`db_bench_rocksdb_compatible`)

1. `--benchmarks=fillrandom,readrandom`
2. `--num=<u64>` total keys for `fillrandom` and key-space for `readrandom`
3. `--reads=<u64>` read operations in `readrandom` (if `0`, uses `--num`)
4. `--key_size=<usize>`
5. `--value_size=<usize>`
6. `--seed=<u64>` (`0` => time-based seed)
7. `--use_existing_db=<bool>` if `false`, removes configured DB root before run
8. `--destroy_db_after=<bool>` if `true`, removes DB root after run

## Example Commands

Small sanity run:

```bash
cargo bench --bench db_bench_rocksdb_compatible -- \
  --benchmarks=fillrandom,readrandom \
  --num=100000 \
  --reads=100000 \
  --key_size=16 \
  --value_size=100 \
  --seed=1
```

RocksDB-style large run (900M write + 900M read):

```bash
cargo bench --bench db_bench_rocksdb_compatible -- \
  --benchmarks=fillrandom,readrandom \
  --num=900000000 \
  --reads=900000000 \
  --key_size=16 \
  --value_size=100 \
  --seed=1 \
  --use_existing_db=false
```

## Notes for Fair Comparison Against RocksDB

1. Use the same values for `num`, `reads`, `key_size`, `value_size`, and `seed`.
2. Run both systems on the same machine and storage device.
3. Keep background load minimal and pin equivalent durability settings when comparing write latency/throughput.
4. Cuendillar currently emits lock tracing logs; expect noisy stdout unless instrumentation is disabled.
