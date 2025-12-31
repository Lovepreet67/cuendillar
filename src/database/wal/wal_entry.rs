use std::default;

#[derive(Default)]
pub struct WALEntry<'a> {
    pub payload: &'a [u8],
}
