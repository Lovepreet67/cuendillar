use std::path::PathBuf;

use crate::database::wal::{errors::WALWriteError, wal_entry::WALEntry};

pub mod default_wal_writer;
pub trait WALWriter: 'static + Sized {
    fn new(root_dir: PathBuf) -> Result<Self, std::io::Error>;
    fn rotate(&mut self, id: Option<uuid::Uuid>) -> Result<(), std::io::Error>;
    fn append(&mut self, entry: WALEntry) -> Result<(), WALWriteError>;
}
