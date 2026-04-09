mod cargo_clippy;
mod config_file;
mod report_html;
mod report_sarif;

use cargo_clippy::run_clippy;
use clap::Parser;
use config_file::{
    FileConfig, OutputFormat, default_config_path, workspace_root, write_default_config,
};
use rscheck::analysis::Workspace;
use rscheck::report::Report;
use rscheck::rules;
use rscheck::runner::Runner;
use std::path::PathBuf;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ExitCode(pub i32);

impl From<i32> for ExitCode {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl std::process::Termination for ExitCode {
    fn report(self) -> std::process::ExitCode {
        std::process::ExitCode::from(self.0 as u8)
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

    #[arg(long)]
    pub with_clippy: Option<bool>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum FormatArg {
    Human,
    Json,
    Sarif,
    Html,
}

impl From<FormatArg> for OutputFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Human => OutputFormat::Human,
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

    #[arg(last = true, trailing_var_arg = true)]
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
    for info in rules::all_rule_infos() {
        println!("{}\t{}", info.id, info.summary);
    }
    ExitCode::from(0)
}

fn run_explain(rule_id: &str) -> ExitCode {
    let infos = rules::all_rule_infos();
    let info = infos.into_iter().find(|i| i.id == rule_id);
    match info {
        Some(info) => {
            println!("{}\n\n{}", info.id, info.summary);
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
    let mut file_config = if config_path.exists() {
        match FileConfig::load_from(&config_path) {
            Ok(cfg) => cfg,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(2);
            }
        }
    } else {
        FileConfig::default()
    };

    if let Some(format) = args.out.format {
        file_config.output.format = format.into();
    }
    if let Some(output) = args.out.output {
        file_config.output.output = Some(output);
    }
    if let Some(with_clippy) = args.out.with_clippy {
        file_config.output.with_clippy = with_clippy;
    }

    let ws = match Workspace::new(root).load_files(&file_config.core) {
        Ok(ws) => ws,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };

    let mut report = Report::default();

    if args.rscheck {
        report = Runner::run(&ws, &file_config.core);
    }

    if file_config.output.with_clippy {
        match run_clippy(&ws.root, &args.cargo_args) {
            Ok(mut findings) => report.findings.append(&mut findings),
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(2);
            }
        }
    }

    if let Err(err) = write_report(&report, &file_config) {
        eprintln!("{err}");
        return ExitCode::from(2);
    }

    ExitCode::from(report.worst_severity().exit_code())
}

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("failed to serialize report")]
    Serialize(#[source] serde_json::Error),
    #[error("failed to write output")]
    Write(#[source] std::io::Error),
}

fn write_report(report: &Report, config: &FileConfig) -> Result<(), OutputError> {
    let text = match config.output.format {
        OutputFormat::Human => human_report(report),
        OutputFormat::Json => {
            serde_json::to_string_pretty(report).map_err(OutputError::Serialize)?
        }
        OutputFormat::Sarif => serde_json::to_string_pretty(&report_sarif::to_sarif(report))
            .map_err(OutputError::Serialize)?,
        OutputFormat::Html => report_html::to_html(report),
    };

    match &config.output.output {
        Some(path) => std::fs::write(path, text).map_err(OutputError::Write),
        None => {
            print!("{text}");
            Ok(())
        }
    }
}

fn human_report(report: &Report) -> String {
    let mut out = String::new();
    for f in &report.findings {
        match &f.primary {
            Some(span) => {
                out.push_str(&format!(
                    "{}:{}:{}: {:?}[{}]: {}\n",
                    span.file, span.start.line, span.start.column, f.severity, f.rule_id, f.message
                ));
            }
            None => {
                out.push_str(&format!("{:?}[{}]: {}\n", f.severity, f.rule_id, f.message));
            }
        }
    }
    out
}
