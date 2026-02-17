use std::collections::VecDeque;

use crate::database::{
    config::memtable_config::{
        MemtableConfig, {MemtableMangerVariant, MemtableVariant},
    },
    memtable::{
        Memtable,
        errors::MemtableError,
        manager::{MemtableManager, default_manager::DefaultManger},
        vector_memtable::VectorMemtable,
    },
};

pub fn build_memtable_manager(
    memtable_config: &MemtableConfig,
    memtable_id: Option<uuid::Uuid>,
) -> Result<Box<dyn MemtableManager>, MemtableError> {
    let memtable_generator: Box<dyn Fn(Option<uuid::Uuid>) -> Box<dyn Memtable>> =
        match memtable_config.variant {
            MemtableVariant::Vector => Box::new(|id| Box::new(VectorMemtable::new(id))),
        };
    let active_memtable = memtable_generator(memtable_id);
    match memtable_config.manager_variant {
        MemtableMangerVariant::Default => {
            let manager = DefaultManger::intialize(
                active_memtable,
                VecDeque::default(),
                memtable_config.max_memtable_size as u64,
                memtable_generator,
            );
            return Ok(Box::new(manager));
        }
    };
}
