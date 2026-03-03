use crate::database::memtable::{
    hash_memtable::HashMetable,
    tests::{memtable_test_delete, memtable_test_insert_and_find, memtable_test_iterator},
};

#[test]
pub fn hash_memtable_test_insert_and_find() {
    let mut memtable = HashMetable::new(None);
    memtable_test_insert_and_find(&mut memtable);
}
#[test]
pub fn hash_memtable_test_iterator() {
    let mut memtable = HashMetable::new(None);
    memtable_test_iterator(&mut memtable);
}

#[test]
pub fn hash_memtable_test_delete() {
    let mut memtable = HashMetable::new(None);
    memtable_test_delete(&mut memtable);
}
