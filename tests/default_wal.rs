use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use cuendillar::database::wal::{WAL, default_wal::DefaultWAL};
use tempfile::TempDir;

pub enum Operation {
    Get(Vec<u8>, bool, Vec<u8>),
    Put(Vec<u8>, Vec<u8>),
    Del(Vec<u8>),
}

pub fn run_workload(mut wal: Box<dyn WAL>, path: &str) {
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
        execute_op(&mut wal, op);
    }
}

pub fn execute_op(wal: &mut Box<dyn WAL>, op: Operation) {
    match op {
        Operation::Get(_key, _hit, _value) => {}
        Operation::Del(key) => {
            let mut payload = Vec::new();
            let entry = cuendillar::database::Entry::Tombstone { key: &key };
            entry.encode(&mut payload).unwrap();
            wal.append_log(&payload).unwrap();
        }
        Operation::Put(key, value) => {
            let mut payload = Vec::new();
            let entry = cuendillar::database::Entry::Row {
                key: &key,
                value: &value,
            };
            entry.encode(&mut payload).unwrap();
            wal.append_log(&payload).unwrap();
        }
    };
}

pub fn verify_wal(wal: Box<dyn WAL>, path: &str) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut wal_iterator = wal.read(0).unwrap();
    for line in reader.lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split(',').collect();
        match parts[0] {
            "GET" => {}
            "PUT" => {
                let mut payload = Vec::new();
                let entry = cuendillar::database::Entry::Row {
                    key: parts[1].as_bytes(),
                    value: parts[2].as_bytes(),
                };
                entry.encode(&mut payload).unwrap();
                let next = wal_iterator.next();
                assert!(next.is_some());
                let (_, recv_payload) = next.unwrap().unwrap();
                assert_eq!(payload, recv_payload);
            }
            "DEL" => {
                let mut payload = Vec::new();
                let entry = cuendillar::database::Entry::Tombstone {
                    key: parts[1].as_bytes(),
                };
                entry.encode(&mut payload).unwrap();
                let next = wal_iterator.next();
                assert!(next.is_some());
                let (_, recv_payload) = next.unwrap().unwrap();
                assert_eq!(payload, recv_payload);
            }
            _ => panic!("Unknow operation: {}", line),
        };
    }
}

#[test]
pub fn default_wal_test() {
    let active_workload = std::env::var("ACTIVE_WORKLOAD").unwrap_or_else(|_| "10k".to_owned());
    let active_workload_file = format!("workload/{}.txt", active_workload);
    println!("Active workload is set to {}", active_workload);
    let root_dir = TempDir::new().unwrap();
    // let root_dir = PathBuf::from_str("./wal").unwrap();
    let default_wal = DefaultWAL::new(root_dir.path().into(), 100).unwrap();
    let wal = Box::new(default_wal);
    run_workload(wal, &active_workload_file);

    // now we will use new wal

    let default_wal2 = DefaultWAL::new(root_dir.path().into(), 100).unwrap();
    let wal2 = Box::new(default_wal2);
    verify_wal(wal2, &active_workload_file);
}
