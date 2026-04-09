use cargo_metadata::{Message, diagnostic::DiagnosticLevel};
use rscheck::report::{Finding, Severity};
use rscheck::span::{Location, Span};
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, thiserror::Error)]
pub enum CargoError {
    #[error("failed to spawn cargo")]
    Spawn(#[source] std::io::Error),
    #[error("failed to read cargo output")]
    Read(#[source] std::io::Error),
    #[error("cargo exited with non-zero status: {0}")]
    Status(i32),
}

pub fn run_clippy(
    workspace_root: &PathBuf,
    extra_args: &[String],
) -> Result<Vec<Finding>, CargoError> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.arg("clippy")
        .arg("--workspace")
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    for arg in extra_args {
        cmd.arg(arg);
    }

    let mut child = cmd.spawn().map_err(CargoError::Spawn)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CargoError::Spawn(std::io::Error::other("cargo stdout missing")))?;

    let reader = BufReader::new(stdout);
    let mut findings = Vec::new();

    for message in Message::parse_stream(reader) {
        let message = message.map_err(CargoError::Read)?;
        if let Message::CompilerMessage(msg) = message {
            if let Some(finding) = diagnostic_to_finding(&msg.message) {
                findings.push(finding);
            }
        }
    }

    let status = child.wait().map_err(CargoError::Read)?;
    if !status.success() {
        let code = status.code().unwrap_or(1);
        return Err(CargoError::Status(code));
    }

    Ok(findings)
}

fn diagnostic_to_finding(diag: &cargo_metadata::diagnostic::Diagnostic) -> Option<Finding> {
    let severity = match diag.level {
        DiagnosticLevel::Error => Severity::Deny,
        DiagnosticLevel::Warning => Severity::Warn,
        DiagnosticLevel::Note | DiagnosticLevel::Help => Severity::Info,
        DiagnosticLevel::FailureNote => Severity::Warn,
        DiagnosticLevel::Ice => Severity::Deny,
        _ => Severity::Warn,
    };

    let primary = diag.spans.iter().find(|s| s.is_primary).map(|s| {
        let file = PathBuf::from(&s.file_name);
        Span::new(
            &file,
            Location {
                line: s.line_start as u32,
                column: s.column_start as u32,
            },
            Location {
                line: s.line_end as u32,
                column: s.column_end as u32,
            },
        )
    });

    Some(Finding {
        rule_id: diag
            .code
            .as_ref()
            .map_or_else(|| "clippy".to_string(), |c| c.code.clone()),
        severity,
        message: diag.message.clone(),
        primary,
        secondary: Vec::new(),
        help: None,
        evidence: None,
    })
}
