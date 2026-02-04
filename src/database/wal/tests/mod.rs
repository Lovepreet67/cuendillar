use std::{fs::File, path::Path};

use crate::database::{
    config::CONFIG,
    wal::{WAL, errors::WALError},
};
mod default_wal;

pub fn test_wal_append(wal: &mut impl WAL) {
    let mut prev = 0;
    for i in 0..100 as i32 {
        let offset = wal.append_log(&i.to_be_bytes()).unwrap();
        assert!(prev == 0 || prev < offset);
        prev = offset;
    }
}

pub fn test_wal_read(wal: &mut impl WAL) {
    for i in 0..100 as i32 {
        wal.append_log(&i.to_be_bytes()).unwrap();
    }
    // now we will read the wal
    let wal_iterator = wal.read(0).unwrap();
    let mut curr: i32 = 0;
    for i in wal_iterator {
        let val = i.unwrap();
        assert_eq!(val, curr.to_be_bytes());
        curr += 1;
    }
    assert_eq!(curr, 100);
}

pub fn test_wal_rotation(wal: &mut impl WAL) {
    let wal_record_size = 40;
    let iterations = ((CONFIG.wal.wal_file_size * 5) / wal_record_size) - 1; // there will be multiple files
    for i in 0..iterations {
        wal.append_log(&i.to_be_bytes()).unwrap();
    }
    // now we will read the wal
    let wal_iterator = wal.read(0).unwrap();
    let mut curr: u64 = 0;
    for i in wal_iterator {
        let val = i.unwrap();
        assert_eq!(val, curr.to_be_bytes());
        curr += 1;
    }
    assert_eq!(curr, iterations);
}

pub fn test_wal_flush(wal: &mut impl WAL) {
    let wal_record_size = 40;
    let iterations = ((CONFIG.wal.wal_file_size * 4) / wal_record_size) - 1; // there will be multiple files
    // we will flush almost half the logs
    let mut wals_to_flush = 0;
    for i in 0..iterations {
        let offset = wal.append_log(&i.to_be_bytes()).unwrap();
        if i == iterations / 2 {
            wals_to_flush = offset;
        }
    }
    // as it is clear that logs may be readable even after flush
    // but current implementation delete file which have all logs < offset this way
    // we can assume that the file 0.wal will be deleted
    wal.flush_wal(wals_to_flush).unwrap();
    assert_eq!(Some(WALError::OffsetUnderflow), wal.read(0).err());
    // now we will read the wal
    let wal_iterator = wal.read(wals_to_flush).unwrap();
    let mut curr: u64 = iterations / 2;
    for i in wal_iterator {
        let val = i.unwrap();
        assert_eq!(val, curr.to_be_bytes());
        curr += 1;
    }
}

pub fn test_wal_corruption(wal: &mut impl WAL) {
    let wal_record_size = 40;
    let iterations = ((CONFIG.wal.wal_file_size * 3) / wal_record_size) - 1; // there will be multiple files
    // we will flush almost half the logs
    let mut wals_to_flush = 0;
    for i in 0..iterations {
        let offset = wal.append_log(&i.to_be_bytes()).unwrap();
        if i == iterations / 2 {
            wals_to_flush = offset;
        }
    }
    // now we will read the wal by missaliging the offset
    let mut wal_iterator = wal.read(wals_to_flush + 3).unwrap();
    assert!(wal_iterator.next().unwrap().is_err()); // there will be value but it should be error
}

pub fn test_wal_invalid_file_name(wal: &mut impl WAL, root_dir: &Path) {
    let invalid_file = root_dir.join("invalid_name.wal");
    File::create(invalid_file).unwrap();
    assert!(matches!(wal.read(0), Err(WALError::InvalidFileName(_))));
}

pub fn test_wal_max_palyload(wal: &mut impl WAL) {
    let payload = vec![0u8; (CONFIG.wal.wal_max_payload_len + 3) as usize];
    assert!(matches!(
        wal.append_log(&payload),
        Err(WALError::PayloadLengthOutOfBound(_))
    ));
}

/// This function assumes that proved wal new without any existing entries
pub fn test_wal_empty(wal: &mut impl WAL) {
    assert!(matches!(wal.read(0).unwrap().next(), None));
}

/// This function assumes that proved wal is wal which is using the same root in which there are some logs (recovered)
pub fn test_wal_recovery_on_new(wal: &mut impl WAL) {
    let offset = wal.append_log(b"test").unwrap();
    assert!(offset > 0);
}
