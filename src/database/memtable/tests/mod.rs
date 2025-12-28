use crate::database::memtable::errors::MemtableError;
use crate::database::memtable::{Entry, Memtable};
use crate::database::tests::common::Entity;

mod vector_memtable_test;

pub fn memtable_test_insert_and_find(memtable: &mut impl Memtable<Entity>) {
    memtable.insert(Entity::new("id1", "name1", 4));
    memtable.insert(Entity::new("id2", "name2", 4));
    memtable.insert(Entity::new("id3", "name3", 4));
    assert_eq!(
        memtable.find("id1".as_bytes()).unwrap(),
        &Entity::new("id1", "name1", 4)
    );
    assert_eq!(
        memtable.find("id2".as_bytes()).unwrap(),
        &Entity::new("id2", "name2", 4)
    );
    assert_eq!(
        memtable.find("id3".as_bytes()).unwrap(),
        &Entity::new("id3", "name3", 4)
    );
}

// TODO: update this test after updating the delete api
pub fn memtable_test_delete(memtable: &mut impl Memtable<Entity>) {
    memtable.insert(Entity::new("id1", "name1", 4));
    memtable.insert(Entity::new("id2", "name2", 4));
    assert_eq!(
        memtable.find("id1".as_bytes()).unwrap(),
        &Entity::new("id1", "name1", 4)
    );
    assert_eq!(
        memtable.find("id2".as_bytes()).unwrap(),
        &Entity::new("id2", "name2", 4)
    );
    let mut id2_deleted = Entity::new("id2", "name2", 4);

    id2_deleted.mark_deleted();
    memtable.insert(id2_deleted);
    assert!(
        memtable
            .find("id2".as_bytes())
            .is_err_and(|x| x == MemtableError::Deleted)
    );
}

pub fn memtable_test_iterator(memtable: &mut impl Memtable<Entity>) {
    memtable.insert(Entity::new("id1", "name1", 4));
    memtable.insert(Entity::new("id2", "name2", 4));
    memtable.insert(Entity::new("id3", "name3", 4));
    // testing iterator
    let items = memtable.iter().collect::<Vec<&Entity>>();
    assert_eq!(
        items,
        vec![
            &Entity::new("id3", "name3", 4),
            &Entity::new("id2", "name2", 4),
            &Entity::new("id1", "name1", 4),
        ]
    )
}
