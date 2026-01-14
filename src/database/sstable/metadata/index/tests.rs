use std::{io::Cursor, ops::Range};

use crate::database::sstable::metadata::index::SSTIndex;

pub fn sst_index_test_add_and_get(mut index: impl SSTIndex) {
    let enteries: Vec<(Vec<u8>, u64)> = vec![
        ("001".as_bytes().into(), 100),
        ("007".as_bytes().into(), 150),
        ("090".as_bytes().into(), 220),
        ("140".as_bytes().into(), 280),
        ("200".as_bytes().into(), 360),
        ("210".as_bytes().into(), 400),
    ];
    for i in &enteries {
        index.add_entry(&i.0, i.1);
    }
    index.add_last_offset(500);
    assert_eq!(
        index.get_offset(b"001"),
        Some(Range {
            start: 100,
            end: 150
        })
    );
    assert_eq!(
        index.get_offset(b"140"),
        Some(Range {
            start: 280,
            end: 360
        })
    );
    assert_eq!(
        index.get_offset(b"210"),
        Some(Range {
            start: 400,
            end: 500
        })
    );
    assert_eq!(
        index.get_offset(b"240"),
        Some(Range {
            start: 400,
            end: 500
        })
    );
    assert_eq!(index.get_offset(b"0"), None);
}

pub fn sst_index_test_persistant<T: SSTIndex>(mut index: impl SSTIndex) {
    let enteries: Vec<(Vec<u8>, u64)> = vec![
        ("001".as_bytes().into(), 100),
        ("007".as_bytes().into(), 150),
        ("090".as_bytes().into(), 220),
        ("140".as_bytes().into(), 280),
        ("200".as_bytes().into(), 360),
        ("210".as_bytes().into(), 400),
    ];
    for i in &enteries {
        index.add_entry(&i.0, i.1);
    }
    index.add_last_offset(500);
    let mut buf = Vec::new();
    index.serialize(&mut buf).unwrap();

    let mut cursor = Cursor::new(buf);

    let mut new_index = T::deserialize(&mut cursor).unwrap();
    assert_eq!(
        new_index.get_offset(b"001"),
        Some(Range {
            start: 100,
            end: 150
        })
    );
    assert_eq!(
        new_index.get_offset(b"140"),
        Some(Range {
            start: 280,
            end: 360
        })
    );
    assert_eq!(
        new_index.get_offset(b"210"),
        Some(Range {
            start: 400,
            end: 500
        })
    );
    assert_eq!(
        new_index.get_offset(b"240"),
        Some(Range {
            start: 400,
            end: 500
        })
    );
    assert_eq!(new_index.get_offset(b"0"), None);
}
