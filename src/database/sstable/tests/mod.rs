use crate::database::{
    Entry, OwnedEntry,
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
    let x: Vec<OwnedEntry> = sst.build_memtable(vm.get_id()).unwrap();
    let x_entry: Vec<Entry> = x.iter().map(|e| e.into()).collect();
    let rev_entities: Vec<Entry> = entities.into_iter().rev().collect();
    assert_eq!(x_entry, rev_entities);
}
