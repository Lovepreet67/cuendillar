use std::{
    cmp::max,
    collections::HashSet,
    fs::{File, create_dir_all},
    io::Write,
    mem::take,
    sync::{Arc, OnceLock, RwLock},
    vec,
};

use byteorder::{BigEndian, WriteBytesExt};
use tracing::{info, instrument};

use crate::database::{
    Entry,
    OwnedEntry,
    config::{
        bloom_config::BloomConfig, compaction_config::CompactionConfig, index_config::IndexConfig,
    },
    // db_engine::instrumented::InstrumentedRwLock as RwLock,
    sstable::{
        compaction::Compaction,
        errors::SSTableError,
        metadata::{
            SSTMetadata, SSTableFooter, SSTableKeyRange, bloom_filter::bloom_factory::BloomFactory,
            index::index_factory::IndexFactory,
        },
        version::{
            Version,
            version_manager::VersionManager,
            version_update::{
                VersionOperation::{AddWithMeta, Del},
                VersionUpdate,
            },
        },
    },
};

pub struct LevelCompaction {
    version_manager: Arc<RwLock<VersionManager>>,
    config: CompactionConfig,
    bloom_config: BloomConfig,
    index_config: IndexConfig,
}

impl LevelCompaction {
    pub fn new(
        version_manager: Arc<RwLock<VersionManager>>,
        config: &CompactionConfig,
        bloom_config: &BloomConfig,
        index_config: &IndexConfig,
    ) -> Self {
        Self {
            version_manager,
            config: config.clone(),
            bloom_config: bloom_config.clone(),
            index_config: index_config.clone(),
        }
    }
    #[instrument(name = "Leveled Compaction Encode Table", skip(self),fields(entries_len=enteries.len()))]
    fn encode_table(
        &self,
        level: u16,
        table_id: uuid::Uuid,
        enteries: &[OwnedEntry],
    ) -> Result<SSTMetadata, SSTableError> {
        assert!(enteries.len() > 0);
        let level_path = self.config.root_dir.join(format!("l{}", level));
        create_dir_all(&level_path)?;
        let new_table_path = level_path.join(table_id.to_string());
        let mut writer = Vec::new();

        let mut bloom = BloomFactory::build_bloom_filter(&self.bloom_config, enteries.len() as u64);
        let mut index = IndexFactory::build_index(&self.index_config);

        let mut bytes_encoded = 0;
        let mut byte_encoded_since_last_index = self.index_config.index_block_min_size as u64;
        let first_key = enteries[0].get_key().into();
        let last_key = enteries[enteries.len() - 1].get_key().into();
        for i in enteries {
            let i = Entry::from(i);
            // check if entry is eligible for entry
            if byte_encoded_since_last_index >= self.index_config.index_block_min_size as u64 {
                index.add_entry(i.get_key(), bytes_encoded);
                byte_encoded_since_last_index = 0;
            }
            let bytes_encoded_for_this_entry = i.encode(&mut writer)?;
            byte_encoded_since_last_index += bytes_encoded_for_this_entry;
            bytes_encoded += bytes_encoded_for_this_entry;
            bloom.add(i.get_key());
        }
        index.add_last_offset(bytes_encoded);

        // now we will serialize the bloom filter
        // first we will write the name of bloom filter for deserilization
        let mut bloom_filter_size = 0;
        let bloom_name = bloom.get_name().as_bytes();
        writer.write_u16::<BigEndian>(bloom_name.len() as u16)?;
        bloom_filter_size += 2;
        writer.write_all(bloom_name)?;
        bloom_filter_size += bloom_name.len() as u64;
        bloom_filter_size += bloom.serialize(&mut writer)?;

        // now we will serialize the index
        // first we will write the name of index for deserilization
        let mut index_size = 0;
        let index_name = index.get_name().as_bytes();
        writer.write_u16::<BigEndian>(index_name.len() as u16)?;
        index_size += 2;
        writer.write_all(index_name)?;
        index_size += index_name.len() as u64;
        index_size += index.serialize(&mut writer)?;
        // now we will write the keyrange block
        let key_range = SSTableKeyRange {
            first_key,
            last_key,
        };
        let key_range_block_size = key_range.serialize(&mut writer)?;
        // now we will create a serialize footer
        let footer = SSTableFooter::new(
            bytes_encoded,
            bloom_filter_size,
            index_size,
            key_range_block_size,
        );
        footer.seriealize(&mut writer)?;
        File::options()
            .append(true)
            .create_new(true)
            .open(&new_table_path)?
            .write_all(&writer)?;
        Ok(SSTMetadata::new(
            table_id,
            bloom.into(),
            index.into(),
            key_range.first_key,
            key_range.last_key,
            OnceLock::new(),
            new_table_path,
            footer,
        ))
    }

