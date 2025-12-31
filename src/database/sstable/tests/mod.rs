use crate::database::{
    memtable::{Memtable, vector_memtable::VectorMemtable},
    sstable::SSTable,
    tests::common::Entity,
};
mod default_sstable;
// TODO: these tests should be decoupled from the Vectorized memtable
pub fn sstable_test_encoding_decoding(sst: &mut impl SSTable) {
    let mut vm = VectorMemtable::new(None);
    let entities = vec![
        Entity::new("id1", "name1", 4),
        Entity::new("id2", "name2", 4),
        Entity::new("id3", "name3", 4),
    ];
    for i in entities.clone() {
        vm.insert(i);
    }
    sst.push_memtable(&vm).unwrap();
    let x: Vec<Entity> = sst
        .build_memtable::<Entity, VectorMemtable<Entity>>(vm.get_id())
        .unwrap();
    let rev_entities: Vec<Entity> = entities.into_iter().rev().collect();
    assert_eq!(x, rev_entities);
}
