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
        wal.append_log(WALEntry::from_entry(&entry)).unwrap();
    }
    let log_file_ids = wal.get_wals().unwrap();
    assert_eq!(log_file_ids.len(), 1);
    let owned_enteries = wal.read(&log_file_ids[0]).unwrap();
    let read_enteries: Vec<Entry> = owned_enteries.iter().map(|e| e.into()).collect();
    assert_eq!(entries, read_enteries);
}