    // list1 will get more preority
    #[instrument(name = "Leveled Compaction Merge", ,fields(list1_size=list1.len(),list2_size=list2.len()))]
    fn merge(list1: Vec<OwnedEntry>, list2: Vec<OwnedEntry>) -> Vec<OwnedEntry> {
        let mut updated_enteries = Vec::with_capacity(list1.len() + list2.len());
        let (mut iter1, mut iter2) = (list1.into_iter(), list2.into_iter());
        let (mut e1, mut e2) = (iter1.next(), iter2.next());
        while let (Some(v1), Some(v2)) = (&e1, &e2) {
            if v1.get_key() == v2.get_key() {
                // we will give list1 priority
                updated_enteries.push(e1.take().unwrap());
                e1 = iter1.next();
                e2 = iter2.next();
            } else if v1.get_key() < v2.get_key() {
                updated_enteries.push(e1.take().unwrap());
                e1 = iter1.next();
            } else {
                updated_enteries.push(e2.take().unwrap());
                e2 = iter2.next();
            }
        }
        while let Some(v1) = e1 {
            updated_enteries.push(v1);
            e1 = iter1.next();
        }
        while let Some(v2) = e2 {
            updated_enteries.push(v2);
            e2 = iter2.next();
        }
        updated_enteries
    }
    #[instrument(name = "Leveled Compaction Merge",skip(self,version,compaction_update,enteries) ,fields(existing_entries_size=enteries.len()))]
    fn compact_ln(
        &self,
        level: u16,
        mut enteries: Vec<OwnedEntry>,
        version: &Version,
        compaction_update: &mut VersionUpdate,
    ) -> Result<(), SSTableError> {
        let ln_tables = version
            .get_level_tables(level as usize)
            .map(|v| v.as_slice())
            .unwrap_or_else(|| &[]);
        info!("Total tables in level {} is {}", level, ln_tables.len());
        // we will find the table in which we entries are overlaping
        // TODO: Avoid this allocation
        let mut first_key = enteries[0].get_key().into();
        // Assumtion: enteries contain always items >1
        let mut last_key = enteries[enteries.len() - 1].get_key().into();

        let mut curr_index = 0;

        for table in ln_tables {
            // we will check using first and last key
            if (table.key_range.first_key <= first_key && table.key_range.last_key >= first_key)
                || (table.key_range.first_key <= last_key && table.key_range.last_key >= last_key)
                || (first_key <= table.key_range.first_key && last_key >= table.key_range.first_key)
                || (first_key <= table.key_range.last_key && last_key >= table.key_range.last_key)
            {
                compaction_update.add_operation(Del {
                    level: level as u32,
                    id: table.id,
                });
                // we need to merge this into enteries
                // we have two list both sorted and
                enteries = Self::merge(enteries, table.item_list()?);
                // first and last key will change after merging the enteries;
                first_key = enteries[0].get_key().into();
                // Assumtion: enteries contain always items >1
                last_key = enteries[enteries.len() - 1].get_key().into();
            }
            // else if the last key of enteris is smaller than current table it means the entries sstable should be in front of current
            else if enteries.len() > 0 && table.key_range.first_key > last_key {
                let original_enteries = if level < self.config.max_level_count as u16 {
                    take(&mut enteries)
                } else {
                    // remove tombstones in max level
                    take(&mut enteries)
                        .into_iter()
                        .filter(|entry| {
                            return match entry {
                                OwnedEntry::Tombstone { seq_no: _, key: _ } => false,
                                _ => true,
                            };
                        })
                        .collect()
                };
                // currently we are merging all enteries to single sstable but it should be multiple tables
                let max_enteries_per_sstable = (self.config.level_entries_growth_factor)
                    .pow(level.into())
                    * self.config.base_entries_per_table;
                // we will split the whole enteries into the block the size calculated
                for entry_group in original_enteries.chunks(max_enteries_per_sstable as usize) {
                    let sstable_meta = self
                        .encode_table(level, uuid::Uuid::new_v4(), entry_group)
                        .expect("Error while creating new sstable");
                    compaction_update.add_operation(AddWithMeta {
                        level: level as u32,
                        meta: sstable_meta,
                        index: curr_index,
                    });
                    curr_index += 1;
                }
                curr_index += 1; // this is for the current table (old one in the level) as after inserting the entries we will inser the curr table
            }
        }
        // entereis may be added at the end
        if enteries.len() > 0 {
            let max_enteries_per_sstable = (self.config.level_entries_growth_factor)
                .pow(level.into())
                * self.config.base_entries_per_table;
            // we will split the whole enteries into the block the size calculated
            for entry_group in enteries.chunks(max_enteries_per_sstable as usize) {
                let sstable_meta = self
                    .encode_table(level, uuid::Uuid::new_v4(), entry_group)
                    .expect("Error while creating new sstable");
                compaction_update.add_operation(AddWithMeta {
                    level: level as u32,
                    meta: sstable_meta,
                    index: curr_index,
                });
                curr_index += 1;
            }
        }
        Ok(())
    }
}

