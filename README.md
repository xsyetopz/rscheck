# rscheck

`rscheck` is a static analysis tool for Rust workspaces, inspired by cppcheck.

It is designed to catch highly configurable issues that are out of scope for Clippy
and to provide a library API so it can be embedded in other tools.

## Install (from Git)

This repo is not published on crates.io. Install the CLI directly from Git:

```bash
cargo install --git https://github.com/xsyetopz/rscheck --locked rscheck-cli
```

That installs:
- `rscheck` (the CLI)
- `cargo-rscheck` (so you can also run `cargo rscheck`)

## Use in a repository

Initialize a local config (optional but recommended):

```bash
rscheck init
```

This writes `.rscheck.toml` at the workspace root (defaults are used if the file
does not exist).

Run checks:

```bash
rscheck check
```

By default `rscheck` also runs `cargo clippy --workspace` and merges its
diagnostics into the same report. Disable Clippy if you want only `rscheck` rules:

```bash
rscheck check --with-clippy=false
```

Pass extra arguments to Clippy/Cargo after `--`:

```bash
rscheck check -- --all-targets --all-features
```

### Output formats

```bash
rscheck check --format human
rscheck check --format json
rscheck check --format sarif --output rscheck.sarif.json
rscheck check --format html --output rscheck.html
```

### Applying fixes (Biome-style)

`rscheck` can apply edits suggested by its own rules and by Clippy (when Clippy
emits machine-applicable replacements).

Preview changes as diffs:

```bash
rscheck check --dry-run
```

Apply safe fixes:

```bash
rscheck check --write
```

Apply safe + unsafe fixes:

```bash
rscheck check --write --unsafe
```

## Configuration

`rscheck` reads `.rscheck.toml` (created by `rscheck init`). Example:

```toml
[output]
format = "human" # or: "json" | "sarif" | "html"
with_clippy = true
# output = "path/to/report.txt"

[rules.absolute_module_paths]
level = "deny"
allow_prefixes = []
roots = ["std", "core", "alloc", "crate"]
allow_crate_root_macros = true
allow_crate_root_consts = true
allow_crate_root_fn_calls = true

[rules.absolute_filesystem_paths]
level = "warn"
check_comments = false
allow_globs = []
allow_regex = []

include = ["**/*.rs"]
exclude = ["target/**", ".git/**"]
```

## Use as a library crate

In another Rust project, add a Git dependency:

```toml
[dependencies]
rscheck = { git = "https://github.com/xsyetopz/rscheck" }
```

Minimal usage:

```rust
use rscheck::analysis::Workspace;
use rscheck::config::Config;
use rscheck::runner::Runner;

let root = std::env::current_dir().unwrap();
let cfg = Config::default();
let ws = Workspace::new(root).load_files(&cfg).unwrap();
let report = Runner::run(&ws, &cfg);
println!("{:#?}", report.worst_severity());
```
