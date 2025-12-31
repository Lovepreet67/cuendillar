use crate::database::common::Entry;

#[derive(Default)]
pub struct WALEntry {
    pub payload: Vec<u8>,
}
impl WALEntry {
    pub fn from_entry<T: Entry>(entry: &T) -> Self {
        let mut buff = Vec::new();
        entry.encode(&mut buff);
        Self { payload: buff }
    }
    pub fn to_entry<T: Entry>(&mut self) -> Result<T, std::io::Error> {
        T::decode(&mut self.payload.as_slice())
    }
}
