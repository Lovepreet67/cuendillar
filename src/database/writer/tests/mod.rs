use std::{path::PathBuf, str::FromStr};

use crate::database::{
    memtable::vector_memtable::VectorMemtable, tests::common::Entity,
    wal::wal_writer::default_wal_writer::DefaultWALWriter, writer::Writer,
};

#[test]
fn database_writer_test_e2e() {
    let mut writer: Writer<Entity, VectorMemtable<Entity>, DefaultWALWriter> =
        Writer::new(PathBuf::from_str("./wal").unwrap(), None, Some(5));
    writer.write(Entity::new("id1", "name1", 4));
    writer.write(Entity::new("id2", "name2", 4));
    writer.write(Entity::new("id3", "name3", 4));
    writer.write(Entity::new("id4", "name4", 4));
    writer.write(Entity::new("id5", "name5", 4));
    writer.write(Entity::new("id6", "name6", 4));
    writer.write(Entity::new("id7", "name6", 4));
    writer.write(Entity::new("id8", "name6", 4));
    writer.write(Entity::new("id9", "name6", 4));
}
