use std::{
    fs::{File, remove_dir_all},
    io::{BufRead, BufReader},
    thread::sleep,
    time::Duration,
};

use cuendillar::{Database, OwnedEntry, config::DbConfig};
use tracing::info;
#[derive(Debug)]
pub enum Operation {
    Get(Vec<u8>, bool, Vec<u8>),
    Put(Vec<u8>, Vec<u8>),
    Del(Vec<u8>),
}

pub fn get_operation(line: &str) -> Operation {
    let parts: Vec<&str> = line.split(',').collect();
    match parts[0] {
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
    }
}

pub fn run_workload(db: &Database, path: &str) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let op = get_operation(&line.unwrap());
        execute_op(db, op);
    }
}

pub fn execute_op(db: &Database, op: Operation) {
    match op {
        Operation::Get(key, hit, value) => {
            let x = db.get(&key).unwrap();
            if !hit {
                if let Some(x) = x {
                    assert!(x.is_equal(&OwnedEntry::Tombstone { seq_no: 0, key }));
                }
            } else {
                let x = x.unwrap();
                assert!(x.is_equal(&OwnedEntry::Row {
                    seq_no: 1,
                    key,
                    value
                }));
            }
        }
        Operation::Del(key) => {
            db.delete(&key).unwrap();
        }
        Operation::Put(key, value) => {
            db.put(&key, &value).unwrap();
        }
    };
}
#[test]
pub fn db_test() {
    // tracing_subscriber::fmt()
    //     .with_env_filter("debug")
    //     .try_init()
    //     .unwrap();
    // let dir = TempDir::new().unwrap();
    // let mut db = db::new(Some(dir.path().into())).unwrap();
    let config = DbConfig::get_config().unwrap();
    let mut db = match Database::new(config.clone()) {
        Ok(v) => v,
        Err(e) => {
            panic!("{:?}", e)
        }
    };

    let active_workload = std::env::var("ACTIVE_WORKLOAD").unwrap_or_else(|_| "100k".to_owned());
    let active_workload_file = format!("workload/{}.txt", active_workload);
    println!("Active workload is set to {}", active_workload);
    run_workload(&mut db, &active_workload_file);
    drop(db);
    remove_dir_all(&config.root_dir).unwrap();
    sleep(Duration::from_secs(10)); // giving time to remove all dir
}

#[test]
pub fn db_controlled_recovery_test() {
    // tracing_subscriber::fmt()
    //     .with_env_filter("debug")
    //     .try_init()
    //     .unwrap();
    // let dir = TempDir::new().unwrap();
    // let mut db = db::new(Some(dir.path().into())).unwrap();
    let config = DbConfig::get_config().unwrap();

    let mut counter = 1;
    let mut db = match Database::new(config.clone()) {
        Ok(v) => Some(v),
        Err(e) => {
            panic!("{:?}", e)
        }
    };
    let active_workload = std::env::var("ACTIVE_WORKLOAD").unwrap_or_else(|_| "100k".to_owned());
    let active_workload_file = format!("workload/{}.txt", active_workload);
    println!("Active workload is set to {}", active_workload);
    let file = File::open(active_workload_file).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        // after every 100000 operations we will delte the db and create new
        if counter % 99999 == 0 {
            // we will delte the db
            drop(db);
            info!("Starting recovery");
            db = Some(Database::new(config.clone()).unwrap());
            info!("recovery complete");
        }
        let op = get_operation(&line.unwrap());
        execute_op(db.as_mut().unwrap(), op);
        counter += 1;
    }
    drop(db);
    remove_dir_all(&config.root_dir).unwrap();
}

use std::collections::BTreeMap;

#[test]
pub fn db_iterator_full_scan_test() {
    let config = DbConfig::get_config().unwrap();

    let db = Database::new(config.clone()).unwrap();

    let active_workload = std::env::var("ACTIVE_WORKLOAD").unwrap_or_else(|_| "100k".to_owned());
    let active_workload_file = format!("workload/{}.txt", active_workload);

    println!("Active workload is set to {}", active_workload);

    let file = File::open(&active_workload_file).unwrap();
    let reader = BufReader::new(file);

    // Ground truth state
    let mut expected: BTreeMap<Vec<u8>, Option<Vec<u8>>> = BTreeMap::new();

    for line in reader.lines() {
        let op = get_operation(&line.unwrap());

        match &op {
            Operation::Put(k, v) => {
                expected.insert(k.clone(), Some(v.clone()));
            }
            Operation::Del(k) => {
                expected.insert(k.clone(), None);
            }
            Operation::Get(_, _, _) => {}
        }

        execute_op(&db, op);
    }

    // Collect iterator output
    let mut iter = db.iter(None, None).unwrap();

    let mut results = Vec::new();

    while let Some(entry) = iter.next_owned() {
        results.push(entry);
    }

    // Build expected entries
    let mut expected_entries = Vec::new();

    for (k, v) in expected {
        match v {
            Some(value) => {
                expected_entries.push(OwnedEntry::Row {
                    seq_no: 1,
                    key: k,
                    value,
                });
            }
            None => {
                expected_entries.push(OwnedEntry::Tombstone { seq_no: 0, key: k });
            }
        }
    }

    assert_eq!(results.len(), expected_entries.len());

    for (a, b) in results.iter().zip(expected_entries.iter()) {
        assert!(a.is_equal(b));
    }

    drop(db);
    remove_dir_all(&config.root_dir).unwrap();
}
