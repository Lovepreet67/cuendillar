use std::{
    fs::{File, remove_dir_all},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    str::FromStr,
    thread::sleep,
    time::Duration,
};

use cuendillar::database::{OwnedEntry, config::DbConfig, db_engine::Engine};

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

pub fn run_workload(engine: &mut Engine, path: &str) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let op = get_operation(&line.unwrap());
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
pub fn db_engine_test() {
    // let dir = TempDir::new().unwrap();
    // let mut engine = Engine::new(Some(dir.path().into())).unwrap();
    let config = DbConfig::get_config().unwrap();
    let mut engine = match Engine::new(config.clone()) {
        Ok(v) => v,
        Err(e) => {
            panic!("{:?}", e)
        }
    };

    let active_workload = std::env::var("ACTIVE_WORKLOAD").unwrap_or_else(|_| "1m".to_owned());
    let active_workload_file = format!("workload/{}.txt", active_workload);
    println!("Active workload is set to {}", active_workload);
    run_workload(&mut engine, &active_workload_file);
    let metrics = engine.metrics;
    let mut output_file = File::options()
        .create(true)
        .write(true)
        .open("./test_result.txt")
        .unwrap();
    writeln!(output_file, "{:?}", metrics).unwrap();
    drop(engine);
    remove_dir_all(&config.root_dir).unwrap();
    sleep(Duration::from_secs(10)); // giving time to remove all dir
}

#[test]
pub fn db_engine_controlled_recovery_test() {
    // let dir = TempDir::new().unwrap();
    // let mut engine = Engine::new(Some(dir.path().into())).unwrap();
    let config = DbConfig::get_config().unwrap();

    let mut counter = 1;
    let mut engine = match Engine::new(config.clone()) {
        Ok(v) => Some(v),
        Err(e) => {
            panic!("{:?}", e)
        }
    };
    let active_workload = std::env::var("ACTIVE_WORKLOAD").unwrap_or_else(|_| "1m".to_owned());
    let active_workload_file = format!("workload/{}.txt", active_workload);
    println!("Active workload is set to {}", active_workload);
    let file = File::open(active_workload_file).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        // after every 100000 operations we will delte the engine and create new
        if counter % 99999 == 0 {
            // we will delte the engine
            drop(engine);
            engine = Some(Engine::new(config.clone()).unwrap());
        }
        let op = get_operation(&line.unwrap());
        execute_op(engine.as_mut().unwrap(), op);
        counter += 1;
    }
    drop(engine);
    remove_dir_all(&config.root_dir).unwrap();
}
