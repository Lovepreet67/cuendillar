use std::fs::create_dir_all;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use std::{fs::File, time::Instant};

use chrono::Utc;
use hdrhistogram::Histogram;

#[derive(Clone, Debug)]
pub struct PhaseResult {
    pub timestamp: Instant,
    pub name: String,
    pub ops: u64,
    pub elapsed: Duration,
    pub found: Option<u64>,
    pub hist: Histogram<u64>,
}

pub struct Report {
    title: String,
    results: Vec<PhaseResult>,
}
impl Report {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            results: vec![],
        }
    }

    /// Appends a phase result to the aggregate report
    pub fn add_result(&mut self, result: &PhaseResult) {
        self.results.push(result.clone());
    }

    fn write_section(file: &mut File, result: &PhaseResult) {
        if result.ops == 0 {
            return;
        }

        writeln!(file, "Timestamp: {:?}", result.timestamp).unwrap();
        writeln!(file, "Name : {:?}", result.name).unwrap();
        writeln!(file, "Count: {}", result.ops).unwrap();
        writeln!(file, "Duration: {} secs", result.elapsed.as_secs()).unwrap();
        writeln!(file, "p50:   {} us", result.hist.value_at_quantile(0.50)).unwrap();
        writeln!(file, "p95:   {} us", result.hist.value_at_quantile(0.95)).unwrap();
        writeln!(file, "p99:   {} us", result.hist.value_at_quantile(0.99)).unwrap();
        writeln!(file, "p99.9: {} us", result.hist.value_at_quantile(0.999)).unwrap();
        writeln!(file, "Max:   {} us", result.hist.max()).unwrap();
        writeln!(file).unwrap();
    }

    pub fn report(&self) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let out_dir = PathBuf::from_str("bench_result")
            .unwrap()
            .join(format!("{}", today));
        create_dir_all(&out_dir).expect("Error while creating report dir");
        let filepath = out_dir.join("rd_report.txt");
        let mut file = File::options()
            .append(true)
            .create(true)
            .open(&filepath)
            .expect("Failed to create report file");

        writeln!(file, "Cuendillar Benchmark Report").unwrap();
        writeln!(file, "===========================").unwrap();
        writeln!(file, "{}", self.title).unwrap();
        let total_ops = self.results.iter().fold(0, |acc, result| acc + result.ops);
        let total_runtime = self
            .results
            .iter()
            .fold(Duration::from_micros(0), |acc, result| acc + result.elapsed);
        let throughput = if total_runtime.as_secs_f64() > 0.0 {
            total_ops as f64 / total_runtime.as_secs_f64()
        } else {
            0.0
        };

        writeln!(file, "Total Ops: {}", total_ops).unwrap();
        writeln!(file, "Total Runtime: {:?} sec", total_runtime).unwrap();
        writeln!(file, "AVG Throughput: {:?} ops/sec", throughput).unwrap();
        writeln!(file).unwrap();

        for result in &self.results {
            Self::write_section(&mut file, &result);
        }

        println!("\nBenchmark finished → {:?}", filepath);
    }
}
