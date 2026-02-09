use std::{
    collections::{HashSet, VecDeque},
    fs::{File, create_dir_all},
    io::{Seek, Write},
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::database::{
    config::CONFIG,
    memtable::Memtable,
    sstable::{
        errors::SSTableError,
        metadata::{
            SSTMetadata, SSTableFooter, SSTableKeyRange, bloom_filter::bloom_factory::BloomFactory,
            index::index_factory::IndexFactory,
        },
        version::Version,
    },
};

pub struct VersionManager {
    root_dir: PathBuf,
    versions: VecDeque<Arc<Version>>,
    version_file: File,
}
const INDEX_BLOCK_MIN_BYTES: u64 = 400;

impl VersionManager {
    pub fn new(root_dir: PathBuf) -> Result<Self, SSTableError> {
        create_dir_all(&root_dir).unwrap();
        let mut version_file = File::options()
            .create(true)
            .write(true)
            .read(true)
            .open(root_dir.join("versions.txt"))
            // .open("versions.txt")
            .unwrap();
        let mut versions = VecDeque::new();

        // if the version file is not empty we need to recover
        let v0 = if version_file.metadata()?.len() > 0 {
            // here comes the recovery
            // first we will read the starting offset of the latest version
            version_file.seek(std::io::SeekFrom::End(-8))?;
            let latest_version_starting_offset = version_file.read_u64::<BigEndian>()?;
            // move the file pointer to that version
            version_file.seek(std::io::SeekFrom::Start(latest_version_starting_offset))?;
            // now can the version to encode
            let v0 = Version::decode(&mut version_file, &root_dir)?;
            // move the file pointer to the end
            version_file.seek(std::io::SeekFrom::End(0))?;
            v0
        } else {
            Version::new(Vec::default(), 0)
        };
        versions.push_back(Arc::new(v0));
        Ok(Self {
            root_dir,
            // we will insert version which doesn't contain any sstable
            versions,
            version_file,
        })
    }
    pub fn get_latest_version(&self) -> &Version {
        assert!(self.versions.len() > 0);
        self.versions.back().unwrap()
    }
    // This function will return the sstatble meta which are clear to be droped
    pub fn claim(&mut self) -> Vec<PathBuf> {
        if self.versions.len() < 2 {
            return vec![];
        }
        // we will find first version whose strong count is >1 (meaning currently in use)
        let index = self
            .versions
            .iter()
            .position(|v| Arc::strong_count(v) > 1)
            .unwrap_or_else(|| self.versions.len() - 1);
        let mut sstables_still_active = HashSet::new();
        let version = &self.versions[index];
        let mut level = 0;
        while let Some(tables) = version.get_level_tables(level) {
            for table in tables {
                sstables_still_active.insert(table.id);
            }
            level += 1;
        }
        let mut sstable_included_in_drop = HashSet::new();
        let mut files_to_delete: Vec<PathBuf> = vec![];
        for i in 0..index {
            // SAFETY: As we know this is the sole owner of arc
            let mut version = Arc::try_unwrap(self.versions.pop_front().unwrap()).unwrap();
            let mut level = 0;
            while let Some(tables) = version.get_level_tables_owned(level) {
                for mut table in tables {
                    if !sstables_still_active.contains(&table.id)
                        && !sstable_included_in_drop.contains(&table.id)
                    {
                        sstable_included_in_drop.insert(table.id);
                        files_to_delete.push(std::mem::take(&mut table.file_path));
                    }
                }
                level += 1;
            }
        }
        // else if
        files_to_delete
    }

    /// This Function doesn't change anything it returns the new version which caller need to to add to version manager
    /// Calling push_version
    pub fn push_memtable(&self, mt: &dyn Memtable) -> Result<SSTMetadata, SSTableError> {
        assert!(mt.size() > 0);
        let new_table_id = format!("{}", mt.get_id());
        let l0_dir = self.root_dir.join("l0");
        create_dir_all(&l0_dir)?;
        let new_table_path = l0_dir.join(&new_table_id);
        let mut writer = File::options()
            .append(true)
            .create_new(true)
            .open(&new_table_path)?;
        let mut bloom = BloomFactory::build_bloom_filter(&CONFIG.bloom);
        let mut index = IndexFactory::build_index(&CONFIG.index);
        let mut bytes_encoded = 0;
        let mut byte_encoded_since_last_index = INDEX_BLOCK_MIN_BYTES;
        let mt_iter = mt.iter();
        let first_key = mt_iter
            .get_first_entry()
            .expect("Memtable to Be flushed should contain atleast one entry")
            .get_key()
            .into();
        let last_key = mt_iter.get_last_entry().unwrap().get_key().into();
        for i in mt_iter {
            // check if entry is eligible for entry
            if byte_encoded_since_last_index >= INDEX_BLOCK_MIN_BYTES {
                index.add_entry(i.get_key(), bytes_encoded);
                byte_encoded_since_last_index = 0;
            }
            let bytes_encoded_for_this_entry = i.encode(&mut writer)?;
            byte_encoded_since_last_index += bytes_encoded_for_this_entry;
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
    pub fn push_l0_update(&mut self, sst_meta: SSTMetadata, commited_wal_offset: u64) {
        let new_version = if self.versions.len() > 0 {
            let latest_version = self.get_latest_version();
            let original_version = latest_version.clone();
            original_version.add_l0_table(sst_meta, commited_wal_offset)
        } else {
            Version::new(vec![vec![sst_meta]], commited_wal_offset)
        };
        //TODO: Handle this unwrap
        self.push_version(new_version).unwrap();
    }

    pub fn push_version(&mut self, v: Version) -> Result<(), SSTableError> {
        // writeln!(self.version_file, "{:#?}", v).unwrap();
        // we will fetch teh starting offset of this version encoding so that we can just read the file from end
        let starting_offset = self.version_file.seek(std::io::SeekFrom::End(0))?;
        v.encode(&mut self.version_file)?;
        self.version_file.write_u64::<BigEndian>(starting_offset)?;
        self.version_file.sync_all()?;
        self.versions.push_back(Arc::new(v));
        Ok(())
    }
}
