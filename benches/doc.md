# Benchmarks

This directory contains **performance benchmarks** for the database
engine. The goal of these benchmarks is to track **performance progress
over time**, not to provide absolute production-grade numbers (yet).

We start with correctness-oriented workloads and gradually scale up.

## Directory Structure

    benches/
    ├── README.md
    ├── workload/
    │   ├── small.txt
    │   ├── 10k.txt
    │   └── (future workloads)
    └── db_workload.rs

## Workload Format

Workloads are defined as **plain text files**.

Each line represents a single operation:

    PUT,key,value
    GET,key
    DEL,key

### Example

    PUT,user1,alice
    PUT,user2,bob
    GET,user1
    DEL,user2
    GET,user2


## Current Workloads

  1. `small.txt` Small workload for correctness + fast feedback.
  2. `10k.txt` Medium workload (\~10k operations) to observe scaling behavior


## Running Benchmarks

``` bash
cargo bench
```