impl Compaction for LevelCompaction {
    #[instrument(name = "Leveled Compaction Need Compaction", skip(self))]
    fn need_compaction(&self) -> bool {
        let version_manager = self
            .version_manager
            .read(
                // "Checking if compaction is needed"
            )
            .unwrap();
        let version = version_manager.get_latest_version();
        match version.get_level_tables(0) {
            Some(tables) if tables.len() < self.config.min_l0_file_count as usize => false,
            None => false,
            _ => true,
        }
    }
    #[instrument(name = "Leveled Compaction Run Compaction", skip(self))]
    fn run_compaction(&self) -> Result<(), SSTableError> {
        info!("Leveled compaction started");
        // we will check for the l0 have table  greater than min files trigger
        let version_manager = self
            .version_manager
            .read(
                // "reading latest version at commpaction start"
            )
            .unwrap();
        let version = version_manager.get_latest_version();
        drop(version_manager);
        let mut version = (*version).clone();
        let mut compaction_update = VersionUpdate::new(0);
        // now we have version we will do compaction int he l0
        // now we get multiple iterator and we will merge them into single table
        let mut key_seen: HashSet<Vec<u8>> = HashSet::new();
        let mut merged_enteries = version
            .get_level_tables(0)
            .unwrap()
            .iter()
            .take(self.config.max_l0_file_count_per_cycle)
            .rev()
            .map(|table| {
                compaction_update.add_operation(Del {
                    level: 0,
                    id: table.id,
                });
                let mut filterd_entries = vec![];
                for item in table
                    .item_list()
                    .expect("Error while getting item list for table")
                {
                    if key_seen.insert(item.get_key().into()) {
                        filterd_entries.push(item);
                    }
                }
                return filterd_entries;
            })
            .flatten()
            .collect::<Vec<OwnedEntry>>();
        merged_enteries.sort_by(|a, b| a.get_key().cmp(&b.get_key()));
        info!(
            "Merged all l0 tables in total entries : {}",
            merged_enteries.len()
        );
        // no we have a single iterator over all the table in l0
        // now we will set merge this table to the l1 and so on
        for i in 1..self.config.max_level_count {
            info!("Compacting level {}", i);
            let mut local_compaction_update = VersionUpdate::new(0);
            self.compact_ln(
                i as u16,
                merged_enteries,
                &version,
                &mut local_compaction_update,
            )?;
            // at this point we will have the version update which will contain all the neccessary updates
            compaction_update
                .operations
                .append(&mut local_compaction_update.operations.clone());
            info!(
                "After compaction version update count become : {}",
                compaction_update.operations.len()
            );
            version.apply_update(local_compaction_update)?;

            // next level we will choose last table only (for now as it is easy to pop)
            let mut li_total_size = 0;
            let mut new_li_meta = version
                .get_level_tables(i)
                .map(|x| x.iter().collect())
                .unwrap_or_else(|| Vec::new());
            for i in &new_li_meta {
                li_total_size += i.get_size();
            }
            // we will check if the level size is exceeding the threshold (to trigger compaction again)
            if li_total_size
                < (self.config.level_size_growth_factor as u64).pow(i as u32)
                    * self.config.level_base_size as u64
                || i == self.config.max_level_count
            {
                // in this branch we will handle when there is no compaction needed
                break;
            }
            // we will pop the last 2 enteries if there are and use them as a new version list
            // and skip the last level as  we can't compact it to next level
            if i < self.config.max_level_count - 1 {
                let poped_table = new_li_meta.pop().unwrap();
                compaction_update.add_operation(Del {
                    level: i as u32,
                    id: poped_table.id,
                });
                merged_enteries = poped_table.item_list()?;
                // we try to move half data we added to the next level
                let mut tables_to_be_poped = new_li_meta.len() / 2;
                while !new_li_meta.is_empty() && tables_to_be_poped > 0 {
                    let poped_table = new_li_meta.pop().unwrap();
                    compaction_update.add_operation(Del {
                        level: i as u32,
                        id: poped_table.id,
                    });
                    let mut updated_vec = poped_table.item_list()?;
                    updated_vec.extend(merged_enteries);
                    merged_enteries = updated_vec;
                    tables_to_be_poped -= 1;
                }
            } else {
                // nothing to worry since current iteration is last iteration
                merged_enteries = vec![];
            }
        }
        // following the above code the version will contain all the neccessary updates
        Version::normalize_version_update_operation(&mut compaction_update, &self.config.root_dir)?;
        self.version_manager
            .write(
                // "Writing compaction update to the "
            )
            .unwrap()
            .push_version_update(compaction_update)?;
        info!("Compaction completed successfully");
        Ok(())
    }
}
