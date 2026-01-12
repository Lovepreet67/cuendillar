use crate::database::{
    Entry,
    memtable::{Memtable, vector_memtable::VectorMemtable},
    sstable::SSTable,
};
mod default_sstable;
// TODO: these tests should be decoupled from the Vectorized memtable
pub fn sstable_test_encoding_decoding(sst: &mut impl SSTable) {
    let mut vm = VectorMemtable::new(None);
    let entities = vec![
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
    for i in entities.clone() {
        vm.insert(i);
    }
    sst.push_memtable(&vm).unwrap();
    assert_eq!(
        sst.find(b"id3").unwrap(),
        Some(
            Entry::Row {
                key: b"id3",
                value: b"value3",
            }
            .into(),
        )
    );
    assert_eq!(
        sst.find(b"id2").unwrap(),
        Some(
            Entry::Row {
                key: b"id2",
                value: b"value2",
            }
            .into(),
        )
    );
    assert_eq!(sst.find(b"id345").unwrap(), None);
}
