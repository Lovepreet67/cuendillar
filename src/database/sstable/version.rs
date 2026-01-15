use crate::database::{
    OwnedEntry,
    sstable::{errors::SSTableError, metadata::SSTMetadata},
};

#[derive(Clone)]
pub struct Version {
    levels: Vec<Vec<SSTMetadata>>,
}

impl Version {
    pub fn new(levels: Vec<Vec<SSTMetadata>>) -> Self {
        Self { levels }
    }
    pub fn add_l0_table(mut self, table: SSTMetadata) -> Self {
        if self.levels.len() > 0 {
            self.levels[0].push(table);
        } else {
            self.levels.push(vec![table]);
        }
        self
    }
    pub fn find(&self, key: &[u8]) -> Result<Option<OwnedEntry>, SSTableError> {
        // for l0 we will check for each and every sstable
        if self.levels.len() == 0 {
            return Ok(None);
        }
        for metatdata in self.levels[0].iter().rev() {
            // check if the key is in bloom filter and in the range of keys in the current table
            match metatdata.find(key)? {
                Some(val) => return Ok(Some(val)),
                None => {
                    continue;
                }
            }
        }
        // for next levels we will perform the data
        if self.levels.len() < 2 {
            return Ok(None);
        }

        // now for each level we will perform the binary search
        for level in &self.levels[1..] {
            // we can assume that there will be no overlapping between data
            // so there will be only one table which may contain the key
            // find the table which contain the data and check for that table only
            let target_table_metadata = match level.binary_search_by(|table_metadata| {
                if table_metadata.first_key.as_slice() <= key
                    && table_metadata.last_key.as_slice() >= key
                {
                    std::cmp::Ordering::Equal
                } else if table_metadata.last_key.as_slice() < key {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            }) {
                Ok(i) => &level[i],
                _ => {
                    continue;
                }
            };
            // now we will have table which may contain the data in
            match target_table_metadata.find(key)? {
                Some(val) => return Ok(Some(val)),
                None => {
                    continue;
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod test {

    use std::path::PathBuf;

    use tempfile::TempDir;

    use crate::database::{
        Entry,
        memtable::{Memtable, vector_memtable::VectorMemtable},
        sstable::version_manager::VersionManager,
    };

    #[test]
    fn version_test_new_and_find() {
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
        let dir = TempDir::new().unwrap();
        let version_manager = VersionManager::new(PathBuf::from(dir.path()));
        let v1 = version_manager.push_memtable(&vm).unwrap();
        assert_eq!(
            v1.find(b"id3").unwrap(),
            Some(
                Entry::Row {
                    key: b"id3",
                    value: b"value3",
                }
                .into(),
            )
        );
        assert_eq!(
            v1.find(b"id2").unwrap(),
            Some(
                Entry::Row {
                    key: b"id2",
                    value: b"value2",
                }
                .into(),
            )
        );
        assert_eq!(v1.find(b"id345").unwrap(), None);
    }
    #[test]
    fn version_test_add_lo_and_find() {
        let mut vm = VectorMemtable::new(None);
        let entities1 = vec![
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
        for i in entities1.clone() {
            vm.insert(i);
        }
        let dir = TempDir::new().unwrap();
        let mut version_manager = VersionManager::new(PathBuf::from(dir.path()));
        let v1 = version_manager.push_memtable(&vm).unwrap();
        version_manager.push_version(v1);
        let entities2 = vec![
            Entry::Row {
                key: b"id3",
                value: b"2value3",
            },
            Entry::Row {
                key: b"id2",
                value: b"2value2",
            },
        ];
        let mut vm2 = VectorMemtable::new(None);
        for i in entities2.clone() {
            vm2.insert(i);
        }
        let v2 = version_manager.push_memtable(&vm2).unwrap();
        assert_eq!(
            v2.find(b"id3").unwrap(),
            Some(
                Entry::Row {
                    key: b"id3",
                    value: b"2value3",
                }
                .into(),
            )
        );
        assert_eq!(
            v2.find(b"id1").unwrap(),
            Some(
                Entry::Row {
                    key: b"id1",
                    value: b"value1",
                }
                .into(),
            )
        );
        assert_eq!(v2.find(b"id34").unwrap(), None);
    }
}
