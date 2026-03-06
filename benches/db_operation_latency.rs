use cuendillar::database::{config::DbConfig, db_engine::Engine};
use hdrhistogram::Histogram;
use plotters::{
    chart::ChartBuilder,
    prelude::{IntoDrawingArea, Rectangle, SVGBackend},
    style::{BLUE, Color, WHITE},
};

use std::{
    fs::{File, create_dir_all, remove_dir_all, write},
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

fn export_histogram_csv(hist: &Histogram<u64>, path: &Path) {
    let mut file = File::create(path).unwrap();
    writeln!(file, "latency_us,count").unwrap();

    for v in hist.iter_recorded() {
        writeln!(
            file,
            "{},{}",
            v.value_iterated_to(),
            v.count_since_last_iteration()
        )
        .unwrap();
    }
}

/* ---------------- Workload ---------------- */

pub enum Operation {
    Get(Vec<u8>),
    Put(Vec<u8>, Vec<u8>),
    Del(Vec<u8>),
}

/* ---------------- Stats ---------------- */

pub struct Stats {
    pub put: Histogram<u64>,
    pub get: Histogram<u64>,
    pub del: Histogram<u64>,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            put: Histogram::new(3).unwrap(),
            get: Histogram::new(3).unwrap(),
            del: Histogram::new(3).unwrap(),
        }
    }

    pub fn plot_latency(hist: &Histogram<u64>, title: &str, path: &Path) {
        let root = SVGBackend::new(path, (900, 600)).into_drawing_area();
        root.fill(&WHITE).unwrap();

        let max_latency = hist.max().max(1);
        let max_count = hist
            .iter_recorded()
            .map(|v| v.count_since_last_iteration())
            .max()
            .unwrap_or(1);

        let mut chart = ChartBuilder::on(&root)
            .caption(title, ("sans-serif", 28))
            .margin(20)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .build_cartesian_2d(1u64..max_latency, 0u64..max_count)
            .unwrap();

        chart
            .configure_mesh()
            .x_desc("Latency (µs)")
            .y_desc("Count")
            .draw()
            .unwrap();

        chart
            .draw_series(hist.iter_recorded().map(|v| {
                let x = v.value_iterated_to();
                let count = v.count_since_last_iteration();
                Rectangle::new(
                    [(x.saturating_sub(1), 0), (x, count)],
                    BLUE.mix(0.7).filled(),
                )
            }))
            .unwrap();

        root.present().unwrap();
    }

    pub fn write_html(out_dir: &Path) {
        let html = r#"
<html>
  <body>
    <h1>DB Latency Report</h1>
    <h2>PUT</h2>
    <object type="image/svg+xml" data="put.svg"></object>
    <h2>GET</h2>
    <object type="image/svg+xml" data="get.svg"></object>
    <h2>DEL</h2>
    <object type="image/svg+xml" data="del.svg"></object>
  </body>
</html>
"#;
        write(out_dir.join("report.html"), html).unwrap();
    }

    pub fn report(&self, out_dir: &Path) {
        println!(
            "PUT  p50={}µs p95={}µs p99={}µs max={}µs",
            self.put.value_at_quantile(0.50),
            self.put.value_at_quantile(0.95),
            self.put.value_at_quantile(0.99),
            self.put.max()
        );
        Self::plot_latency(&self.put, "PUT", &out_dir.join("put.svg"));
        export_histogram_csv(&self.put, &out_dir.join("put_hist.csv"));

        println!(
            "GET  p50={}µs p95={}µs p99={}µs max={}µs",
            self.get.value_at_quantile(0.50),
            self.get.value_at_quantile(0.95),
            self.get.value_at_quantile(0.99),
            self.get.max()
        );
        Self::plot_latency(&self.get, "GET", &out_dir.join("get.svg"));
        export_histogram_csv(&self.get, &out_dir.join("get_hist.csv"));

        println!(
            "DEL  p50={}µs p95={}µs p99={}µs max={}µs",
            self.del.value_at_quantile(0.50),
            self.del.value_at_quantile(0.95),
            self.del.value_at_quantile(0.99),
            self.del.max()
        );
        Self::plot_latency(&self.del, "DEL", &out_dir.join("del.svg"));
        export_histogram_csv(&self.del, &out_dir.join("del_hist.csv"));

        Self::write_html(out_dir);
    }
}

/* ---------------- Benchmark Logic ---------------- */

fn timed<F>(f: F, metric: &mut Histogram<u64>)
where
    F: FnOnce(),
{
    let start = Instant::now();
    f();
    let elapsed = start.elapsed().as_micros() as u64;
    metric.record(elapsed).unwrap();
}

pub fn run_workload(engine: &mut Engine, path: &str) -> Stats {
    let mut stats = Stats::new();
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split(',').collect();

        match parts[0] {
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
            _ => panic!("Unknown operation: {}", line),
        }
    }
    stats
}

pub fn execute_op(engine: &mut Engine, op: Operation) {
    match op {
        Operation::Get(key) => {
            engine.find(&key).unwrap();
        }
        Operation::Del(key) => engine
            .write(cuendillar::database::Entry::Tombstone { key: &key })
            .unwrap(),
        Operation::Put(key, value) => engine
            .write(cuendillar::database::Entry::Row {
                key: &key,
                value: &value,
            })
            .unwrap(),
    };
}

/* ---------------- Entry Points ---------------- */

pub fn run(workload: &str, file: &str) {
    let out_dir = output_dir(workload);
    let config = DbConfig::get_config().unwrap();
    let mut engine = Engine::new(config).unwrap();

    sleep(Duration::from_secs(10)); // warm-up
    let stats = run_workload(&mut engine, file);
    stats.report(&out_dir);
    drop(engine);
    remove_dir_all("./table").unwrap();
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();
    let active_workload = std::env::var("ACTIVE_WORKLOAD").unwrap_or_else(|_| "10k".to_owned());
    let active_workload_file = format!("workload/{}.txt", active_workload);
    run(&active_workload, &active_workload_file);
}
