use std::path::PathBuf;

use crate::database::{
    OwnedEntry,
    wal::{errors::WALError, wal_entry::WALEntry},
};

pub mod default_wal;
pub mod errors;
#[cfg(test)]
mod tests;
pub mod wal_entry;

pub trait WAL: Sized {
    fn new(root_dir: PathBuf) -> Result<Self, WALError>;
    fn rotate(&mut self, id: Option<uuid::Uuid>) -> Result<(), WALError>;
    fn append_log(&mut self, entry: WALEntry) -> Result<(), WALError>;
    fn read(&mut self, log_id: &uuid::Uuid) -> Result<Vec<OwnedEntry>, WALError>;
    fn get_wals(&mut self) -> Result<Vec<uuid::Uuid>, WALError>;
    fn flush_wal(&mut self, id: uuid::Uuid) -> Result<(), WALError>;
}
