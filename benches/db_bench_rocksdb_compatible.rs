use cuendillar::database::{Entry, config::DbConfig, db_engine::Engine};
use hdrhistogram::Histogram;
use std::fs::remove_dir_all;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const N: usize = 312;
const M: usize = 156;
const MATRIX_A: u64 = 0xB502_6F5A_A966_19E9;
const LOWER_MASK: u64 = (1u64 << 31) - 1;
const UPPER_MASK: u64 = !LOWER_MASK;

#[derive(Debug, Clone)]
struct Opts {
    benchmarks: Vec<String>,
    num: u64,
    reads: u64,
    key_size: usize,
    value_size: usize,
    seed: u64,
    use_existing_db: bool,
    destroy_db_after: bool,
}

#[derive(Debug)]
struct PhaseResult {
    name: String,
    ops: u64,
    elapsed: Duration,
    found: Option<u64>,
    hist: Histogram<u64>,
}

impl Opts {
    fn parse() -> Result<Self, String> {
        let mut opts = Self {
            benchmarks: vec!["fillrandom".to_string(), "readrandom".to_string()],
            num: 1_000_000,
            reads: 1_000_000,
            key_size: 16,
            value_size: 100,
            seed: 0,
            use_existing_db: false,
            destroy_db_after: false,
        };

        for arg in std::env::args().skip(1) {
            if let Some(v) = arg.strip_prefix("--benchmarks=") {
                opts.benchmarks = v
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            } else if let Some(v) = arg.strip_prefix("--num=") {
                opts.num = v
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --num: {e}"))?;
            } else if let Some(v) = arg.strip_prefix("--reads=") {
                opts.reads = v
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --reads: {e}"))?;
            } else if let Some(v) = arg.strip_prefix("--key_size=") {
                opts.key_size = v
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --key_size: {e}"))?;
            } else if let Some(v) = arg.strip_prefix("--value_size=") {
                opts.value_size = v
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --value_size: {e}"))?;
            } else if let Some(v) = arg.strip_prefix("--seed=") {
                opts.seed = v
                    .parse::<u64>()
                    .map_err(|e| format!("invalid --seed: {e}"))?;
            } else if let Some(v) = arg.strip_prefix("--use_existing_db=") {
                opts.use_existing_db = parse_bool(v)?;
            } else if let Some(v) = arg.strip_prefix("--destroy_db_after=") {
                opts.destroy_db_after = parse_bool(v)?;
            } else if arg == "--help" || arg == "-h" {
                print_help();
                std::process::exit(0);
            } else if arg == "--bench" {
                // Passed by `cargo bench` even when harness = false.
                continue;
            } else {
                return Err(format!("unknown argument: {arg}"));
            }
        }

        if opts.reads == 0 {
            opts.reads = opts.num;
        }
        if opts.key_size == 0 {
            return Err("--key_size must be > 0".to_string());
        }
        if opts.benchmarks.is_empty() {
            return Err("--benchmarks cannot be empty".to_string());
        }
        Ok(opts)
    }
}

fn parse_bool(v: &str) -> Result<bool, String> {
    match v {
        "1" | "true" | "TRUE" | "True" => Ok(true),
        "0" | "false" | "FALSE" | "False" => Ok(false),
        _ => Err(format!("invalid bool value: {v}")),
    }
}

fn print_help() {
    println!("RocksDB-compatible benchmark (fillrandom/readrandom)");
    println!("Options:");
    println!("  --benchmarks=fillrandom,readrandom");
    println!("  --num=1000000");
    println!("  --reads=1000000    (0 means same as --num)");
    println!("  --key_size=16");
    println!("  --value_size=100");
    println!("  --seed=0           (0 means use current time in micros)");
    println!("  --use_existing_db=false");
    println!("  --destroy_db_after=false");
}

struct Mt19937_64 {
    mt: [u64; N],
    index: usize,
}

