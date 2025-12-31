use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::database::common::Entry;

#[derive(Debug, PartialEq, Clone)]
pub struct Entity {
    id: String,
    name: String,
    class: u8,
    deleted: bool,
}
impl Entity {
    pub fn new(id: &str, name: &str, class: u8) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            class,
            deleted: false,
        }
    }
}
impl Entry for Entity {
    fn get_key(&self) -> &[u8] {
        self.id.as_bytes()
    }
    fn is_deleted(&self) -> bool {
        self.deleted
    }
    fn mark_deleted(&mut self) {
        self.deleted = true;
    }
    fn encode<W: std::io::Write>(&self, buf: &mut W) -> Result<usize, std::io::Error> {
        let mut bytes_written = 0;
        // writing key first
        buf.write_u32::<BigEndian>(self.get_key().len() as u32)?;
        bytes_written += 4;
        bytes_written += buf.write(self.get_key())?;
        // writing name second
        buf.write_u32::<BigEndian>(self.name.len() as u32)?;
        bytes_written += 4;
        bytes_written += buf.write(self.name.as_bytes())?;
        // writing class
        buf.write_u8(self.class)?;
        bytes_written += 1;
        // writing is deleted
        buf.write_u8(self.deleted as u8)?;
        bytes_written += 1;
        Ok(bytes_written)
    }
    fn decode<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        //read key
        let key_len = reader.read_u32::<BigEndian>()? as usize;
        let mut key_buf = vec![0u8; key_len];
        reader.read_exact(&mut key_buf)?;
        let key = String::from_utf8(key_buf).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid key utf8")
        })?;

        // read name
        let name_len = reader.read_u32::<BigEndian>()? as usize;
        let mut name_buf = vec![0u8; name_len];
        reader.read_exact(&mut name_buf)?;
        let name = String::from_utf8(name_buf).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid name utf8")
        })?;

        //read class
        let class = reader.read_u8()?;

        // read deleted flag
        let deleted = reader.read_u8()? != 0;

        Ok(Self {
            id: key,
            name,
            class,
            deleted,
        })
    }
}
