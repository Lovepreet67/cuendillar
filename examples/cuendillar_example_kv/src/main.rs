//! Demo application: a small key–value shell built on **Cuendillar** as a normal Rust dependency.
//!
//! Run once with `cargo run` (or after building the binary), then type commands at the **`kv>`**
//! prompt. You can still run a single subcommand for scripts, e.g. `cargo run -- put a b`.

use std::{
    fs,
    io::{BufRead, Write, stdin, stdout},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cuendillar::{Database, DbConfig, EngineError, OwnedEntry};

/// Key–value demo backed by the Cuendillar embedded engine.
#[derive(Parser, Debug)]
#[command(
    name = "cuendillar-example-kv",
    version,
    arg_required_else_help = false,
    about = "Interactive KV shell (or pass one subcommand for a single shot). Uses cuendillar as a path dependency."
)]
struct Cli {
    /// TOML file passed to `DbConfig::get_config()` via `CONFIG_PATH`.
    #[arg(
        long,
        global = true,
        env = "CONFIG_PATH",
        default_value = "example_config.toml"
    )]
    config: PathBuf,

    /// If omitted, starts the interactive shell (terminal stays open).
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create parent dirs for the DB root (optional) and show resolved paths. Use `--wipe` to delete existing data.
    Init {
        #[arg(long)]
        wipe: bool,
    },
    /// Insert or update a UTF-8 key and value.
    Put { key: String, value: Vec<String> },
    /// Look up a key (prints value, NOT_FOUND, or TOMBSTONE).
    Get { key: String },
    /// Delete a key (tombstone).
    Del { key: String },
    /// Scan keys in sorted order. If `prefix` is set, only keys starting with that UTF-8 prefix (prefix scan).
    Scan {
        #[arg(long, default_value_t = 200)]
        limit: usize,
    },
    /// Start the interactive shell (same as running with no subcommand).
    Shell,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    apply_config_path(&cli.config)?;

    match &cli.command {
        None | Some(Commands::Shell) => run_repl(),
        Some(Commands::Init { wipe }) => cmd_init(wipe),
        Some(Commands::Put { key, value }) => {
            let db = open_db()?;
            let val = value.join(" ");
            cmd_put(&db, key, &val)
        }
        Some(Commands::Get { key }) => {
            let db = open_db()?;
            cmd_get(&db, key)
        }
        Some(Commands::Del { key }) => {
            let db = open_db()?;
            cmd_del(&db, key)
        }
        Some(Commands::Scan { limit }) => {
            let db = open_db()?;
            cmd_scan(&db, *limit)
        }
    }
}

fn run_repl() -> Result<()> {
    println!("Cuendillar KV shell — commands: help | quit");
    println!("Tip: `init` then `open` on first use, or `open` if the database already exists.\n");

    let mut db: Option<Database> = match open_db() {
        Ok(d) => {
            println!("(connected to database)\n");
            Some(d)
        }
        Err(e) => {
            println!("(not connected: {e:?})\nType `init` then `open`, or `help`.\n");
            None
        }
    };

    let stdin = stdin();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let mut line = String::new();

    loop {
        print!("kv> ");
        stdout().flush()?;
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            println!();
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = parts[0].to_lowercase();
        let result = match cmd.as_str() {
            "quit" | "exit" => break,
            "help" | "?" => {
                print_repl_help();
                Ok(())
            }
            "init" => {
                let wipe = parts.get(1).is_some_and(|a| a.eq_ignore_ascii_case("wipe"));
                cmd_init(&wipe)
            }
            "open" | "connect" => match open_db() {
                Ok(d) => {
                    db = Some(d);
                    println!("OK — database open");
                    Ok(())
                }
                Err(e) => Err(e),
            },
            "put" => {
                if parts.len() < 3 {
                    println!("usage: put <key> <value...>");
                    Ok(())
                } else {
                    let key = parts[1];
                    let value = parts[2..].join(" ");
                    let d = require_db(&db)?;
                    cmd_put(d, key, &value)
                }
            }
            "get" => {
                if parts.len() < 2 {
                    println!("usage: get <key>");
                    Ok(())
                } else {
                    let d = require_db(&db)?;
                    cmd_get(d, parts[1])
                }
            }
            "del" | "delete" => {
                if parts.len() < 2 {
                    println!("usage: del <key>");
                    Ok(())
                } else {
                    let d = require_db(&db)?;
                    cmd_del(d, parts[1])
                }
            }
            "scan" => {
                let d = require_db(&db)?;
                let limit = parse_scan_args(&parts[1..]);
                cmd_scan(d, limit)
            }
            other => {
                println!("unknown command `{other}` — type `help`");
                Ok(())
            }
        };

        if let Err(e) = result {
            eprintln!("error: {e:#}");
        }
    }

    println!("bye.");
    Ok(())
}