impl Mt19937_64 {
    fn new(seed: u64) -> Self {
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

    fn next_u64(&mut self) -> u64 {
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

    fn twist(&mut self) {
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
fn generate_key_from_int(v: u64, key_buf: &mut [u8]) {
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

fn seed_base(flag_seed: u64) -> u64 {
    if flag_seed != 0 {
        return flag_seed;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_micros() as u64
}

fn run_fillrandom(
    engine: &mut Engine,
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
        engine
            .write(Entry::Row {
                key: &key_buf,
                value: &value,
            })
            .expect("fillrandom write failed");
        let micros = op_started.elapsed().as_micros() as u64;
        hist.record(micros).expect("hist record");
    }

    PhaseResult {
        name: "fillrandom".to_string(),
        ops: num,
        elapsed: started.elapsed(),
        found: None,
        hist,
    }
}

fn run_readrandom(
    engine: &mut Engine,
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
    for _ in 0..reads {
        let key_rand = rng.next_u64() % key_space;
        generate_key_from_int(key_rand, &mut key_buf);
        let op_started = Instant::now();
        if engine
            .find(&key_buf)
            .expect("readrandom find failed")
            .is_some()
        {
            found += 1;
        }
        let micros = op_started.elapsed().as_micros() as u64;
        hist.record(micros).expect("hist record");
    }

    PhaseResult {
        name: "readrandom".to_string(),
        ops: reads,
        elapsed: started.elapsed(),
        found: Some(found),
        hist,
    }
}

fn print_phase(result: &PhaseResult) {
    let secs = result.elapsed.as_secs_f64();
    let micros_per_op = if result.ops == 0 {
        0.0
    } else {
        result.elapsed.as_micros() as f64 / result.ops as f64
    };
    let ops_per_sec = if secs > 0.0 {
        result.ops as f64 / secs
    } else {
        0.0
    };
    println!(
        "{name:<12}: {micros:.2} micros/op {ops:.0} ops/sec; count={count}; p50={p50} p95={p95} p99={p99} p99.9={p999} max={max}",
        name = result.name,
        micros = micros_per_op,
        ops = ops_per_sec,
        count = result.ops,
        p50 = result.hist.value_at_quantile(0.50),
        p95 = result.hist.value_at_quantile(0.95),
        p99 = result.hist.value_at_quantile(0.99),
        p999 = result.hist.value_at_quantile(0.999),
        max = result.hist.max(),
    );
    if let Some(found) = result.found {
        println!("  ({found} of {} found)", result.ops);
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();
    let opts = match Opts::parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            print_help();
            std::process::exit(2);
        }
    };

    let seed = seed_base(opts.seed);
    println!(
        "Using seed={} benchmarks={} num={} reads={} key_size={} value_size={}",
        seed,
        opts.benchmarks.join(","),
        opts.num,
        opts.reads,
        opts.key_size,
        opts.value_size
    );

    let config = DbConfig::get_config().expect("failed to load db config");
    if !opts.use_existing_db {
        let _ = remove_dir_all(&config.root_dir);
    }

    let mut engine = Engine::new(config.clone()).expect("failed to create engine");

    for benchmark in &opts.benchmarks {
        match benchmark.as_str() {
            "fillrandom" => {
                let result =
                    run_fillrandom(&mut engine, opts.num, opts.key_size, opts.value_size, seed);
                print_phase(&result);
            }
            "readrandom" => {
                // Mirrors db_bench typical use_existing_db flow between phases.
                drop(engine);
                engine = Engine::new(config.clone()).expect("failed to reopen engine");
                let result = run_readrandom(&mut engine, opts.reads, opts.num, opts.key_size, seed);
                print_phase(&result);
            }
            other => {
                eprintln!("unsupported benchmark: {other}");
                std::process::exit(2);
            }
        }
    }

    if opts.destroy_db_after {
        drop(engine);
        let _ = remove_dir_all(&config.root_dir);
    }
}
