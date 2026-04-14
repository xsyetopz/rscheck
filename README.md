# rscheck

`rscheck` checks Rust workspaces against layered project rules.

It covers rules that sit above compiler errors and default Clippy linting:
architecture boundaries, API contracts, duplication thresholds, scoped
overrides, and repo-specific constraints.

Site: <https://xsyetopz.github.io/rscheck/>

## Install

```bash
cargo install rscheck-cli
```

That installs:
- `rscheck`
- `cargo-rscheck`

Git install still works if you want the repo head instead of the published
crate:

```bash
cargo install --git https://github.com/xsyetopz/rscheck --locked rscheck-cli
```

## Check A Workspace

Initialize a policy file:

```bash
rscheck init
```

Run checks:

```bash
rscheck check
```

Choose a report format:

```bash
rscheck check --format text
rscheck check --format json
rscheck check --format sarif --output rscheck.sarif.json
rscheck check --format html --output rscheck.html
```

Preview edits:

```bash
rscheck check --dry-run
```

Apply safe fixes:

```bash
rscheck check --write
```

Apply safe and unsafe fixes:

```bash
rscheck check --write --unsafe
```

Pass extra cargo/clippy arguments after `--`:

```bash
rscheck check -- --all-targets --all-features
```

Inspect rule metadata:

```bash
rscheck list-rules
rscheck explain shape.file_complexity
```

## Configuration

`rscheck` reads `.rscheck.toml` from the workspace root. The v2 config uses
engine mode, adapters, dot-form rule IDs, and path-scoped overrides.

```toml
version = 2

[engine]
semantic = "auto"

[workspace]
include = ["**/*.rs"]
exclude = ["target/**", ".git/**"]

[output]
format = "text"
# output = "path/to/report.txt"

[adapters.clippy]
enabled = true
args = []

[rules."architecture.qualified_module_paths"]
level = "deny"
roots = ["std", "core", "alloc", "crate"]
allow_crate_root_macros = true
allow_crate_root_consts = true
allow_crate_root_fn_calls = true

[rules."shape.file_complexity"]
level = "warn"
mode = "cyclomatic"
max_file = 200
max_fn = 25

[rules."architecture.banned_dependencies"]
level = "deny"
banned_prefixes = ["std::process::Command"]

[[scope]]
include = ["crates/rscheck-cli/**"]

[scope.rules."shape.file_complexity"]
max_file = 260
max_fn = 35
```

`semantic = "auto"` runs syntax rules on stable and runs semantic checks when
the semantic backend is available. `require` fails the run if that backend is
missing. `off` disables semantic rules.

## Rule Families

Current built-in families include:
- `architecture.*`
- `design.*`
- `shape.*`
- `portability.*`

Current rules include:
- `architecture.qualified_module_paths`
- `architecture.banned_dependencies`
- `architecture.layer_direction`
- `design.public_api_errors`
- `design.repeated_type_aliases`
- `shape.file_complexity`
- `shape.duplicate_logic`
- `shape.responsibility_split`
- `portability.absolute_literal_paths`

## Use As A Library

The library crates are not published on crates.io yet.

```toml
[dependencies]
rscheck = { git = "https://github.com/xsyetopz/rscheck" }
```

```rust
use rscheck::analysis::Workspace;
use rscheck::config::Policy;
use rscheck::runner::Runner;
use std::env;

let root = env::current_dir().unwrap();
let policy = Policy::default();
let ws = Workspace::new(root).load_files(&policy).unwrap();
let report = Runner::run(&ws, &policy).unwrap();
println!("{:#?}", report.worst_severity());
```

## Release

Manual release flow for the `rscheck-cli` package:

```bash
cargo test -p rscheck-cli
cargo package --list -p rscheck-cli
cargo publish --dry-run -p rscheck-cli
cargo publish -p rscheck-cli
```

Verify the published package in a fresh environment:

```bash
cargo install rscheck-cli --version <version>
rscheck --help
cargo-rscheck --help
```

Trusted publishing can be added later once the manual crates.io flow is settled.

## Local Site

```bash
bun install
bun run site:dev
```

Build the Pages artifact locally:

```bash
bun run site:build
```