fn parse_scan_args(args: &[&str]) -> usize {
    let default_limit = 200usize;
    match args.len() {
        1 => {
            if let Ok(lim) = args[0].parse::<usize>() {
                lim
            } else {
                default_limit
            }
        }
        _ => default_limit,
    }
}

fn print_repl_help() {
    println!(
        r#"
  init [wipe]     prepare paths; optional wipe existing DB
  open            (re)connect after init or config change
  put KEY VAL...  insert/update (value = rest of line)
  get KEY
  del KEY
  scan  [LIMIT]   sorted walk with row limit
  help
  quit | exit
"#
    );
}

fn require_db(db: &Option<Database>) -> Result<&Database> {
    db.as_ref()
        .ok_or_else(|| anyhow::anyhow!("database not open — run `open` (after `init` if needed)"))
}

fn apply_config_path(path: &Path) -> Result<()> {
    let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    // before spawning workers, and `DbConfig::get_config` reads `CONFIG_PATH` after this.
    unsafe {
        std::env::set_var("CONFIG_PATH", abs.to_string_lossy().as_ref());
    }
    Ok(())
}

fn load_config() -> Result<std::sync::Arc<DbConfig>> {
    DbConfig::get_config().map_err(|e| anyhow::anyhow!("{:?}", e))
}

fn open_db() -> Result<Database> {
    let config = load_config()?;
    Database::new(config).map_err(map_engine_err)
}

fn map_engine_err(e: EngineError) -> anyhow::Error {
    anyhow::anyhow!("{:?}", e)
}

fn cmd_init(wipe: &bool) -> Result<()> {
    let config = load_config()?;
    let root = &config.root_dir;
    if *wipe && root.exists() {
        fs::remove_dir_all(root).with_context(|| format!("wipe {:?}", root))?;
        println!("Removed {:?}", root);
    }
    if let Some(parent) = root.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {:?}", parent))?;
    }
    println!("Cuendillar demo database root: {:?}", root);
    println!("SSTables: {:?}", config.sstable_root_dir);
    println!("WAL: {:?}", config.wal.wal_dir);
    println!(
        "CONFIG_PATH={}",
        std::env::var("CONFIG_PATH").unwrap_or_default()
    );
    Ok(())
}

fn cmd_put(db: &Database, key: &str, value: &str) -> Result<()> {
    let seq = db
        .put(key.as_bytes(), value.as_bytes())
        .map_err(map_engine_err)?;
    println!(
        "OK put seq={seq} key_len={} value_len={}",
        key.len(),
        value.len()
    );
    Ok(())
}

fn cmd_get(db: &Database, key: &str) -> Result<()> {
    match db.get(key.as_bytes()).map_err(map_engine_err)? {
        None => println!("NOT_FOUND"),
        Some(OwnedEntry::Row {
            key: k, value: v, ..
        }) => {
            let val = String::from_utf8_lossy(&v);
            println!("OK key={} value={}", String::from_utf8_lossy(&k), val);
        }
        Some(OwnedEntry::Tombstone { key: k, .. }) => {
            println!("TOMBSTONE key={}", String::from_utf8_lossy(&k));
        }
    }
    Ok(())
}

fn cmd_del(db: &Database, key: &str) -> Result<()> {
    let seq = db.delete(key.as_bytes()).map_err(map_engine_err)?;
    println!("OK delete seq={seq}");
    Ok(())
}

fn cmd_scan(db: &Database, limit: usize) -> Result<()> {
    let mut iter = db.iter(None, None).map_err(map_engine_err)?;
    let mut n = 0usize;
    while let Some(entry) = iter.next_owned() {
        match entry {
            OwnedEntry::Row {
                key: k, value: v, ..
            } => {
                println!(
                    "{} = {}",
                    String::from_utf8_lossy(&k),
                    String::from_utf8_lossy(&v)
                );
            }
            OwnedEntry::Tombstone { key: k, .. } => {
                println!("{} = <tombstone>", String::from_utf8_lossy(&k));
            }
        }
        n += 1;
        if n >= limit {
            break;
        }
    }
    println!("-- listed {n} entries (limit {limit})");
    Ok(())
}
