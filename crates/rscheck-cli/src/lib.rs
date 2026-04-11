mod cargo_clippy;
mod config_file;
mod fix_apply;
mod report_html;
mod report_sarif;

use cargo_clippy::run_clippy;
use clap::Parser;
use config_file::{default_config_path, load_from, workspace_root, write_default_config};
use fix_apply::{apply_planned_edits, plan_edits, print_dry_run};
use rscheck::analysis::Workspace;
use rscheck::config::{OutputFormat, Policy};
use rscheck::report::{FixSafety, Report};
use rscheck::rules;
use rscheck::runner::Runner;
use std::path::PathBuf;
use std::process::ExitCode as ProcessExitCode;
use std::{fs, io};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ExitCode(pub i32);

impl From<i32> for ExitCode {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl std::process::Termination for ExitCode {
    fn report(self) -> ProcessExitCode {
        ProcessExitCode::from(self.0 as u8)
    }
}

#[derive(Debug, Parser)]
#[command(name = "rscheck", version)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    Check(CheckArgs),
    ListRules,
    Explain { rule_id: String },
    Init(InitArgs),
}

#[derive(Debug, clap::Args)]
pub struct CommonOutputArgs {
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,

    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum FormatArg {
    Text,
    Json,
    Sarif,
    Html,
}

impl From<FormatArg> for OutputFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Text => OutputFormat::Text,
            FormatArg::Json => OutputFormat::Json,
            FormatArg::Sarif => OutputFormat::Sarif,
            FormatArg::Html => OutputFormat::Html,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    #[command(flatten)]
    pub out: CommonOutputArgs,

    #[arg(long, default_value_t = true)]
    pub rscheck: bool,

    #[arg(long)]
    pub write: bool,

    #[arg(long = "unsafe")]
    pub unsafe_fixes: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long, default_value_t = 10)]
    pub max_fix_iterations: u32,

    #[arg(trailing_var_arg = true)]
    pub cargo_args: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    #[arg(long)]
    pub path: Option<PathBuf>,
}

