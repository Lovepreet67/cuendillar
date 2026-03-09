use cuendillar::database::{config::DbConfig, db_engine::Engine};
use hdrhistogram::Histogram;

use std::{
    fs::{File, create_dir_all, remove_dir_all},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};

use chrono::Utc;

/* ---------------- Utilities ---------------- */

fn today_date() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn output_dir(workload: &str) -> PathBuf {
    let dir = PathBuf::from("bench_result")
        .join(today_date())
        .join(workload);

    create_dir_all(&dir).unwrap();
    dir
}

/* ---------------- Workload ---------------- */

pub enum Operation {
    Get(Vec<u8>),
    Put(Vec<u8>, Vec<u8>),
    Del(Vec<u8>),
}

/* ---------------- Failure Tracking ---------------- */

pub struct FailureInfo {
    pub op_index: u64,
    pub line: String,
    pub error: String,
}

/* ---------------- Stats ---------------- */

pub struct Stats {
    pub put: Histogram<u64>,
    pub get: Histogram<u64>,
    pub del: Histogram<u64>,
    pub total_ops: u64,
    pub total_runtime: Duration,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            put: Histogram::new(3).unwrap(),
            get: Histogram::new(3).unwrap(),
            del: Histogram::new(3).unwrap(),
            total_ops: 0,
            total_runtime: Duration::ZERO,
        }
    }

    fn write_section(file: &mut File, name: &str, hist: &Histogram<u64>) {
        if hist.len() == 0 {
            return;
        }

        writeln!(file, "{}:", name).unwrap();
        writeln!(file, "  count: {}", hist.len()).unwrap();
        writeln!(file, "  avg: {:.2} µs", hist.mean()).unwrap();
        writeln!(file, "  p50: {} µs", hist.value_at_quantile(0.50)).unwrap();
        writeln!(file, "  p95: {} µs", hist.value_at_quantile(0.95)).unwrap();
        writeln!(file, "  p99: {} µs", hist.value_at_quantile(0.99)).unwrap();
        writeln!(file, "  p999: {} µs", hist.value_at_quantile(0.999)).unwrap();
        writeln!(file, "  max: {} µs", hist.max()).unwrap();
        writeln!(file).unwrap();
    }

    pub fn report(
        &self,
        out_dir: &Path,
        workload: &str,
        failure: Option<&FailureInfo>,
        panic_mode: bool,
    ) {
        let filename = if panic_mode {
            "report_panic.txt"
        } else {
            "report.txt"
        };

        let mut file = File::create(out_dir.join(filename)).unwrap();

        let runtime_secs = self.total_runtime.as_secs_f64();
        let throughput = if runtime_secs > 0.0 {
            self.total_ops as f64 / runtime_secs
        } else {
            0.0
        };
        writeln!(
            file,
            "Using config {:?}",
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "./default_config.toml".to_owned())
        )
        .unwrap();

        writeln!(file, "Workload: {}", workload).unwrap();
        writeln!(file, "Total Ops Completed: {}", self.total_ops).unwrap();
        writeln!(file, "Runtime: {:.2} sec", runtime_secs).unwrap();
        writeln!(file, "Throughput: {:.2} ops/sec", throughput).unwrap();
        writeln!(file).unwrap();

        if let Some(f) = failure {
            writeln!(file, "FAILED AT OP: {}", f.op_index).unwrap();
            writeln!(file, "LINE: {}", f.line).unwrap();
            writeln!(file, "ERROR: {}", f.error).unwrap();
            writeln!(file).unwrap();
        }

        Self::write_section(&mut file, "PUT", &self.put);
        Self::write_section(&mut file, "GET", &self.get);
        Self::write_section(&mut file, "DEL", &self.del);

        println!("Benchmark finished → {:?}", out_dir.join(filename));
    }
}

/* ---------------- Benchmark Logic ---------------- */

fn timed<F>(f: F, metric: &mut Histogram<u64>) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_micros() as u64;

    if result.is_ok() {
        metric.record(elapsed).unwrap();
    }

    result
}

pub fn execute_op(engine: &mut Engine, op: Operation) -> Result<(), String> {
    match op {
        Operation::Get(key) => {
            engine.find(&key).map_err(|e| format!("{:?}", e))?;
        }
        Operation::Del(key) => {
            engine
                .write(cuendillar::database::Entry::Tombstone { key: &key })
                .map_err(|e| format!("{:?}", e))?;
        }
        Operation::Put(key, value) => {
            engine
                .write(cuendillar::database::Entry::Row {
                    key: &key,
                    value: &value,
                })
                .map_err(|e| format!("{:?}", e))?;
        }
    }
    Ok(())
}

pub fn run_workload(engine: &mut Engine, path: &str) -> (Stats, Option<FailureInfo>) {
    let mut stats = Stats::new();
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    let start_total = Instant::now();

    for line in reader.lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split(',').collect();

        let result = match parts[0] {
            "GET" => timed(
                || execute_op(engine, Operation::Get(parts[1].into())),
                &mut stats.get,
            ),
            "PUT" => timed(
                || execute_op(engine, Operation::Put(parts[1].into(), parts[2].into())),
                &mut stats.put,
            ),
            "DEL" => timed(
                || execute_op(engine, Operation::Del(parts[1].into())),
                &mut stats.del,
            ),
            _ => Err(format!("Unknown operation: {}", line)),
        };

        match result {
            Ok(_) => {
                stats.total_ops += 1;
                if stats.total_ops % 1000 == 0 {
                    sleep(Duration::from_millis(3));
                }
            }
            Err(e) => {
                stats.total_runtime = start_total.elapsed();
                let total_ops = stats.total_ops;
                return (
                    stats,
                    Some(FailureInfo {
                        op_index: total_ops,
                        line,
                        error: e,
                    }),
                );
            }
        }
    }

    stats.total_runtime = start_total.elapsed();
    (stats, None)
}

/* ---------------- Entry Point ---------------- */

pub fn run(workload: &str, file: &str) {
    let out_dir = output_dir(workload);
    let config = DbConfig::get_config().unwrap();
    let mut engine = Engine::new(config).unwrap();

    println!("Warming up...");
    sleep(Duration::from_secs(5));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_workload(&mut engine, file)
    }));

    match result {
        Ok((stats, failure)) => {
            stats.report(&out_dir, workload, failure.as_ref(), false);
        }
        Err(_) => {
            println!("⚠ Benchmark panicked!");
            let stats = Stats::new();
            stats.report(&out_dir, workload, None, true);
        }
    }

    drop(engine);
    let _ = remove_dir_all("./table");
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init()
        .unwrap();
    let active_workload = std::env::var("ACTIVE_WORKLOAD").unwrap_or_else(|_| "10k".to_owned());

    let active_workload_file = format!("workload/{}.txt", active_workload);

    run(&active_workload, &active_workload_file);
}
