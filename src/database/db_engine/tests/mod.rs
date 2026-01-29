use std::{fs::remove_dir_all, path::PathBuf, str::FromStr};

use crate::database::{Entry, db_engine::Engine};

#[test]
pub fn db_engine_test_insert_find_and_delete() {
    let mut engine = Engine::new(Some(PathBuf::from_str("./wal").unwrap())).unwrap();
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
    remove_dir_all("./wal").unwrap();
}
