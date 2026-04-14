mod cargo_clippy;
mod config_file;
mod fix_apply;
mod report_html;
mod report_sarif;
mod text_report;
mod toolchain;

use cargo_clippy::run_clippy;
use clap::Parser;
use config_file::{default_config_path, load_from, workspace_root, write_default_config};
use fix_apply::{ApplyError, PlannedEdits, apply_planned_edits, plan_edits, print_dry_run};
use rscheck::analysis::Workspace;
use rscheck::config::{OutputFormat, Policy, ToolchainMode};
use rscheck::report::{AdapterRun, Report};
use rscheck::rules;
use rscheck::runner::Runner;
use std::path::{Path, PathBuf};
use std::process::ExitCode as ProcessExitCode;
use std::{fs, io};
use text_report::render_text_report;
use toolchain::{ResolvedToolchain, resolve_toolchain};

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

    #[arg(long, value_enum)]
    pub toolchain: Option<ToolchainArg>,

    #[arg(trailing_var_arg = true)]
    pub cargo_args: Vec<String>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ToolchainArg {
    Current,
    Auto,
    Nightly,
}

impl From<ToolchainArg> for ToolchainMode {
    fn from(value: ToolchainArg) -> Self {
        match value {
            ToolchainArg::Current => ToolchainMode::Current,
            ToolchainArg::Auto => ToolchainMode::Auto,
            ToolchainArg::Nightly => ToolchainMode::Nightly,
        }
    }
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
    if let Err(code) = validate_check_args(&args) {
        return code;
    }

    let root = match resolve_workspace_root() {
        Ok(root) => root,
        Err(code) => return code,
    };
    let policy = match load_check_policy(&args, &root) {
        Ok(policy) => policy,
        Err(code) => return code,
    };
    let resolved_toolchain = match resolve_toolchain(&policy, args.toolchain.map(Into::into)) {
        Ok(toolchain) => toolchain,
        Err(err) => return toolchain_error_to_exit_code(err),
    };

    execute_check_iterations(args, root, policy, resolved_toolchain)
}

fn validate_check_args(args: &CheckArgs) -> Result<(), ExitCode> {
    if args.write && args.dry_run {
        eprintln!("`--write` and `--dry-run` are mutually exclusive");
        return Err(ExitCode::from(2));
    }
    Ok(())
}

fn resolve_workspace_root() -> Result<PathBuf, ExitCode> {
    workspace_root().map_err(|err| {
        eprintln!("{err}");
        ExitCode::from(2)
    })
}

fn load_check_policy(args: &CheckArgs, root: &Path) -> Result<Policy, ExitCode> {
    let config_path = args
        .out
        .config
        .clone()
        .unwrap_or_else(|| default_config_path(root));
    let mut policy = if config_path.exists() {
        load_from(&config_path).map_err(|err| {
            eprintln!("{err}");
            ExitCode::from(2)
        })?
    } else {
        Policy::default_with_rules(rules::default_rule_settings())
    };
    if let Some(format) = args.out.format {
        policy.output.format = format.into();
    }
    if let Some(output) = args.out.output.clone() {
        policy.output.output = Some(output);
    }
    Ok(policy)
}

fn execute_check_iterations(
    args: CheckArgs,
    root: PathBuf,
    policy: Policy,
    resolved_toolchain: ResolvedToolchain,
) -> ExitCode {
    let iterations = iteration_count(&args);
    let mut last_report = Report::default();

    for is_last in (0..iterations).map(|iter| iter + 1 == iterations) {
        let ws = match load_workspace(root.clone(), &policy) {
            Ok(ws) => ws,
            Err(code) => return code,
        };
        let report = match build_iteration_report(&args, &policy, &resolved_toolchain, &ws) {
            Ok(report) => report,
            Err(code) => return code,
        };
        let action = match handle_iteration(&args, &policy, &report) {
            Ok(action) => action,
            Err(code) => return code,
        };
        match action {
            IterationAction::Return(code) => return code,
            IterationAction::ContinueWithReport(report) => {
                last_report = *report;
                if is_last {
                    break;
                }
            }
        }
    }

    finish_write_mode(&last_report, &policy)
}

fn iteration_count(args: &CheckArgs) -> u32 {
    if args.write || args.dry_run {
        args.max_fix_iterations.max(1)
    } else {
        1
    }
}

enum IterationAction {
    Return(ExitCode),
    ContinueWithReport(Box<Report>),
}

fn handle_iteration(
    args: &CheckArgs,
    policy: &Policy,
    report: &Report,
) -> Result<IterationAction, ExitCode> {
    let planned = plan_edits(report, args.unsafe_fixes);
    if args.dry_run {
        return dry_run_result(policy, report, &planned).map(IterationAction::Return);
    }
    if !args.write {
        return write_and_return(policy, report).map(IterationAction::Return);
    }
    apply_write_iteration(report, &planned)
        .map(Box::new)
        .map(IterationAction::ContinueWithReport)
}

