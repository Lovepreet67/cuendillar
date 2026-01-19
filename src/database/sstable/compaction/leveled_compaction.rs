use std::{
    collections::HashSet,
    fs::{File, create_dir_all},
    mem::{replace, take},
    path::PathBuf,
    sync::{Arc, OnceLock, RwLock},
    thread::{self, sleep},
    time::Duration,
    vec,
};

use crate::database::{
    OwnedEntry,
    sstable::{
        errors::SSTableError,
        metadata::{
            SSTMetadata, SSTableFooter,
            bloom_filter::{BloomFilter, default_bloom_filter::DefaultBloomFilter},
            index::{SSTIndex, default_index::DefaultIndex},
        },
        version::Version,
        version_manager::VersionManager,
    },
};
const INDEX_BLOCK_MIN_BYTES: u64 = 400;

pub struct LevelCompaction {
    version_manager: Arc<RwLock<VersionManager>>,
    root_dir: PathBuf,
    min_l0_file_count: u16,
    max_levels: u16,
}

impl LevelCompaction {
    pub fn new(
        version_manager: Arc<RwLock<VersionManager>>,
        min_l0_file_count: u16,
        root_dir: PathBuf,
        max_levels: u16,
    ) -> Self {
        Self {
            version_manager,
            min_l0_file_count,
            root_dir,
            max_levels,
        }
    }
    fn encode_table(
        &self,
        level: u16,
        table_id: uuid::Uuid,
        enteries: Vec<OwnedEntry>,
    ) -> Result<SSTMetadata, SSTableError> {
        assert!(enteries.len() > 0);
        let new_table_id = format!("{}", table_id);
        let level_path = self.root_dir.join("{}", level);
        create_dir_all(&level_path);
        let new_table_path = level_path.join("{}", table_id);
        let mut writer = File::options()
            .append(true)
            .create_new(true)
            .open(&new_table_path)?;

        let mut bloom = DefaultBloomFilter::new(10000, 100);
        let mut index = DefaultIndex::new();
        let mut bytes_encoded = 0;
        let mut byte_encoded_since_last_index = INDEX_BLOCK_MIN_BYTES;
        let first_key = enteries[0].get_id().into();
        let last_key = enteries[enteries.len() - 1].get_id().into();
        for i in enteries {
            let i = i.into();
            // check if entry is eligible for entry
            if byte_encoded_since_last_index >= INDEX_BLOCK_MIN_BYTES {
                index.add_entry(i.into().get_key(), bytes_encoded);
                byte_encoded_since_last_index = 0;
            }
            let bytes_encoded_for_this_entry = i.encode(&mut writer)?;
            byte_encoded_since_last_index += bytes_encoded_for_this_entry;
            bytes_encoded += bytes_encoded_for_this_entry;
            bloom.add(i.get_key());
        }
        index.add_last_offset(bytes_encoded);
        Ok(SSTMetadata::new(
            table_id,
            bloom,
            index,
            first_key,
            last_key,
            OnceLock::new(),
            new_table_path,
            // TODO: encode the bytes_encoded and bloom filter to fil
            SSTableFooter::new(bytes_encoded, 0, 0),
        ))
    }

    // list1 will get more preority
    fn merge(list1: Vec<OwnedEntry>, list2: Vec<OwnedEntry>) -> Vec<OwnedEntry> {
        let updated_enteries = vec![];
        let (l1, l2, ln1, ln2) = (0, 0, list1.len(), list2.len());
        while (l1 < ln1 && l2 < ln2) {}
        while (l1 < ln1) {}
        while (l2 < ln2) {}
    }
    fn compact_ln(
        &self,
        level: u16,
        mut enteries: Vec<OwnedEntry>,
        version: &Version,
    ) -> Vec<SSTMetadata> {
        let ln_tables = version
            .get_level_tables(level as usize)
            .map(|v| v.as_slice())
            .unwrap_or_else(|| &[]);
        // we will find the table in which we entries are overlaping
        let mut updated_sst_list = vec![];
        // TODO: Avoid this allocation
        let first_key = enteries[0].get_id().into();
        // Assumtion: enteries contain always items >1
        let last_key = enteries[enteries.len() - 1].get_id().into();
        for table in ln_tables {
            // we will check using first and last key
            if (table.first_key <= first_key && table.last_key >= first_key)
                || (table.first_key <= last_key && table.last_key >= last_key)
            {
                // we need to merge this into enteries
                // we have two list both sorted and
            }
            // else if the last key of enteris is smaller than current table it means the entries sstable should be in front of current
            else if enteries.len() > 0 && table.first_key >= last_key {
                let original_enteries = take(&mut enteries);
                let sstable_meta = self
                    .encode_table(level, uuid::Uuid::new_v4(), original_enteries)
                    .expect("Error while creating new sstable");
                updated_sst_list.push(sstable_meta);
                updated_sst_list.push(table.clone());
            } else {
                // we will use that sstable as it is
                updated_sst_list.push(table.clone());
            }
        }
        // entereis may be added at the end
        if enteries.len() > 0 {
            let sstable_meta = self
                .encode_table(level, uuid::Uuid::new_v4(), enteries)
                .expect("Error while creating new sstable");
            updated_sst_list.push(sstable_meta);
        }
        updated_sst_list
    }
    pub fn init(self) {
        thread::spawn(move || {
            loop {
                // we will check for the l0 have table  greater than min files trigger
                let version_manager = self.version_manager.read().unwrap();
                let version = version_manager.get_latest_version();
                let need_compaction = match version.get_level_tables(0) {
                    Some(tables) if tables.len() < self.min_l0_file_count as usize => false,
                    None => false,
                    _ => true,
                };
                if !need_compaction {
                    drop(version_manager);
                    sleep(Duration::from_secs(10));
                    continue;
                }
                let version = version.clone();
                drop(version_manager);
                // now we have version we will do compaction int he l0
                // now we get multiple iterator and we will merge them into single table
                let mut key_seen: HashSet<Vec<u8>> = HashSet::new();
                let mut merged_enteries = version
                    .get_level_tables(0)
                    .unwrap()
                    .into_iter()
                    .map(|table| {
                        let mut filterd_entries = vec![];
                        for item in table
                            .item_list()
                            .expect("Error while getting item list for table")
                        {
                            if key_seen.insert(item.get_id().into()) {
                                filterd_entries.push(item);
                            }
                        }
                        return filterd_entries;
                    })
                    .flatten()
                    .collect::<Vec<OwnedEntry>>();
                merged_enteries.sort_by(|a, b| a.get_id().cmp(&b.get_id()));
                // no we have a single iterator over all the table in l0
                // now we will set merge this table to the l1
                let l1_tables = version.get_level_tables(0);
                if l1_tables.is_none() {
                    // no need to merge we will directly push the table to the l1
                    let new_sstable = self
                        .encode_table(1, uuid::Uuid::new_v4(), merged_enteries)
                        .expect("error while creating a new table ");
                    let new_version = Version::new(vec![vec![new_sstable]]);
                }
            }
        });
    }
}
