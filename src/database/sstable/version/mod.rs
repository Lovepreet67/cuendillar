use std::{
    collections::HashSet,
    fmt::Debug,
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
    sync::OnceLock,
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use tracing::instrument;

use crate::database::{
    OwnedEntry,
    sstable::{
        errors::SSTableError,
        metadata::{
            SSTMetadata, SSTableFooter, SSTableKeyRange, bloom_filter::bloom_factory::BloomFactory,
            index::index_factory::IndexFactory,
        },
        version::version_update::{VersionOperation, VersionUpdate},
    },
};
pub mod version_manager;
pub mod version_update;

#[derive(Clone)]
pub struct Version {
    poisened: bool,
    levels: Vec<Vec<SSTMetadata>>,
    commited_wal_offset: u64,
}
impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("Version");

        for (level_idx, level) in self.levels.iter().enumerate() {
            ds.field(&format!("level_{}", level_idx), level);
        }

        ds.finish()
    }
}

impl Version {
    pub fn new(levels: Vec<Vec<SSTMetadata>>, commited_wal_offset: u64) -> Self {
        Self {
            poisened: false,
            levels,
            commited_wal_offset: commited_wal_offset,
        }
    }
    pub fn get_commited_wal_offset(&self) -> u64 {
        self.commited_wal_offset
    }
    pub fn add_l0_table(mut self, table: SSTMetadata, commited_wal_offset: u64) -> Self {
        if self.levels.len() > 0 {
            self.levels[0].push(table);
        } else {
            self.levels.push(vec![table]);
        }
        self.commited_wal_offset = commited_wal_offset;
        self
    }
    /// INDEX_SAFETY: This function is used by compaction which will only run if the tables in l0 is > 0
    pub fn get_level_tables(&self, level: usize) -> Option<&Vec<SSTMetadata>> {
        if self.levels.len() <= level {
            None
        } else {
            Some(&self.levels[level])
        }
    }
    pub fn get_level_tables_owned(&mut self, level: usize) -> Option<Vec<SSTMetadata>> {
        if self.levels.len() <= level {
            None
        } else {
            Some(std::mem::take(&mut self.levels[level]))
        }
    }
    #[instrument(name = "Version Find", skip(self))]
    pub fn find(&self, key: &[u8]) -> Result<Option<OwnedEntry>, SSTableError> {
        if self.poisened {
            return Err(SSTableError::PoisonedError);
        }
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
            /*
            we can assume that there will be no overlapping between data
            so there will be only one table which may contain the key
            find the table which contain the data and check for that table only
            */
            let target_table_metadata = match level.binary_search_by(|table_metadata| {
                if table_metadata.key_range.first_key.as_slice() <= key
                    && table_metadata.key_range.last_key.as_slice() >= key
                {
                    std::cmp::Ordering::Equal
                } else if table_metadata.key_range.last_key.as_slice() < key {
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

    #[instrument(name = "Apply version Update", skip(self, normalized_update))]
    pub fn apply_update(&mut self, normalized_update: VersionUpdate) -> Result<(), SSTableError> {
        use std::collections::HashMap;

        let mut adds: HashMap<usize, Vec<(usize, SSTMetadata)>> = HashMap::new();
        let mut dels: HashMap<usize, Vec<uuid::Uuid>> = HashMap::new();

        let wal_offset = normalized_update.wal_offset;

        for op in normalized_update.operations {
            match op {
                VersionOperation::Add { .. } => {
                    return Err(SSTableError::General(
                        "Add without metadata found in apply_update".into(),
                    ));
                }
                VersionOperation::AddWithMeta { level, meta, index } => {
                    adds.entry(level as usize)
                        .or_default()
                        .push((index as usize, meta));
                }
                VersionOperation::Del { level, id } => {
                    dels.entry(level as usize).or_default().push(id);
                }
            }
        }

        let max_level = adds.keys().chain(dels.keys()).copied().max().unwrap_or(0);

        if max_level >= self.levels.len() {
            self.levels.resize_with(max_level + 1, Vec::new);
        }

        for (level, ids) in dels {
            if level >= self.levels.len() {
                return Err(SSTableError::General(format!(
                    "Invalid level {} in Del",
                    level
                )));
            }

            let id_set: std::collections::HashSet<_> = ids.into_iter().collect();

            self.levels[level].retain(|meta| !id_set.contains(&meta.id));
        }

        // 4️⃣ Apply inserts (sorted by index to preserve intent)
        for (level, mut entries) in adds {
            let level_vec = &mut self.levels[level];

            // Sort by index to apply deterministically
            entries.sort_by_key(|(index, _)| *index);

            for (index, meta) in entries {
                let insert_index = index.min(level_vec.len());
                level_vec.insert(insert_index, meta);
            }
        }

        self.commited_wal_offset = wal_offset;
        Ok(())
    }
    pub fn encode(&self, writer: &mut impl Write) -> Result<u64, SSTableError> {
        let mut bytes_written: u64 = 0;
        // write last commited offset
        writer.write_u64::<BigEndian>(self.commited_wal_offset)?;
        bytes_written += 8;
        // then number of levels as u16
        writer.write_u16::<BigEndian>(self.levels.len() as u16)?;
        bytes_written += 2;

        // then we will encode each level
        for i in 0..self.levels.len() {
            // number of tables in this version
            writer.write_u64::<BigEndian>(self.levels[i].len() as u64)?;
            bytes_written += 8;

            for table in &self.levels[i] {
                // as uuid is 16 bytes long
                writer.write(table.id.as_bytes())?;
                bytes_written += 16;
            }
        }
        Ok(bytes_written)
    }
    // this function will convert all the add to add with sstmetadata
    #[instrument(name = "Normalize Version Update Operation", skip(update, root_dir))]
    pub fn normalize_version_update_operation(
        update: &mut VersionUpdate,
        root_dir: &Path,
    ) -> Result<(), SSTableError> {
        // first we will remove all the tables which are added first in the changelog and then deleted to pervent uneccessary operatons
        let mut removed_tables = HashSet::new();
        let original_updates = std::mem::take(&mut update.operations);
        update.operations = original_updates
            .into_iter()
            .rev()
            .filter(|operation| match operation {
                VersionOperation::Del { level: _, id } => {
                    removed_tables.insert(id.clone());
                    return false;
                }
                VersionOperation::Add {
                    level: _,
                    id,
                    index: _,
                } => {
                    if removed_tables.contains(id) {
                        return false;
                    } else {
                        return true;
                    }
                }
                // below case will not happen
                VersionOperation::AddWithMeta {
                    level: _,
                    meta,
                    index: _,
                } => {
                    if removed_tables.contains(&meta.id) {
                        return false;
                    } else {
                        return true;
                    }
                }
            })
            .rev()
            .collect();

        for op in &mut update.operations {
            // We only transform Add → AddWithMeta
            let new_op = match op {
                VersionOperation::Add { level, id, index } => {
                    let level = *level;
                    let id = *id;
                    let index = *index;

                    // Build file path
                    let table_file_path = root_dir.join(format!("l{}/{}", level, id));

                    // Open SST file
                    let mut table = File::open(&table_file_path)?;

                    // ---- Read footer ----
                    table.seek(std::io::SeekFrom::End(-32))?;
                    let table_footer = SSTableFooter::deserialize(&mut table)?;

                    // ---- Read bloom filter ----
                    table.seek(std::io::SeekFrom::Start(table_footer.data_block_size))?;

                    let bloom = BloomFactory::deserialize_bloom_filter(&mut table)?;

                    // ---- Read index ----
                    let index_block = IndexFactory::deserialize_index(&mut table)?;

                    // ---- Read key range ----
                    let key_range = SSTableKeyRange::deserialize(&mut table)?;

                    let meta = SSTMetadata::new(
                        id,
                        bloom.into(),
                        index_block.into(),
                        key_range.first_key,
                        key_range.last_key,
                        OnceLock::new(), // no FD caching here
                        table_file_path,
                        table_footer,
                    );

                    VersionOperation::AddWithMeta { level, meta, index }
                }

                // Keep everything else unchanged
                _ => continue,
            };
            // Replace old op with new one
            *op = new_op;
        }
        Ok(())
    }
    pub fn decode(reader: &mut impl Read, root_dir: &Path) -> Result<Version, SSTableError> {
        // read last commited offset
        let comitted_wal_offset = reader.read_u64::<BigEndian>()?;

        // then number of levels as u16
        let level_count = reader.read_u16::<BigEndian>()?;
        let mut levels = Vec::with_capacity(level_count as usize);
        // then we will encode each level
        let mut id_bytes = [0u8; 16];
        for i in 0..level_count {
            // number of tables in this version
            let i_level_len = reader.read_u64::<BigEndian>()?;
            let mut level = Vec::with_capacity(i_level_len as usize);
            for _table_index in 0..i_level_len {
                // as uuid is 16 bytes long
                reader.read_exact(&mut id_bytes)?;
                let table_id = uuid::Uuid::from_bytes(id_bytes);
                // now we have table id we need to get SSTmeta for this
                let table_file_path = root_dir.join(format!("l{}/{}", i, table_id));
                // read the file to decode bloom,index, footer and (first,last) keys
                let mut table = File::open(&table_file_path)?;
                // 1. first we will read the footer
                table.seek(std::io::SeekFrom::End(-32))?; // as footer size is fixed for 
                let table_footer = SSTableFooter::deserialize(&mut table)?;
                // now we have all offsets we will try to deserialize bloom and index
                // deserilizing bloom
                table.seek(std::io::SeekFrom::Start(table_footer.data_block_size))?;
                // now the file pointer is on the bloom filter start
                let bloom = BloomFactory::deserialize_bloom_filter(&mut table)?;
                // now the pointer will be at index block start
                let index = IndexFactory::deserialize_index(&mut table)?;
                // now we will deserialize the key range
                let key_range = SSTableKeyRange::deserialize(&mut table)?;

                let sst_meta = SSTMetadata::new(
                    table_id,
                    bloom.into(),
                    index.into(),
                    key_range.first_key,
                    key_range.last_key,
                    OnceLock::new(), // we will not be storing the open FD here
                    table_file_path,
                    table_footer,
                );
                level.push(sst_meta);
            }
            levels.push(level);
        }
        Ok(Version::new(levels, comitted_wal_offset))
    }
}

#[cfg(test)]
mod test {

    use std::sync::Arc;

    use crate::database::{
        Entry,
        config::DbConfig,
        memtable::{Memtable, vector_memtable::VectorMemtable},
        sstable::version::{version_manager::VersionManager, version_update::VersionUpdate},
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
        for i in entities.clone().into_iter().enumerate() {
            vm.insert(i.1, i.0 as u64);
        }

        let (config, _dir) = DbConfig::get_test_config();
        let (cleaner_channel_producer, _cleaner_channel_receiver) = std::sync::mpsc::channel();

        let version_manager = VersionManager::new(config, cleaner_channel_producer).unwrap();
        let v1 = version_manager.push_memtable(Arc::new(vm)).unwrap();
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
        for i in entities1.clone().into_iter().enumerate() {
            vm.insert(i.1, i.0 as u64);
        }
        let vm = Arc::new(vm);
        let (config, _dir) = DbConfig::get_test_config();
        let (cleaner_channel_producer, _cleaner_channel_receiver) = std::sync::mpsc::channel();

        let mut version_manager = VersionManager::new(config, cleaner_channel_producer).unwrap();

        let v1 = version_manager.push_memtable(vm.clone()).unwrap();
        let mut vu1 = VersionUpdate::new(0);
        vu1.add_operation(
            crate::database::sstable::version::version_update::VersionOperation::AddWithMeta {
                level: 0,
                meta: v1,
                index: u32::MAX,
            },
        );
        version_manager.push_version_update(vu1).unwrap();
        // version_manager.push_l0_update(v1, vm.get_wal_offset());
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
        for i in entities2.clone().into_iter().enumerate() {
            vm2.insert(i.1, (i.0 + 5) as u64);
        }
        let vm2 = Arc::new(vm2);
        let v2 = version_manager.push_memtable(vm2.clone()).unwrap();
        let mut vu2 = VersionUpdate::new(0);
        vu2.add_operation(
            crate::database::sstable::version::version_update::VersionOperation::AddWithMeta {
                level: 0,
                meta: v2,
                index: u32::MAX,
            },
        );
        version_manager.push_version_update(vu2).unwrap();
        // version_manager.push_l0_update(v2, vm2.get_wal_offset());
        let version = version_manager.get_latest_version();

        assert_eq!(
            version.find(b"id3").unwrap(),
            Some(
                Entry::Row {
                    key: b"id3",
                    value: b"2value3",
                }
                .into(),
            )
        );
        assert_eq!(
            version.find(b"id1").unwrap(),
            Some(
                Entry::Row {
                    key: b"id1",
                    value: b"value1",
                }
                .into(),
            )
        );
        assert_eq!(version.find(b"id34").unwrap(), None);
    }
}
