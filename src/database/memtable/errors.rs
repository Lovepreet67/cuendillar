#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemtableError {
    NotFound,
    Deleted,
}
