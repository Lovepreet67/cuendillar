use std::{convert::Infallible, sync::PoisonError};

use crate::database::{
    memtable::errors::MemtableError, sstable::errors::SSTableError, wal::errors::WALError,
};

#[derive(Debug)]
pub enum EngineError {
    General,
    PosionError,
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

impl<T> From<PoisonError<T>> for EngineError {
    fn from(_value: PoisonError<T>) -> Self {
        Self::PosionError
    }
}