fn dry_run_result(
    policy: &Policy,
    report: &Report,
    planned: &PlannedEdits,
) -> Result<ExitCode, ExitCode> {
    let would_change = print_dry_run(planned).map_err(io_error_to_exit_code)?;
    write_report(report, policy).map_err(output_error_to_exit_code)?;
    Ok(ExitCode::from(if would_change { 1 } else { 0 }))
}

fn write_and_return(policy: &Policy, report: &Report) -> Result<ExitCode, ExitCode> {
    write_report(report, policy).map_err(output_error_to_exit_code)?;
    Ok(ExitCode::from(report.worst_severity().exit_code()))
}

fn apply_write_iteration(report: &Report, planned: &PlannedEdits) -> Result<Report, ExitCode> {
    if planned.is_empty() {
        return Ok(report.clone());
    }
    let _applied = apply_planned_edits(planned).map_err(io_error_to_exit_code)?;
    Ok(report.clone())
}

fn finish_write_mode(report: &Report, policy: &Policy) -> ExitCode {
    if let Err(err) = write_report(report, policy) {
        return output_error_to_exit_code(err);
    }
    ExitCode::from(report.worst_severity().exit_code())
}

fn load_workspace(root: PathBuf, policy: &Policy) -> Result<Workspace, ExitCode> {
    Workspace::new(root).load_files(policy).map_err(|err| {
        eprintln!("{err}");
        ExitCode::from(2)
    })
}

fn build_iteration_report(
    args: &CheckArgs,
    policy: &Policy,
    resolved_toolchain: &ResolvedToolchain,
    ws: &Workspace,
) -> Result<Report, ExitCode> {
    let mut report = run_rscheck_engine(args.rscheck, policy, resolved_toolchain, ws)?;
    run_clippy_adapter(
        &mut report,
        policy,
        resolved_toolchain,
        ws,
        &args.cargo_args,
    )?;
    report.summary.toolchain = Some(resolved_toolchain.summary());
    Ok(report)
}

fn run_rscheck_engine(
    enabled: bool,
    policy: &Policy,
    resolved_toolchain: &ResolvedToolchain,
    ws: &Workspace,
) -> Result<Report, ExitCode> {
    if !enabled {
        return Ok(Report::default());
    }

    Runner::run_with_semantic_status(ws, policy, resolved_toolchain.semantic_status()).map_err(
        |err| {
            eprintln!("{err}");
            ExitCode::from(2)
        },
    )
}

fn run_clippy_adapter(
    report: &mut Report,
    policy: &Policy,
    resolved_toolchain: &ResolvedToolchain,
    ws: &Workspace,
    cargo_args: &[String],
) -> Result<(), ExitCode> {
    if !policy.adapters.clippy.enabled {
        return Ok(());
    }

    ensure_clippy_adapter_run(report);
    let toolchain = resolved_toolchain
        .clippy_selector(policy.adapters.clippy.toolchain)
        .map_err(toolchain_error_to_exit_code)?;
    let runtime = resolved_toolchain
        .clippy_runtime_label(policy.adapters.clippy.toolchain)
        .map_err(toolchain_error_to_exit_code)?;
    let mut clippy_args = policy.adapters.clippy.args.clone();
    clippy_args.extend(cargo_args.iter().cloned());
    let mut findings = run_clippy(&ws.root, toolchain, &clippy_args).map_err(|err| {
        eprintln!("{err}");
        ExitCode::from(2)
    })?;
    report.findings.append(&mut findings);
    set_clippy_adapter_status(report, runtime);
    Ok(())
}

fn ensure_clippy_adapter_run(report: &mut Report) {
    if report
        .summary
        .adapter_runs
        .iter()
        .any(|run| run.name == "clippy")
    {
        return;
    }
    report.summary.adapter_runs.push(AdapterRun {
        name: "clippy".to_string(),
        enabled: true,
        toolchain: None,
        status: None,
    });
}

fn set_clippy_adapter_status(report: &mut Report, runtime: String) {
    if let Some(adapter_run) = report
        .summary
        .adapter_runs
        .iter_mut()
        .find(|run| run.name == "clippy")
    {
        adapter_run.toolchain = Some(runtime);
        adapter_run.status = Some("ok".to_string());
    }
}

fn toolchain_error_to_exit_code(err: toolchain::ToolchainError) -> ExitCode {
    eprintln!("{err}");
    ExitCode::from(2)
}

fn io_error_to_exit_code(err: ApplyError) -> ExitCode {
    eprintln!("{err}");
    ExitCode::from(2)
}

fn output_error_to_exit_code(err: OutputError) -> ExitCode {
    eprintln!("{err}");
    ExitCode::from(2)
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
        OutputFormat::Text => render_text_report(report),
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
