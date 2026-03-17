use crate::OwnedEntry;
use crate::database::Entry;
use crate::database::memtable::Memtable;

mod btree_memtable_test;
mod hash_memtable_test;
mod vector_memtable_test;

pub fn memtable_test_insert_and_find(memtable: &mut impl Memtable) {
    memtable.insert(
        Entry::Row {
            seq_no: 0,
            key: b"id1",
            value: b"value1",
        },
        1,
    );
    memtable.insert(
        Entry::Row {
            seq_no: 1,
            key: b"id2",
            value: b"value2",
        },
        2,
    );
    memtable.insert(
        Entry::Row {
            seq_no: 2,
            key: b"id3",
            value: b"value3",
        },
        3,
    );
    assert_eq!(
        memtable.find(b"id1").unwrap(),
        Some(Entry::Row {
            seq_no: 0,
            key: b"id1",
            value: b"value1",
        })
    );
    assert_eq!(
        memtable.find("id2".as_bytes()).unwrap(),
        Some(Entry::Row {
            seq_no: 1,
            key: b"id2",
            value: b"value2",
        })
    );
    assert_eq!(
        memtable.find("id3".as_bytes()).unwrap(),
        Some(Entry::Row {
            seq_no: 2,
            key: b"id3",
            value: b"value3",
        })
    );
}

pub fn memtable_test_delete(memtable: &mut impl Memtable) {
    memtable.insert(
        Entry::Row {
            seq_no: 0,
            key: b"id1",
            value: b"value1",
        },
        0,
    );
    memtable.insert(
        Entry::Row {
            seq_no: 1,
            key: b"id2",
            value: b"value2",
        },
        1,
    );
    assert_eq!(
        memtable.find(b"id1").unwrap(),
        Some(Entry::Row {
            seq_no: 0,
            key: b"id1",
            value: b"value1",
        })
    );
    assert_eq!(
        memtable.find("id2".as_bytes()).unwrap(),
        Some(Entry::Row {
            seq_no: 1,
            key: b"id2",
            value: b"value2",
        })
    );
    memtable.insert(
        Entry::Tombstone {
            seq_no: 2,
            key: b"id2",
        },
        2,
    );
    assert_eq!(
        memtable.find("id2".as_bytes()),
        Ok(Some(Entry::Tombstone {
            seq_no: 2,
            key: b"id2"
        }))
    );
}

pub fn memtable_test_iterator(memtable: &mut impl Memtable) {
    memtable.insert(
        Entry::Row {
            seq_no: 0,
            key: b"id1",
            value: b"value1",
        },
        0,
    );
    memtable.insert(
        Entry::Row {
            seq_no: 1,
            key: b"id2",
            value: b"value2",
        },
        1,
    );
    memtable.insert(
        Entry::Row {
            seq_no: 2,
            key: b"id3",
            value: b"value3",
        },
        2,
    );
    assert_eq!(
        memtable.find(b"id1").unwrap(),
        Some(Entry::Row {
            seq_no: 0,
            key: b"id1",
            value: b"value1",
        })
    );
    // testing iterator
    let items = &memtable.iter(None, None).collect::<Vec<OwnedEntry>>();
    assert_eq!(
        items,
        &vec![
            OwnedEntry::Row {
                seq_no: 0,
                key: b"id1".into(),
                value: b"value1".into(),
            },
            OwnedEntry::Row {
                seq_no: 1,
                key: b"id2".into(),
                value: b"value2".into(),
            },
            OwnedEntry::Row {
                seq_no: 2,
                key: b"id3".into(),
                value: b"value3".into(),
            },
        ]
    )
}
