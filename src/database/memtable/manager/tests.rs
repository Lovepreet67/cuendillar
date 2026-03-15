use std::{collections::VecDeque, sync::Arc};

use uuid::Uuid;

use crate::database::{
    Entry,
    iterator::DatabaseIterator,
    memtable::{
        btree_memtable::BTreeMemtable,
        manager::{MemtableManager, default_manager::DefaultManger},
    },
};

fn generator()
-> Arc<dyn Fn(Option<Uuid>) -> Box<dyn crate::database::memtable::Memtable> + Send + Sync> {
    Arc::new(|id| Box::new(BTreeMemtable::new(id)))
}

fn row<'a>(seq: u64, key: &'a [u8], value: &'a [u8]) -> Entry<'a> {
    Entry::Row {
        seq_no: seq,
        key,
        value,
    }
}

fn tomb(seq: u64, key: &[u8]) -> Entry {
    Entry::Tombstone { seq_no: seq, key }
}

fn setup_manager() -> DefaultManger {
    DefaultManger::intialize(
        Box::new(BTreeMemtable::new(None)),
        VecDeque::new(),
        100,
        generator(),
    )
}

#[test]
fn test_insert_and_find_active() {
    let mut manager = setup_manager();

    manager.insert(row(1, b"k1", b"v1"), 0).unwrap();

    let result = manager.find(b"k1").unwrap();

    assert!(result.is_some());
    assert_eq!(result.unwrap().get_key(), b"k1");
}

#[test]
fn test_find_falls_back_to_immutable() {
    let mut manager = setup_manager();

    manager.insert(row(1, b"k1", b"v1"), 0).unwrap();

    let id = Uuid::new_v4();
    manager.rotate(id).unwrap();

    let result = manager.find(b"k1").unwrap();

    assert!(result.is_some());
    assert_eq!(result.unwrap().get_key(), b"k1");
}

#[test]
fn test_active_overrides_immutable() {
    let mut manager = setup_manager();

    manager.insert(row(1, b"k1", b"old"), 0).unwrap();

    manager.rotate(Uuid::new_v4()).unwrap();

    manager.insert(row(2, b"k1", b"new"), 0).unwrap();

    let result = manager.find(b"k1").unwrap().unwrap();

    assert_eq!(result.get_seq_no(), 2);
}

#[test]
fn test_rotation_moves_memtable() {
    let mut manager = setup_manager();

    manager.insert(row(1, b"k1", b"v1"), 0).unwrap();

    let id = Uuid::new_v4();
    manager.rotate(id).unwrap();

    assert!(manager.get_memtable_to_push().is_some());
}

#[test]
fn test_iteration_across_tables() {
    let mut manager = setup_manager();

    manager.insert(row(1, b"a", b"1"), 0).unwrap();
    manager.insert(row(1, b"b", b"2"), 0).unwrap();

    manager.rotate(Uuid::new_v4()).unwrap();

    manager.insert(row(2, b"d", b"3"), 0).unwrap();
    manager.rotate(Uuid::new_v4()).unwrap();

    manager.insert(row(2, b"c", b"3"), 0).unwrap();

    let items = manager.iter();

    let keys: Vec<Vec<u8>> = items.as_iterator().map(|e| e.get_key().to_vec()).collect();
    eprintln!("{:?}", keys);

    assert_eq!(keys[0], b"a".to_vec());
    assert_eq!(keys[1], b"b".to_vec());
    assert_eq!(keys[2], b"c".to_vec());
    assert_eq!(keys[3], b"d".to_vec());
}

#[test]
fn test_require_rotation() {
    let mut manager = DefaultManger::intialize(
        Box::new(BTreeMemtable::new(None)),
        VecDeque::new(),
        0, // force rotation
        generator(),
    );

    manager.insert(row(1, b"k1", b"value"), 0).unwrap();

    assert!(manager.require_rotation());
}

#[test]
fn test_get_memtable_to_push() {
    let mut manager = setup_manager();

    manager.insert(row(1, b"k1", b"v1"), 0).unwrap();

    let id = Uuid::new_v4();
    manager.rotate(id).unwrap();

    let memtable = manager.get_memtable_to_push();

    assert!(memtable.is_some());
}

#[test]
fn test_mark_pushed_success() {
    let mut manager = setup_manager();

    manager.insert(row(1, b"k1", b"v1"), 0).unwrap();

    let id = Uuid::new_v4();
    manager.rotate(id).unwrap();

    let memtable = manager.get_memtable_to_push().unwrap();
    let memtable_id = *memtable.get_id();

    manager.mark_pushed(memtable_id).unwrap();

    assert!(manager.get_memtable_to_push().is_none());
}

#[test]
fn test_mark_pushed_wrong_id() {
    let mut manager = setup_manager();

    manager.insert(row(1, b"k1", b"v1"), 0).unwrap();

    manager.rotate(Uuid::new_v4()).unwrap();

    let wrong_id = Uuid::new_v4();

    let result = manager.mark_pushed(wrong_id);

    assert!(result.is_err());
}
