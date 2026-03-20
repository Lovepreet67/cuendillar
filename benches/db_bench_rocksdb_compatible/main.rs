use std::fs::remove_dir_all;

use cuendillar::{Database, DbConfig};

use crate::{
    report::{PhaseResult, Report},
    runners::{run_fillrandom, run_iterator_scan, run_readrandom, run_recovery, seed_base},
};

mod constants;
mod opts_parser;
mod report;
mod runners;

fn print_help() {
    println!("RocksDB-compatible benchmark (fillrandom/readrandom/iteratorscan)");
    println!("Options:");
    println!("  --benchmarks=fillrandom,readrandom,iteratorscan");
    println!(
        "  --num=1000000 (fillrandom : Number of keys to be generated, iteratorscan : key space)"
    );
    println!(
        "  --reads=1000000 (readrandom : Number of keys to be Read, 0 means same as --num, iteratorscan: number of iterator to be created)"
    );
    println!("  --key_size=16");
    println!("  --value_size=100");
    println!("  --seed=0           (0 means use current time in micros)");
    println!("  --use_existing_db=false");
    println!("  --destroy_db_after=false");
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
    // tracing_subscriber::fmt()
    //     .with_env_filter("debug")
    //     .try_init();
    let opts = match opts_parser::Opts::parse() {
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

    for benchmark in &opts.benchmarks {
        match benchmark.as_str() {
            "recovery" => {
                let result = run_recovery(config.clone(), opts.num);
                let mut report = Report::new("General Report");
                report.add_result(&result);
                report.report();
                print_phase(&result);
            }
            "fillrandom" => {
                let mut db = Database::new(config.clone()).expect("failed to create db");

                let result =
                    run_fillrandom(&mut db, opts.num, opts.key_size, opts.value_size, seed);
                let mut report = Report::new("General Report");
                report.add_result(&result);
                report.report();
                print_phase(&result);
                drop(db);
            }
            "readrandom" => {
                let mut db = Database::new(config.clone()).expect("failed to create db");
                let result = run_readrandom(&mut db, opts.reads, opts.num, opts.key_size, seed);
                let mut report = Report::new("General Report");
                report.add_result(&result);
                report.report();
                print_phase(&result);
                drop(db);
            }
            "iteratorscan" => {
                let mut db = Database::new(config.clone()).expect("failed to create db");
                let result = run_iterator_scan(
                    &mut db,
                    opts.reads, // number of scans
                    opts.num,   // key space
                    opts.key_size,
                    seed,
                );

                let mut report = Report::new("Iterator Scan Report");
                report.add_result(&result);
                report.report();
                print_phase(&result);
                drop(db);
            }
            other => {
                eprintln!("unsupported benchmark: {other}");
                std::process::exit(2);
            }
        }
    }

    // This should be done only after droping
    if opts.destroy_db_after {
        let _ = remove_dir_all(&config.root_dir);
    }
}
