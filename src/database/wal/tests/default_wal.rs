use std::{fs::File, io::Write};

use crate::database::{
    config::wal_config::WALConfig,
    wal::{
        WAL,
        default_wal::DefaultWAL,
        tests::{
            test_wal_append, test_wal_corruption, test_wal_empty, test_wal_flush,
            test_wal_invalid_file_name, test_wal_max_palyload, test_wal_read,
            test_wal_recovery_on_new, test_wal_rotation,
        },
    },
};

#[test]
fn test_default_wal_append() {
    let (config, _root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    test_wal_append(&mut wal);
}

#[test]
fn test_default_wal_read() {
    let (config, _root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    test_wal_read(&mut wal);
}

#[test]
fn test_default_wal_rotation() {
    let (config, _root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    test_wal_rotation(&mut wal, config.wal_file_size_in_bytes);
}

#[test]
fn test_default_wal_flush() {
    let (config, _root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    test_wal_flush(&mut wal, config.wal_file_size_in_bytes);
}

#[test]
fn test_default_wal_corruption() {
    let (config, _root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    test_wal_corruption(&mut wal, config.wal_file_size_in_bytes);
}

#[test]
fn test_default_wal_invalid_file_name() {
    let (config, root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    test_wal_invalid_file_name(&mut wal, root_dir.path());
}

#[test]
fn test_default_wal_max_payload() {
    let (config, _root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    test_wal_max_palyload(&mut wal, config.wal_max_payload_len_in_bytes);
}

#[test]
fn test_default_wal_empty() {
    let (config, _root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    test_wal_empty(&mut wal);
}

#[test]
pub fn test_default_wal_recovery_on_new() {
    let (config, _root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();
    for i in 0..100 as i32 {
        wal.append_log(&i.to_be_bytes()).unwrap();
    }
    drop(wal);
    let mut wal2 = DefaultWAL::new(config).unwrap();
    test_wal_recovery_on_new(&mut wal2);
}

#[test]
pub fn test_default_wal_truncation() {
    let (config, root_dir) = &WALConfig::get_default_wal_test_config();
    let mut wal = DefaultWAL::new(config).unwrap();

    // We need to keep this small in order to make sure that 0.wal is still active
    for i in 0..10 as usize {
        wal.append_log(&i.to_be_bytes()).unwrap();
    }

    // simulate crash mid write
    let mut f = File::options()
        .append(true)
        .open(root_dir.path().join("0.wal"))
        .unwrap();
    // this value will be trimmed at end
    f.write_all(b"garbage").unwrap();
    f.sync_all().unwrap();

    drop(wal);
    let mut wal = DefaultWAL::new(config).unwrap();
    wal.append_log(&(10 as usize).to_be_bytes()).unwrap();
    let wal_iter = wal.read(0).unwrap();
    for i in wal_iter.enumerate() {
        let item = i.1.unwrap();
        assert_eq!(&item.1, &i.0.to_be_bytes());
    }
}
