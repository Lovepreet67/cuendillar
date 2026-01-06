use crate::database::{memtable::errors::MemtableError, wal::errors::WALError};

#[derive(Debug)]
pub enum EngineError {
    General,
}

impl From<MemtableError> for EngineError {
    fn from(value: MemtableError) -> Self {
        EngineError::General
    }
}

impl From<WALError> for EngineError {
    fn from(value: WALError) -> Self {
        EngineError::General
    }
}
