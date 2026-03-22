# Cuendillar example: KV shell

This package is a **separate binary** that depends on **`cuendillar`** via a normal path dependency (`cuendillar = { path = "../.." }`), the same shape you would use for a Git path or a crates.io release once published.

## Interactive mode (default)

Run the binary **once** and leave the terminal open. You get a **`kv>`** prompt and type commands until **`quit`** or **`exit`**:

```bash
cd examples/cuendillar_example_kv
cargo run
```

Example session:

```text
Cuendillar KV shell — commands: help | quit
(connected to database)

kv> help
kv> init wipe
kv> open
kv> put user:1 {"name":"Ada"}
kv> get user:1
kv> scan 50
kv> quit
```

- If the database directory does not exist yet, run **`init`** (or **`init wipe`** to reset), then **`open`** to connect.
- After **`init wipe`**, run **`open`** again so the shell picks up a fresh **`Database`** handle.

You can also start the shell explicitly:

```bash
cargo run -- shell
```

Global options go **before** the subcommand, e.g.:

```bash
cargo run -- --config /abs/path/to/example_config.toml
```

## One-shot mode (scripts / CI)

Pass a **single subcommand** to run one operation and exit (same as before):

```bash
cargo run -- put mykey my value
cargo run -- get mykey
cargo run -- --config example_config.toml init --wipe
```

## Paths and `CONFIG_PATH`

Paths in **`example_config.toml`** are **relative to the process current working directory**, not relative to the TOML file.

- **Interactive:** usually `cd examples/cuendillar_example_kv` so **`./data/...`** in the config matches that folder.
- **From repo root:** pass **`--config examples/cuendillar_example_kv/example_config.toml`**; data then lands under **`./data/cuendillar_demo`** at the repo root.

The binary sets **`CONFIG_PATH`** from **`--config`** at startup.

## REPL commands

| Command | Purpose |
|---------|---------|
| **`help`** | Short reference. |
| **`quit`** / **`exit`** | Leave the shell. |
| **`init [wipe]`** | Ensure parent dirs; **`wipe`** deletes the DB root first. |
| **`open`** | (Re)open **`Database`** after **`init`** or errors. |
| **`put KEY VALUE...`** | Value = rest of the line after the key. |
| **`get KEY`** | |
| **`del KEY`** | Tombstone delete. |
| **`scan [LIMIT]`** | Sorted iteration; optional numeric limit (default 200). |

## Copying into your own project

1. Copy this folder (or only `Cargo.toml` + `src` + your own config).
2. Point `cuendillar` at a path, git revision, or registry version.
3. Keep **`compaction.root_dir`** and **`cleaning.root_dir`** identical (see `CONFIG_TUNING.md` in the main crate).
