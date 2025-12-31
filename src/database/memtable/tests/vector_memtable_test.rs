use crate::database::memtable::{
    Memtable,
    tests::{memtable_test_delete, memtable_test_insert_and_find, memtable_test_iterator},
    vector_memtable::VectorMemtable,
};

#[test]
pub fn vector_memtable_test_insert_and_find() {
    let mut memtable = VectorMemtable::new(None);
    memtable_test_insert_and_find(&mut memtable);
}
#[test]
pub fn vector_memtable_test_iterator() {
    let mut memtable = VectorMemtable::new(None);
    memtable_test_iterator(&mut memtable);
}

#[test]
pub fn vector_memtable_test_delete() {
    let mut memtable = VectorMemtable::new(None);
    memtable_test_delete(&mut memtable);
}
