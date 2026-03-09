use std::io::{Read, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::database::sstable::{errors::SSTableError, metadata::SSTMetadata};

// we need to define at which index this table is required
#[derive(Clone, Debug)]
pub enum VersionOperation {
    Del {
        level: u32,
        id: uuid::Uuid,
    },
    Add {
        level: u32,
        id: uuid::Uuid,
        index: u32,
    },
    AddWithMeta {
        level: u32,
        meta: SSTMetadata,
        index: u32,
    },
}
impl VersionOperation {
    pub fn encode(&self, buff: &mut dyn Write) -> Result<u64, SSTableError> {
        let mut bytes_written = 0;
        let (op_bytes, level, id, index) = match self {
            Self::Del { level, id } => (b"del", level, id, None),
            Self::AddWithMeta { level, meta, index } => (b"add", level, &meta.id, Some(index)),
            Self::Add { level, id, index } => (b"add", level, id, Some(index)),
        };

        // write operation
        bytes_written += 3;
        buff.write_all(op_bytes)?;

        // write level
        bytes_written += 4;
        buff.write_u32::<BigEndian>(*level)?;

        let id_bytes = id.as_bytes();

        // write id
        bytes_written += 16;
        buff.write_all(id_bytes)?;

        // write index
        if let Some(index) = index {
            bytes_written += 4;
            buff.write_u32::<BigEndian>(*index)?;
        }

        Ok(bytes_written)
    }
    pub fn decode(reader: &mut dyn Read) -> Result<Self, SSTableError> {
        // read op
        let mut op_bytes = [0u8; 3];
        reader.read_exact(&mut op_bytes)?;

        // read level
        let level = reader.read_u32::<BigEndian>()?;

        // read id
        let mut id_buf = [0u8; 16];
        reader.read_exact(&mut id_buf)?;
        let id = uuid::Uuid::from_bytes(id_buf);
        if &op_bytes == b"del" {
            return Ok(Self::Del { level, id });
        } else if &op_bytes == b"add" {
            let index = reader.read_u32::<BigEndian>()?;
            return Ok(Self::Add { level, id, index });
        } else {
            return Err(SSTableError::General(
                "Invalid operation in verions operation".into(),
            ));
        }
    }
}

#[derive(Debug)]
pub struct VersionUpdate {
    pub wal_offset: u64,
    pub operations: Vec<VersionOperation>,
}

impl VersionUpdate {
    pub fn new(wal_offset: u64) -> Self {
        Self {
            wal_offset,
            operations: Vec::new(),
        }
    }
    pub fn add_operation(&mut self, op: VersionOperation) {
        self.operations.push(op);
    }
    pub fn get_operations(&self) -> &Vec<VersionOperation> {
        &self.operations
    }
    pub fn clear(&mut self) {
        self.operations.clear();
    }
    pub fn encode(&self, buf: &mut dyn Write) -> Result<u64, SSTableError> {
        let mut total_bytes = 0;
        // first we will write the wal offset
        buf.write_u64::<BigEndian>(self.wal_offset)?;
        total_bytes += 8;

        // second we will endcode the len of operation
        buf.write_u32::<BigEndian>(self.operations.len() as u32)?;
        total_bytes += 4;
        for op in &self.operations {
            total_bytes += op.encode(buf)?;
        }
        Ok(total_bytes)
    }
    pub fn decode(reader: &mut dyn Read) -> Result<Self, SSTableError> {
        let wal_offset = reader.read_u64::<BigEndian>()?;
        let op_count = reader.read_u32::<BigEndian>()?;
        let mut update = Self {
            wal_offset,
            operations: vec![],
        };
        for _i in 0..op_count {
            match VersionOperation::decode(reader) {
                Ok(op) => {
                    update.operations.push(op);
                }
                Err(e) => {
                    return Err(SSTableError::General(format!(
                        "Error while reading the operation from version update {:?}",
                        e
                    )));
                }
            }
        }
        Ok(update)
    }
}
