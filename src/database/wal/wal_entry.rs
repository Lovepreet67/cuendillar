use crate::database::{Entry, OwnedEntry};

#[derive(Default)]
pub struct WALEntry {
    pub payload: Vec<u8>,
}
impl WALEntry {
    pub fn from_entry(entry: Entry) -> Self {
        let mut buff = Vec::new();
        entry.encode(&mut buff);
        Self { payload: buff }
    }
    pub fn to_entry(&mut self) -> Result<OwnedEntry, std::io::Error> {
        OwnedEntry::decode(&mut self.payload.as_slice())
    }
}
