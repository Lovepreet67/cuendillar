use crate::database::memtable::{
    btree_memtable::BTreeMemtable,
    tests::{memtable_test_delete, memtable_test_insert_and_find, memtable_test_iterator},
};

#[test]
pub fn btree_memtable_test_insert_and_find() {
    let mut memtable = BTreeMemtable::new(None);
    memtable_test_insert_and_find(&mut memtable);
}
#[test]
pub fn btree_memtable_test_iterator() {
    let mut memtable = BTreeMemtable::new(None);
    memtable_test_iterator(&mut memtable);
}

#[test]
pub fn btree_memtable_test_delete() {
    let mut memtable = BTreeMemtable::new(None);
    memtable_test_delete(&mut memtable);
}
