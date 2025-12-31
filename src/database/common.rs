pub(crate) trait Entry: 'static + Sized {
    fn get_key(&self) -> &[u8];
    fn mark_deleted(&mut self);
    fn is_deleted(&self) -> bool;
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error>;
    fn decode<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error>;
}
