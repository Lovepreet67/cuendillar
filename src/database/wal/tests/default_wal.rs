use std::{fs::remove_dir_all, path::PathBuf, str::FromStr};

use crate::database::wal::{WAL, default_wal::DefaultWAL, tests::test_wal};

#[test]
fn test_default_wal() {
    let mut wal_res = DefaultWAL::new(PathBuf::from_str("./wal_test").unwrap()).unwrap();
    test_wal(&mut wal_res);
    remove_dir_all("./wal_test").unwrap();
}
