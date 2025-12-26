use crate::database::memtable::{Entry, Memtable, errors::MemtableError};

pub struct VectorMemtable<K>
where
    K: Entry,
{
    store: Vec<K>,
}

impl<K> VectorMemtable<K>
where
    K: Entry,
{
    pub fn new() -> Self {
        Self { store: Vec::new() }
    }
}

impl<K> Memtable<K> for VectorMemtable<K>
where
    K: Entry,
{
    fn insert(&mut self, e: K) {
        self.store.push(e);
    }
    fn delete(&mut self, mut e: K) {
        e.mark_deleted();
        self.store.push(e);
    }
    fn find(&self, key: &[u8]) -> Result<&K, MemtableError> {
        for element in self.store.iter().rev() {
            if element.get_key() == key {
                if element.is_deleted() {
                    return Err(MemtableError::NotFound);
                }
                return Ok(element);
            }
        }
        return Err(MemtableError::NotFound);
    }
}

#[cfg(test)]
mod test {
    use crate::database::memtable::{
        Entry, Memtable, errors::MemtableError, vector_memetable::VectorMemtable,
    };
    #[test]
    pub fn test_vector_memetable() {
        #[derive(Debug, PartialEq)]
        struct Entity {
            id: String,
            name: String,
            class: u8,
            deleted: bool,
        }
        impl Entity {
            pub fn new(id: &str, name: &str, class: u8) -> Self {
                Self {
                    id: id.to_string(),
                    name: name.to_string(),
                    class,
                    deleted: false,
                }
            }
        }
        impl Entry for Entity {
            fn get_key(&self) -> &[u8] {
                self.id.as_bytes()
            }
            fn is_deleted(&self) -> bool {
                self.deleted
            }
            fn mark_deleted(&mut self) {
                self.deleted = true;
            }
        }
        let mut vm = VectorMemtable::new();
        vm.insert(Entity::new("id1", "name1", 4));
        vm.insert(Entity::new("id2", "name2", 4));
        vm.insert(Entity::new("id3", "name3", 4));
        assert_eq!(
            vm.find("id1".as_bytes()).unwrap(),
            &Entity::new("id1", "name1", 4)
        );
        assert_eq!(
            vm.find("id2".as_bytes()).unwrap(),
            &Entity::new("id2", "name2", 4)
        );
        assert_eq!(
            vm.find("id3".as_bytes()).unwrap(),
            &Entity::new("id3", "name3", 4)
        );
        vm.insert(Entity::new("id2", "name2updated", 4));
        assert_eq!(
            vm.find("id2".as_bytes()).unwrap(),
            &Entity::new("id2", "name2updated", 4)
        );
        vm.insert(Entity::new("id2", "name2", 4));
        assert_eq!(
            vm.find("id2".as_bytes()).unwrap(),
            &Entity::new("id2", "name2", 4)
        );
        assert!(vm.find("id7".as_bytes()).is_err());
    }
}
