use crate::database::{Entry, config::DbConfig, db_engine::Engine};

#[test]
pub fn db_engine_test_insert_find_and_delete() {
    let (config, _root_dir) = DbConfig::get_test_config();
    let mut engine = Engine::new(config).unwrap();
    engine
        .write(Entry::Row {
            key: b"id1",
            value: b"value1",
        })
        .unwrap();
    engine
        .write(Entry::Row {
            key: b"id2",
            value: b"value2",
        })
        .unwrap();
    engine
        .write(Entry::Row {
            key: b"id3",
            value: b"value3",
        })
        .unwrap();
    assert_eq!(
        engine.find(b"id1").unwrap(),
        Some(
            Entry::Row {
                key: b"id1",
                value: b"value1",
            }
            .into()
        )
    );
    assert_eq!(
        engine.find("id2".as_bytes()).unwrap(),
        Some(
            Entry::Row {
                key: b"id2",
                value: b"value2",
            }
            .into()
        )
    );
    assert_eq!(
        engine.find("id3".as_bytes()).unwrap(),
        Some(
            Entry::Row {
                key: b"id3",
                value: b"value3",
            }
            .into()
        )
    );
    engine.write(Entry::Tombstone { key: b"id2" }).unwrap();
    assert!(engine.find("id2".as_bytes()).is_ok());
    let result = engine.find(b"id2").unwrap();
    assert_eq!(result, Some(Entry::Tombstone { key: b"id2" }.into()));
}
