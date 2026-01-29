use crate::database::sstable::errors::SSTableError;

pub mod leveled_compaction;

pub trait Compaction: Send + Sync {
    fn need_compaction(&self) -> bool;
    fn run_compaction(&self) -> Result<(), SSTableError>;
}
