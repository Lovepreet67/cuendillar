use crate::database::Entry;
use crate::database::memtable::Memtable;

mod vector_memtable_test;

pub fn memtable_test_insert_and_find(memtable: &mut impl Memtable) {
    memtable.insert(
        Entry::Row {
            key: b"id1",
            value: b"value1",
        },
        1,
    );
    memtable.insert(
        Entry::Row {
            key: b"id2",
            value: b"value2",
        },
        2,
    );
    memtable.insert(
        Entry::Row {
            key: b"id3",
            value: b"value3",
        },
        3,
    );
    assert_eq!(
        memtable.find(b"id1").unwrap(),
        Some(Entry::Row {
            key: b"id1",
            value: b"value1",
        })
    );
    assert_eq!(
        memtable.find("id2".as_bytes()).unwrap(),
        Some(Entry::Row {
            key: b"id2",
            value: b"value2",
        })
    );
    assert_eq!(
        memtable.find("id3".as_bytes()).unwrap(),
        Some(Entry::Row {
            key: b"id3",
            value: b"value3",
        })
    );
}

pub fn memtable_test_delete(memtable: &mut impl Memtable) {
    memtable.insert(
        Entry::Row {
            key: b"id1",
            value: b"value1",
        },
        0,
    );
    memtable.insert(
        Entry::Row {
            key: b"id2",
            value: b"value2",
        },
        1,
    );
    assert_eq!(
        memtable.find(b"id1").unwrap(),
        Some(Entry::Row {
            key: b"id1",
            value: b"value1",
        })
    );
    assert_eq!(
        memtable.find("id2".as_bytes()).unwrap(),
        Some(Entry::Row {
            key: b"id2",
            value: b"value2",
        })
    );
    memtable.insert(Entry::Tombstone { key: b"id2" }, 2);
    assert_eq!(
        memtable.find("id2".as_bytes()),
        Ok(Some(Entry::Tombstone { key: b"id2" }))
    );
}

pub fn memtable_test_iterator(memtable: &mut impl Memtable) {
    memtable.insert(
        Entry::Row {
            key: b"id1",
            value: b"value1",
        },
        0,
    );
    memtable.insert(
        Entry::Row {
            key: b"id2",
            value: b"value2",
        },
        1,
    );
    memtable.insert(
        Entry::Row {
            key: b"id3",
            value: b"value3",
        },
        2,
    );
    assert_eq!(
        memtable.find(b"id1").unwrap(),
        Some(Entry::Row {
            key: b"id1",
            value: b"value1",
        })
    );
    // testing iterator
    let items = memtable.iter().collect::<Vec<Entry>>();
    assert_eq!(
        items,
        vec![
            Entry::Row {
                key: b"id1",
                value: b"value1",
            },
            Entry::Row {
                key: b"id2",
                value: b"value2",
            },
            Entry::Row {
                key: b"id3",
                value: b"value3",
            },
        ]
    )
}
