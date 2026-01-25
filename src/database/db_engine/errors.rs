use std::{convert::Infallible, sync::PoisonError};

use crate::database::{
    memtable::errors::MemtableError, sstable::errors::SSTableError, wal::errors::WALError,
};

#[derive(Debug)]
pub enum EngineError {
    General,
    Internal(String),
    PosionError,
    IoError(std::io::Error),
}

impl From<MemtableError> for EngineError {
    fn from(value: MemtableError) -> Self {
        let message = match value {
            MemtableError::NotFound => "Not Found in memtable",
            MemtableError::Deleted => "value is deleted from memtable",
            MemtableError::NoImmutableMemtableExist => "Immutable memtable doesn't exist",
            MemtableError::InvalidCandidateId => "Invalid Candidate Id in Memtable",
        };
        EngineError::Internal(message.to_owned())
    }
}

impl From<WALError> for EngineError {
    fn from(value: WALError) -> Self {
        match value {
            WALError::IOError(e) => return EngineError::IoError(e),
            WALError::CrrouptedMetadataFile(f) => {
                return EngineError::Internal(format!("Crrouped metadata file for wal, {}", f));
            }
        };
    }
}

impl From<SSTableError> for EngineError {
    fn from(value: SSTableError) -> Self {
        match value {
            SSTableError::IoError(e) => return EngineError::IoError(e),
            SSTableError::UuidError(e) => {
                return EngineError::Internal("Got UUid errir in SSTable".into());
            }
            SSTableError::PoisonedError => {
                return EngineError::Internal("trying to access poisened SStable".into());
            }
        }
    }
}
impl From<Infallible> for EngineError {
    fn from(value: Infallible) -> Self {
        Self::Internal(format!("Infallible error {:?}", value))
    }
}

impl From<std::io::Error> for EngineError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl<T> From<PoisonError<T>> for EngineError {
    fn from(_value: PoisonError<T>) -> Self {
        Self::PosionError
    }
}
