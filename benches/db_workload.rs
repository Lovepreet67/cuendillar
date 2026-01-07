use criterion::{Criterion, criterion_group, criterion_main};
use cuendillar::database::db_engine::Engine;
use tempfile::TempDir;

use std::{
    env::temp_dir,
    fs::File,
    hint::black_box,
    io::{BufRead, BufReader},
};

pub enum Operation {
    Get(Vec<u8>),
    Put(Vec<u8>, Vec<u8>),
    Del(Vec<u8>),
}

pub fn run_workload(engine: &mut Engine, path: &str) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split(',').collect();
        let op = match parts[0] {
            "GET" => Operation::Get(parts[1].into()),
            "PUT" => Operation::Put(parts[1].into(), parts[2].into()),
            "DEL" => Operation::Del(parts[1].into()),
            _ => panic!("Unknow operation: {}", line),
        };
        execute_op(engine, op);
    }
}

pub fn execute_op(engine: &mut Engine, op: Operation) {
    match op {
        Operation::Get(key) => {
            engine.find(&key).unwrap();
        }
        Operation::Del(key) => engine
            .write(cuendillar::database::Entry::Tombstone { key: &key })
            .unwrap(),
        Operation::Put(key, value) => engine
            .write(cuendillar::database::Entry::Row {
                key: &key,
                value: &value,
            })
            .unwrap(),
    };
}

pub fn small_workload(c: &mut Criterion) {
    c.bench_function("small_workload", |b| {
        b.iter_batched(
            || {
                let dir = TempDir::new().unwrap();
                let mut engine = Engine::new(dir.path().to_str().unwrap()).unwrap();
                (dir, engine)
            },
            |(dir, mut engine)| {
                run_workload(&mut engine, "benches/workload/small.txt");
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

pub fn b10k_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_workloads");

    group
        .sample_size(10)
        .warm_up_time(std::time::Duration::from_secs(1));
    group.bench_function("10k", |b| {
        b.iter_batched(
            || {
                let dir = TempDir::new().unwrap();
                let mut engine = Engine::new(dir.path().to_str().unwrap()).unwrap();
                (dir, engine)
            },
            |(dir, mut engine)| {
                run_workload(&mut engine, "benches/workload/10k.txt");
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, small_workload, b10k_workload);
criterion_main!(benches);
