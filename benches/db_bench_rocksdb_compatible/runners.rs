use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cuendillar::{Database, DbConfig};
use hdrhistogram::Histogram;

use crate::{
    constants::{LOWER_MASK, M, MATRIX_A, N, UPPER_MASK},
    report::PhaseResult,
};

pub struct Mt19937_64 {
    mt: [u64; N],
    index: usize,
}

impl Mt19937_64 {
    pub fn new(seed: u64) -> Self {
        let mut mt = [0u64; N];
        mt[0] = seed;
        let mut i = 1usize;
        while i < N {
            let prev = mt[i - 1];
            mt[i] = 6364136223846793005u64
                .wrapping_mul(prev ^ (prev >> 62))
                .wrapping_add(i as u64);
            i += 1;
        }
        Self { mt, index: N }
    }

    pub fn next_u64(&mut self) -> u64 {
        if self.index >= N {
            self.twist();
        }
        let mut y = self.mt[self.index];
        self.index += 1;

        y ^= (y >> 29) & 0x5555_5555_5555_5555;
        y ^= (y << 17) & 0x71D6_7FFF_EDA6_0000;
        y ^= (y << 37) & 0xFFF7_EEE0_0000_0000;
        y ^= y >> 43;
        y
    }

    pub fn twist(&mut self) {
        let mut i = 0usize;
        while i < N {
            let x = (self.mt[i] & UPPER_MASK) + (self.mt[(i + 1) % N] & LOWER_MASK);
            let mut xa = x >> 1;
            if (x & 1) != 0 {
                xa ^= MATRIX_A;
            }
            self.mt[i] = self.mt[(i + M) % N] ^ xa;
            i += 1;
        }
        self.index = 0;
    }
}

// Mirrors RocksDB db_bench GenerateKeyFromInt() when keys_per_prefix_ == 0.
pub fn generate_key_from_int(v: u64, key_buf: &mut [u8]) {
    let bytes_to_fill = key_buf.len().min(8);
    let mut i = 0usize;
    while i < bytes_to_fill {
        let shift = ((bytes_to_fill - i - 1) << 3) as u32;
        key_buf[i] = ((v >> shift) & 0xff) as u8;
        i += 1;
    }
    if key_buf.len() > bytes_to_fill {
        key_buf[bytes_to_fill..].fill(b'0');
    }
}

pub fn seed_base(flag_seed: u64) -> u64 {
    if flag_seed != 0 {
        return flag_seed;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_micros() as u64
}

pub fn run_fillrandom(
    db: &mut Database,
    num: u64,
    key_size: usize,
    value_size: usize,
    seed: u64,
) -> PhaseResult {
    let mut rng = Mt19937_64::new(seed);
    let mut key_buf = vec![0u8; key_size];
    let value = vec![b'v'; value_size];
    let mut hist = Histogram::new(3).expect("histogram init");

    let started = Instant::now();
    for _ in 0..num {
        let key_rand = rng.next_u64() % num;
        generate_key_from_int(key_rand, &mut key_buf);
        let op_started = Instant::now();
        db.put(&key_buf, &value).expect("fillrandom write failed");
        let micros = op_started.elapsed().as_micros() as u64;
        hist.record(micros).expect("hist record");
    }

    PhaseResult {
        timestamp: Instant::now(),
        name: "fillrandom".to_string(),
        ops: num,
        elapsed: started.elapsed(),
        found: None,
        hist,
    }
}

pub fn run_readrandom(
    db: &mut Database,
    reads: u64,
    key_space: u64,
    key_size: usize,
    seed: u64,
) -> PhaseResult {
    let mut rng = Mt19937_64::new(seed);
    let mut key_buf = vec![0u8; key_size];
    let mut found = 0u64;
    let mut hist = Histogram::new(3).expect("histogram init");

    let started = Instant::now();
    for i in 0..reads {
        let key_rand = rng.next_u64() % key_space;
        generate_key_from_int(key_rand, &mut key_buf);
        let op_started = Instant::now();
        if db.get(&key_buf).expect("readrandom find failed").is_some() {
            found += 1;
        }
        let micros = op_started.elapsed().as_micros() as u64;
        hist.record(micros).expect("hist record");
        if i % 1000000 == 0 {
            eprintln!("last iterations is  : {}", i)
        }
    }

    PhaseResult {
        timestamp: Instant::now(),
        name: "readrandom".to_string(),
        ops: reads,
        elapsed: started.elapsed(),
        found: Some(found),
        hist,
    }
}
pub fn run_iterator_scan(
    db: &mut Database,
    scans: u64,
    key_space: u64,
    key_size: usize,
    seed: u64,
) -> PhaseResult {
    let mut rng = Mt19937_64::new(seed);

    let mut start_key_buf = vec![0u8; key_size];
    let mut end_key_buf = vec![0u8; key_size];

    let mut hist = Histogram::new(3).expect("histogram init");
    let mut total_entries = 0u64;

    // size of iterator which will be created
    let scan_range = 10000;

    let started = Instant::now();

    for _ in 0..scans {
        // pick random start key
        let start_rand = rng.next_u64() % key_space;

        // define end key (bounded range)
        let end_rand = (start_rand + scan_range).min(key_space);

        generate_key_from_int(start_rand, &mut start_key_buf);
        generate_key_from_int(end_rand, &mut end_key_buf);

        let mut iter = db
            .iter(Some(&start_key_buf), Some(&end_key_buf))
            .expect("iterator creation failed");

        let scan_started = Instant::now();

        while let Some(_entry) = iter.next_owned() {
            total_entries += 1;
        }

        let micros = scan_started.elapsed().as_micros() as u64;
        hist.record(micros).expect("hist record");
    }

    PhaseResult {
        timestamp: Instant::now(),
        name: "iteratorscan".to_string(),
        ops: scans,
        elapsed: started.elapsed(),
        found: Some(total_entries),
        hist,
    }
}

pub fn run_recovery(config: Arc<DbConfig>, num: u64) -> PhaseResult {
    let mut hist = Histogram::new(3).expect("histogram init");
    let mut total_time = Duration::new(0, 0);
    for _ in 0..num {
        let start = Instant::now();
        let db = Database::new(config.clone()).expect("failed to create db");
        let recovery_time = start.elapsed();
        total_time += recovery_time;
        // record recovery as a single "op"
        hist.record(recovery_time.as_micros() as u64)
            .expect("hist record");
        let x = db.get(b"testing_key_may_not_exist");
        eprintln!("{:?}", x);
        drop(db);
    }

    PhaseResult {
        timestamp: Instant::now(),
        name: "recovery".to_string(),
        ops: num,
        elapsed: total_time,
        found: None,
        hist,
    }
}
