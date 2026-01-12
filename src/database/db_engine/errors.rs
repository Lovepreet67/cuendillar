use std::convert::Infallible;

use crate::database::{
    memtable::errors::MemtableError, sstable::errors::SSTableError, wal::errors::WALError,
};

#[derive(Debug)]
pub enum EngineError {
    General,
}

impl From<MemtableError> for EngineError {
    fn from(_value: MemtableError) -> Self {
        EngineError::General
    }
}

impl From<WALError> for EngineError {
    fn from(_value: WALError) -> Self {
        EngineError::General
    }
}

impl From<SSTableError> for EngineError {
    fn from(_value: SSTableError) -> Self {
        Self::General
    }
}
impl From<Infallible> for EngineError {
    fn from(_value: Infallible) -> Self {
        Self::General
    }
}

impl From<std::io::Error> for EngineError {
    fn from(_value: std::io::Error) -> Self {
        Self::General
    }
}
