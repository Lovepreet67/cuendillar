use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
};

use cuendillar::database::{OwnedEntry, db_engine::Engine};
use tempfile::TempDir;

pub enum Operation {
    Get(Vec<u8>, bool, Vec<u8>),
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
            "GET" => {
                let hit = parts[2] == "HIT";
                Operation::Get(
                    parts[1].into(),
                    hit,
                    if hit { parts[3].into() } else { vec![] },
                )
            }
            "PUT" => Operation::Put(parts[1].into(), parts[2].into()),
            "DEL" => Operation::Del(parts[1].into()),
            _ => panic!("Unknow operation: {}", line),
        };
        execute_op(engine, op);
    }
}

pub fn execute_op(engine: &mut Engine, op: Operation) {
    match op {
        Operation::Get(key, hit, value) => {
            let x = engine.find(&key).unwrap();
            if !hit {
                if x.is_some() {
                    assert_eq!(x, Some(OwnedEntry::Tombstone { key }));
                }
            } else {
                assert_eq!(x, Some(OwnedEntry::Row { key, value }))
            }
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
#[test]
pub fn b10k_workload() {
    let dir = TempDir::new().unwrap();
    let mut engine = Engine::new(dir.path().to_str().unwrap()).unwrap();
    // let mut engine = Engine::new("./table").unwrap();
    run_workload(&mut engine, "tests/workload/10000.txt");
    let metrics = engine.metrics;
    let mut output_file = File::options()
        .create(true)
        .write(true)
        .open("./test_result.txt")
        .unwrap();
    writeln!(output_file, "{:?}", metrics).unwrap();
}
