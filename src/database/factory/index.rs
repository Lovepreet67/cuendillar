use crate::database::{
    config::{IndexConfig, variants::IndexVariant},
    sstable::metadata::index::{SSTIndex, default_index::DefaultIndex},
};

pub fn build_index(config: &IndexConfig) -> Box<dyn SSTIndex> {
    match config.variant {
        IndexVariant::Default => Box::new(DefaultIndex::new()),
    }
}