pub fn main_entry() -> ExitCode {
    init_tracing();
    let cli = Cli::parse();
    match cli.cmd {
        Command::Check(args) => run_check(args),
        Command::ListRules => run_list_rules(),
        Command::Explain { rule_id } => run_explain(&rule_id),
        Command::Init(args) => run_init(args),
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

fn run_list_rules() -> ExitCode {
    for info in rules::rule_catalog() {
        println!(
            "{}\t{:?}\t{:?}\t{:?}\t{}",
            info.id, info.family, info.backend, info.default_level, info.summary
        );
    }
    ExitCode::from(0)
}

fn run_explain(rule_id: &str) -> ExitCode {
    let info = rules::rule_catalog().into_iter().find(|i| i.id == rule_id);
    match info {
        Some(info) => {
            println!(
                "{}\n\nfamily: {:?}\nbackend: {:?}\ndefault: {:?}\nschema: {}\n\n{}\n",
                info.id,
                info.family,
                info.backend,
                info.default_level,
                info.schema,
                info.config_example
            );
            ExitCode::from(0)
        }
        None => {
            eprintln!("unknown rule: {rule_id}");
            ExitCode::from(2)
        }
    }
}

fn run_init(args: InitArgs) -> ExitCode {
    let root = match workspace_root() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };

    let path = args.path.unwrap_or_else(|| default_config_path(&root));
    if path.exists() {
        eprintln!("config already exists: {}", path.to_string_lossy());
        return ExitCode::from(1);
    }

    if let Err(err) = write_default_config(&path) {
        eprintln!("failed to write config: {err}");
        return ExitCode::from(2);
    }

    println!("{}", path.to_string_lossy());
    ExitCode::from(0)
}

fn run_check(args: CheckArgs) -> ExitCode {
    if args.write && args.dry_run {
        eprintln!("`--write` and `--dry-run` are mutually exclusive");
        return ExitCode::from(2);
    }

    let root = match workspace_root() {
        Ok(root) => root,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };

    let config_path = args
        .out
        .config
        .unwrap_or_else(|| default_config_path(&root));
    let mut policy = if config_path.exists() {
        match load_from(&config_path) {
            Ok(cfg) => cfg,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(2);
            }
        }
    } else {
        Policy::default_with_rules(rules::default_rule_settings())
    };

    if let Some(format) = args.out.format {
        policy.output.format = format.into();
    }
    if let Some(output) = args.out.output {
        policy.output.output = Some(output);
    }

    let wants_fix = args.write || args.dry_run;
    let mut last_report = Report::default();

    let iterations = if wants_fix {
        args.max_fix_iterations.max(1)
    } else {
        1
    };

    for iter in 0..iterations {
        let ws = match Workspace::new(root.clone()).load_files(&policy) {
            Ok(ws) => ws,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(2);
            }
        };

        let mut report = if args.rscheck {
            match Runner::run(&ws, &policy) {
                Ok(report) => report,
                Err(err) => {
                    eprintln!("{err}");
                    return ExitCode::from(2);
                }
            }
        } else {
            Report::default()
        };

        if policy.adapters.clippy.enabled {
            let mut clippy_args = policy.adapters.clippy.args.clone();
            clippy_args.extend(args.cargo_args.clone());
            match run_clippy(&ws.root, &clippy_args) {
                Ok(mut findings) => report.findings.append(&mut findings),
                Err(err) => {
                    eprintln!("{err}");
                    return ExitCode::from(2);
                }
            }
        }

        let planned = plan_edits(&report, args.unsafe_fixes);

        if args.dry_run {
            match print_dry_run(&planned) {
                Ok(would_change) => {
                    if let Err(err) = write_report(&report, &policy) {
                        eprintln!("{err}");
                        return ExitCode::from(2);
                    }
                    return ExitCode::from(if would_change { 1 } else { 0 });
                }
                Err(err) => {
                    eprintln!("{err}");
                    return ExitCode::from(2);
                }
            }
        }

        if args.write {
            if planned.is_empty() {
                last_report = report;
                break;
            }
            match apply_planned_edits(&planned) {
                Ok(applied) => {
                    if !applied {
                        last_report = report;
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("{err}");
                    return ExitCode::from(2);
                }
            }

            last_report = report;
            if iter + 1 == iterations {
                break;
            }
            continue;
        }

        if let Err(err) = write_report(&report, &policy) {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
        return ExitCode::from(report.worst_severity().exit_code());
    }

    if let Err(err) = write_report(&last_report, &policy) {
        eprintln!("{err}");
        return ExitCode::from(2);
    }

    ExitCode::from(last_report.worst_severity().exit_code())
}

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("failed to serialize report")]
    Serialize(#[source] serde_json::Error),
    #[error("failed to write output")]
    Write(#[source] io::Error),
}

fn write_report(report: &Report, policy: &Policy) -> Result<(), OutputError> {
    let text = match policy.output.format {
        OutputFormat::Text => text_report(report),
        OutputFormat::Json => {
            serde_json::to_string_pretty(report).map_err(OutputError::Serialize)?
        }
        OutputFormat::Sarif => serde_json::to_string_pretty(&report_sarif::to_sarif(report))
            .map_err(OutputError::Serialize)?,
        OutputFormat::Html => report_html::to_html(report),
    };

    match &policy.output.output {
        Some(path) => fs::write(path, text).map_err(OutputError::Write),
        None => {
            print!("{text}");
            Ok(())
        }
    }
}

fn text_report(report: &Report) -> String {
    let mut out = String::new();
    for f in &report.findings {
        let fixable = f.fixes.iter().any(|fx| fx.safety == FixSafety::Safe);
        match &f.primary {
            Some(span) => {
                out.push_str(&format!(
                    "{}:{}:{}: {:?}[{}]: {}{}\n",
                    span.file,
                    span.start.line,
                    span.start.column,
                    f.severity,
                    f.rule_id,
                    f.message,
                    if fixable { " (fixable)" } else { "" }
                ));
            }
            None => {
                out.push_str(&format!(
                    "{:?}[{}]: {}{}\n",
                    f.severity,
                    f.rule_id,
                    f.message,
                    if fixable { " (fixable)" } else { "" }
                ));
            }
        }
    }
    if !report.summary.skipped_rules.is_empty() {
        out.push_str("\nskipped semantic rules:\n");
        for rule in &report.summary.skipped_rules {
            out.push_str(&format!("- {rule}\n"));
        }
    }
    out
}
