use std::{
    collections::{HashSet, VecDeque},
    fs::{File, create_dir_all},
    io::Write,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use crate::database::{
    config::CONFIG,
    factory::{bloom::build_bloom_filter, index::build_index},
    memtable::Memtable,
    sstable::{
        errors::SSTableError,
        metadata::{SSTMetadata, SSTableFooter},
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
    pub fn new(root_dir: PathBuf) -> Self {
        create_dir_all(&root_dir).unwrap();
        let version_file = File::options()
            .create(true)
            .append(true)
            .open(root_dir.join("versions.txt"))
            // .open("versions.txt")
            .unwrap();
        let mut versions = VecDeque::new();
        let v0 = Arc::new(Version::new(Vec::default()));
        versions.push_back(v0);
        Self {
            root_dir,
            // we will insert version which doesn't contain any sstable
            versions,
            version_file,
        }
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
        let mut bloom = build_bloom_filter(&CONFIG.bloom);
        let mut index = build_index(&CONFIG.index);
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
        let sst_meta = SSTMetadata::new(
            *mt.get_id(),
            bloom.into(),
            index.into(),
            first_key,
            last_key,
            OnceLock::new(),
            new_table_path,
            // TODO: encode the bytes_encoded and bloom filter to fil
            SSTableFooter::new(bytes_encoded, 0, 0),
        );
        // now we will update
        // we will insert this to the the L0 of the latest version
        Ok(sst_meta)
    }
    pub fn push_l0_update(&mut self, sst_meta: SSTMetadata) {
        let new_version = if self.versions.len() > 0 {
            let latest_version = self.get_latest_version();
            let original_version = latest_version.clone();
            original_version.add_l0_table(sst_meta)
        } else {
            Version::new(vec![vec![sst_meta]])
        };
        self.push_version(new_version);
    }

    pub fn push_version(&mut self, v: Version) {
        writeln!(self.version_file, "{:#?}", v).unwrap();
        self.versions.push_back(Arc::new(v));
    }
}
