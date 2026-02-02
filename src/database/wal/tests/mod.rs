use std::{fs::read_dir, path::PathBuf, str::FromStr};

use crate::database::{
    Entry,
    wal::{WAL, wal_entry::WALEntry},
};
mod default_wal;

pub fn test_wal(wal: &mut impl WAL) {
    let entries = vec![
        Entry::Row {
            key: b"id3",
            value: b"value3",
        },
        Entry::Row {
            key: b"id2",
            value: b"value2",
        },
        Entry::Row {
            key: b"id1",
            value: b"value1",
        },
    ];

    for entry in entries.clone() {
        wal.append_log(&entry).unwrap();
    }
    // let owned_enteries = wal.read(&log_file_ids[0]).unwrap();
    // let read_enteries: Vec<Entry> = owned_enteries.iter().map(|e| e.into()).collect();
    // assert_eq!(entries, read_enteries);
}

#[test]
fn temp() {
    let wal_dir = PathBuf::from_str("./workload").unwrap();
    // we will check for the file and read its last entry
    let dir_enteries = read_dir(&wal_dir).unwrap();
    let mut files = vec![];
    for dir_entry in dir_enteries {
        let dir_entry = dir_entry.unwrap();
        if dir_entry.path().is_dir() {
            continue;
        }
        files.push(dir_entry.path());
    }
    files.sort();

    println!("{:?}", files);
}
