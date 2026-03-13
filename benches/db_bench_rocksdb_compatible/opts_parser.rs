use crate::print_help;

#[derive(Debug, Clone)]
pub struct Opts {
    pub benchmarks: Vec<String>,
    pub num: u64,
    pub reads: u64,
    pub key_size: usize,
    pub value_size: usize,
    pub seed: u64,
    pub use_existing_db: bool,
    pub destroy_db_after: bool,
}

impl Opts {
    pub fn parse() -> Result<Self, String> {
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
