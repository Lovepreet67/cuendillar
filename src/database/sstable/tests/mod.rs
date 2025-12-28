use std::{fs::remove_dir_all, path::PathBuf, str::FromStr};

use crate::database::{
    memtable::{Memtable, vector_memtable::VectorMemtable},
    sstable::SSTable,
    tests::common::Entity,
};
// TODO: these tests should be decoupled from the Vectorized memtable
#[test]
pub fn sstable_test_encoding_decoding() {
    let sstable_root = "./tables";
    let mut vm = VectorMemtable::new(None);
    let entities = vec![
        Entity::new("id1", "name1", 4),
        Entity::new("id2", "name2", 4),
        Entity::new("id3", "name3", 4),
    ];
    for i in entities.clone() {
        vm.insert(i);
    }
    let mut sst = SSTable::new(PathBuf::from_str(sstable_root).unwrap()).unwrap();
    sst.push_memtable(&vm).unwrap();
    let x: Vec<Entity> = sst
        .build_memtable::<Entity, VectorMemtable<Entity>>(vm.get_id())
        .unwrap();
    let rev_entities: Vec<Entity> = entities.into_iter().rev().collect();
    assert_eq!(x, rev_entities);
    remove_dir_all(sstable_root).unwrap();
}
