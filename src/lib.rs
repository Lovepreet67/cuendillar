use std::sync::{Arc, RwLock};

pub use crate::database::OwnedEntry;
pub use crate::database::config::{
    self, DbConfig, bloom_config, cleaner_config, compaction_config, config_error, index_config,
    memtable_config, version_manager_config, wal_config,
};
use crate::database::{
    Entry,
    db_engine::{Engine, errors::EngineError},
};

mod database;
/// Public database handle.
///
/// `Database` is the main entry point for interacting with the storage engine.
/// It acts as a lightweight wrapper around the internal `Engine`.
///
/// The engine is wrapped in `Arc<RwLock<...>>` so the database handle can
/// be cloned and safely shared across multiple threads.
///
/// Although the current implementation may run in a single-threaded context,
/// this design allows the internal engine to evolve into a fully concurrent
/// implementation without requiring changes to the public API.
#[derive(Clone)]
pub struct Database {
    /// Shared reference to the underlying database engine.
    ///
    /// `Arc` allows cheap cloning of the database handle.
    /// `RwLock` allows multiple readers or a single writer.
    db: Arc<RwLock<Engine>>,
}
impl Database {
    /// Creates a new database instance.
    ///
    /// This initializes the internal storage engine using the provided
    /// configuration. During initialization the engine may:
    ///
    /// - Open or create storage files
    /// - Recover state from the Write-Ahead Log (WAL)
    /// - Rebuild memtables
    ///
    /// # Arguments
    ///
    /// * `config` - Shared database configuration.
    ///
    /// # Errors
    ///
    /// Returns `EngineError` if initialization fails (e.g. WAL corruption,
    /// file system errors, or recovery failures).
    pub fn new(config: Arc<DbConfig>) -> Result<Self, EngineError> {
        let engine = Engine::new(config)?;
        Ok(Database {
            db: Arc::new(RwLock::new(engine)),
        })
    }
    /// Retrieves the value associated with the given key.
    ///
    /// This method performs a read operation on the database.
    /// Internally it acquires a shared read lock on the engine,
    /// allowing multiple concurrent readers.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(OwnedEntry))` if the key exists
    /// * `Ok(None)` if the key does not exist
    ///
    /// # Errors
    ///
    /// Returns `EngineError` if an internal error occurs.
    pub fn get(&self, key: &[u8]) -> Result<Option<database::OwnedEntry>, EngineError> {
        self.db.read()?.find(key)
    }
    /// Inserts or updates a key-value pair in the database.
    ///
    /// The write is first recorded in the Write-Ahead Log (WAL)
    /// and then applied to the memtable.
    ///
    /// # Arguments
    ///
    /// * `key` - Key to insert or update
    /// * `value` - Value associated with the key
    ///
    /// # Tombstone Semantics
    ///
    /// An **empty value (`value.len() == 0`) is treated as a tombstone**, meaning
    /// the key is logically deleted. Tombstones are later cleaned up during
    /// compaction.
    ///
    /// # Errors
    ///
    /// Returns `EngineError` if the write fails due to WAL or storage errors.
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), EngineError> {
        let e = Entry::Row { key, value };
        self.db.write()?.write(e)
    }
    /// Deletes a key from the database.
    ///
    /// This operation does not immediately remove the key from storage.
    /// Instead, a **tombstone entry** is written.
    ///
    /// Tombstones mark a key as deleted and are later removed during
    /// compaction when older versions of the key are cleaned up.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete
    ///
    /// # Errors
    ///
    /// Returns `EngineError` if the delete operation fails.
    pub fn delete(&self, key: &[u8]) -> Result<(), EngineError> {
        let e = Entry::Tombstone { key };
        self.db.write()?.write(e)
    }
}
