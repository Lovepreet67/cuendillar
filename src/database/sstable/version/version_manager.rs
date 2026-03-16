use std::{
    fs::{File, create_dir_all},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use byteorder::{BigEndian, WriteBytesExt};
use tracing::{info, instrument};

use crate::database::{
    config::{DbConfig, bloom_config::BloomConfig, index_config::IndexConfig},
    factory::wal::build_wal_manger,
    iterator::merged_iterator::MergedIterator,
    memtable::Memtable,
    sstable::{
        errors::SSTableError,
        metadata::{
            SSTMetadata, SSTableFooter, SSTableKeyRange, bloom_filter::bloom_factory::BloomFactory,
            index::index_factory::IndexFactory,
        },
        version::{
            Version,
            version_update::{VersionOperation, VersionUpdate},
        },
    },
    wal::WAL,
};

pub struct VersionManager {
    root_dir: PathBuf,
    version: Arc<Version>,
    version_wal: Box<dyn WAL>,
    config: Arc<DbConfig>,
    cleaner_channel_producer: std::sync::mpsc::Sender<(Arc<Version>, Vec<PathBuf>)>,
}

impl VersionManager {
    #[instrument(name = "Verion Manger New", skip(config, cleaner_channel_producer))]
    pub fn new(
        config: Arc<DbConfig>,
        cleaner_channel_producer: std::sync::mpsc::Sender<(Arc<Version>, Vec<PathBuf>)>,
    ) -> Result<Self, SSTableError> {
        info!("Creating new Version Manager");
        let sstable_root_dir = config.sstable_root_dir.clone();
        let mut wal_config = config.wal.clone();
        wal_config.wal_sync_variant = config
            .version_manager
            .version_manager_sync_mode
            .clone()
            .into();
        wal_config.wal_dir = sstable_root_dir.join("version");
        create_dir_all(&sstable_root_dir).unwrap();
        let version_wal = build_wal_manger(&wal_config)?;
        let mut version = Version::new(Vec::default(), 0);
        // now we will replay the wal // we will start from 0 as we will never be flushing the version wal
        let version_iterator = version_wal.read(0)?;
        let mut version_update_operations = vec![];
        let mut last_commited_offset = 0;
        info!("Reading the Version WAL");
        for version_update_encoded in version_iterator {
            if let Ok((_, update_bytes)) = version_update_encoded {
                let mut version_update = VersionUpdate::decode(&mut update_bytes.as_slice())?;
                last_commited_offset =
                    std::cmp::max(version_update.wal_offset, last_commited_offset);
                version_update_operations.append(&mut version_update.operations);
            } else {
                return Err(SSTableError::General(
                    "Error happend while reading the version update manifest".into(),
                ));
            }
        }
        let mut version_update = VersionUpdate::new(last_commited_offset);
        version_update.operations = version_update_operations;
        info!(
            "Read Version WAL found {} total version operations, and last committed offset {}",
            version_update.operations.len(),
            last_commited_offset
        );

        // then we will normalize this version
        Version::normalize_version_update_operation(&mut version_update, &sstable_root_dir)?;
        version.apply_update(version_update)?;

        Ok(Self {
            root_dir: sstable_root_dir,
            // we will insert version which doesn't contain any sstable
            version: Arc::new(version),
            version_wal,
            config,
            cleaner_channel_producer,
        })
    }
    pub fn get_latest_version(&self) -> Arc<Version> {
        // let mut file = File::options()
        //     .create(true)
        //     .append(true)
        //     .open("version_log.txt")
        //     .unwrap();
        // writeln!(file, " version {:?}", self.version);
        self.version.clone()
    }
    // This function will return the sstatble meta which are clear to be droped
    /// This Function doesn't change anything it returns the new version which caller need to to add to version manager
    /// Calling push_version
    pub fn push_memtable(&self, mt: Arc<dyn Memtable>) -> Result<SSTMetadata, SSTableError> {
        assert!(mt.size() > 0);
        let new_table_id = format!("{}", mt.get_id());
        let l0_dir = self.root_dir.join("l0");
        create_dir_all(&l0_dir)?;
        let new_table_path = l0_dir.join(&new_table_id);
        let mut writer = Vec::new();

        let mut bloom = BloomFactory::build_bloom_filter(&self.config.bloom, mt.num_enteries());
        let mut index = IndexFactory::build_index(&self.config.index);
        let mut bytes_encoded = 0;
        let mut byte_encoded_since_last_index = self.config.index.index_block_min_size;
        let mt_iter = mt.iter();
        let first_key = mt_iter
            .first_entry()
            .expect("Memtable to Be flushed should contain atleast one entry")
            .get_key()
            .into();
        let last_key = mt_iter.last_entry().unwrap().get_key().into();
        for i in mt_iter {
            // check if entry is eligible for entry
            if byte_encoded_since_last_index >= self.config.index.index_block_min_size {
                index.add_entry(i.get_key(), bytes_encoded);
                byte_encoded_since_last_index = 0;
            }
            let bytes_encoded_for_this_entry = i.encode(&mut writer)?;
            byte_encoded_since_last_index += bytes_encoded_for_this_entry as usize;
            bytes_encoded += bytes_encoded_for_this_entry;
            bloom.add(i.get_key());
        }
        index.add_last_offset(bytes_encoded);
        // now we have written all the entries to file

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

        // now we will add the create and add the SSTableKeyRange
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
        let sst_meta = SSTMetadata::new(
            *mt.get_id(),
            bloom.into(),
            index.into(),
            key_range.first_key,
            key_range.last_key,
            OnceLock::new(),
            new_table_path,
            footer,
        );
        // now we will update
        // we will insert this to the the L0 of the latest version
        Ok(sst_meta)
    }

    pub fn push_memtable_static(
        sstable_root_dir: &Path,
        bloom_config: &BloomConfig,
        index_config: &IndexConfig,
        mt: Arc<dyn Memtable>,
    ) -> Result<SSTMetadata, SSTableError> {
        assert!(mt.size() > 0);
        let new_table_id = format!("{}", mt.get_id());
        let l0_dir = sstable_root_dir.join("l0");
        create_dir_all(&l0_dir)?;
        let new_table_path = l0_dir.join(&new_table_id);
        let mut writer = Vec::new();
        let mut bloom = BloomFactory::build_bloom_filter(bloom_config, mt.num_enteries());
        let mut index = IndexFactory::build_index(index_config);
        let mut bytes_encoded = 0;
        let mut byte_encoded_since_last_index = index_config.index_block_min_size;
        let mt_iter = mt.iter();
        let first_key = mt_iter
            .first_entry()
            .expect("Memtable to Be flushed should contain atleast one entry")
            .get_key()
            .into();
        let last_key = mt_iter.last_entry().unwrap().get_key().into();
        for i in mt_iter {
            // check if entry is eligible for entry
            if byte_encoded_since_last_index >= index_config.index_block_min_size {
                index.add_entry(i.get_key(), bytes_encoded);
                byte_encoded_since_last_index = 0;
            }
            let bytes_encoded_for_this_entry = i.encode(&mut writer)?;
            byte_encoded_since_last_index += bytes_encoded_for_this_entry as usize;
            bytes_encoded += bytes_encoded_for_this_entry;
            bloom.add(i.get_key());
        }
        index.add_last_offset(bytes_encoded);
        // now we have written all the entries to file

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

        // now we will add the create and add the SSTableKeyRange
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
        let sst_meta = SSTMetadata::new(
            *mt.get_id(),
            bloom.into(),
            index.into(),
            key_range.first_key,
            key_range.last_key,
            OnceLock::new(),
            new_table_path,
            footer,
        );
        // now we will update
        // we will insert this to the the L0 of the latest version
        Ok(sst_meta)
    }

    pub fn push_version_update(&mut self, update: VersionUpdate) -> Result<(), SSTableError> {
        let mut buff = vec![];
        update.encode(&mut buff)?;
        self.version_wal.append_log(&buff)?;
        let mut updated_version = (*self.version).clone();
        let deleted_files: Vec<_> = update
            .operations
            .iter()
            .filter(|op| {
                matches!(
                    op,
                    VersionOperation::Del {
                        level: _level,
                        id: _id
                    }
                )
            })
            .map(|op| {
                match op {
                    VersionOperation::Del { level, id } => self
                        .root_dir
                        .join(format!("l{}", level))
                        .join(id.to_string()),
                    _ => {
                        // this will not happen
                        panic!("Filter is not working broken")
                    }
                }
            })
            .collect();

        updated_version.apply_update(update)?;

        let old_version = std::mem::replace(&mut self.version, Arc::new(updated_version));
        // then we will push the delete files to cleaner channel
        // TODO: handle this result
        // if deleted_files.len() > 0 {
        let _ = self
            .cleaner_channel_producer
            .send((old_version, deleted_files));
        // }
        Ok(())
    }
    pub fn iter(&self) -> Result<MergedIterator, SSTableError> {
        self.version.iter()
    }
}
