// #[cfg(test)]
// mod test {
//     use std::{fs::remove_dir_all, io::BufRead, path::PathBuf, str::FromStr};

//     use crate::database::wal::WAL;

//     #[test]
//     pub fn test_wal() {
//         // create a new wal
//         let wal_res = WAL::new(PathBuf::from_str("./wal").unwrap());
//         assert!(wal_res.is_ok());
//         let mut wal = wal_res.unwrap();
//         let log_id_opt = wal.append_log("Testing a log\n").unwrap();
//         assert!(log_id_opt.is_some());
//         let log_id = log_id_opt.unwrap();
//         wal.append_log("Testing a log1\n").unwrap();
//         wal.append_log("Testing a log2\n").unwrap();
//         wal.append_log("Testing a log3\n").unwrap();
//         wal.append_log("Testing a log4\n").unwrap();
//         let log_id2_res = wal.new_log_file();
//         assert!(log_id2_res.is_ok());
//         let mut first_log = wal.read_log(&log_id).unwrap();
//         let mut buff = String::new();
//         first_log.read_line(&mut buff).unwrap();
//         assert_eq!(&buff, "Testing a log\n");
//         buff.clear();
//         first_log.read_line(&mut buff).unwrap();
//         assert_eq!(&buff, "Testing a log1\n");
//         buff.clear();
//         first_log.read_line(&mut buff).unwrap();
//         assert_eq!(&buff, "Testing a log2\n");
//         buff.clear();
//         first_log.read_line(&mut buff).unwrap();
//         assert_eq!(&buff, "Testing a log3\n");
//         buff.clear();
//         first_log.read_line(&mut buff).unwrap();
//         assert_eq!(&buff, "Testing a log4\n");
//         drop(wal);
//         // cleanup
//         remove_dir_all("./wal").unwrap();
//     }
// }
