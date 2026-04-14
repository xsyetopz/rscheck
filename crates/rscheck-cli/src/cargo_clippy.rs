use cargo_metadata::{Message, diagnostic::DiagnosticLevel};
use rscheck::report::{
    Finding, FindingLabel, FindingLabelKind, FindingNote, FindingNoteKind, Fix, FixSafety,
    Severity, TextEdit,
};
use rscheck::rules::RuleBackend;
use rscheck::span::{Location, Span};
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, thiserror::Error)]
pub enum CargoError {
    #[error("failed to spawn cargo")]
    Spawn(#[source] io::Error),
    #[error("failed to read cargo output")]
    Read(#[source] io::Error),
    #[error("cargo exited with non-zero status: {0}")]
    Status(i32),
}

pub fn run_clippy(
    workspace_root: &PathBuf,
    toolchain: Option<&str>,
    extra_args: &[String],
) -> Result<Vec<Finding>, CargoError> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    if let Some(toolchain) = toolchain {
        cmd.arg(toolchain);
    }
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
        .ok_or_else(|| CargoError::Spawn(io::Error::other("cargo stdout missing")))?;

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

    let primary = diag
        .spans
        .iter()
        .find(|s| s.is_primary)
        .map(span_to_report_span);
    let secondary: Vec<_> = diag
        .spans
        .iter()
        .filter(|span| !span.is_primary)
        .map(span_to_report_span)
        .collect();
    let labels = collect_labels(diag);
    let notes = collect_notes(diag);

    let mut fixes = Vec::new();
    for (idx, span) in diag.spans.iter().enumerate() {
        let Some(repl) = &span.suggested_replacement else {
            continue;
        };
        let safety = match span.suggestion_applicability {
            Some(cargo_metadata::diagnostic::Applicability::MachineApplicable) => FixSafety::Safe,
            _ => FixSafety::Unsafe,
        };
        fixes.push(Fix {
            id: format!(
                "{}::clippy_suggestion::{idx}",
                diag.code.as_ref().map_or("clippy", |c| c.code.as_str())
            ),
            safety,
            message: span
                .label
                .clone()
                .unwrap_or_else(|| "apply suggestion".to_string()),
            edits: vec![TextEdit {
                file: span.file_name.clone(),
                byte_start: span.byte_start,
                byte_end: span.byte_end,
                replacement: repl.clone(),
            }],
        });
    }

    Some(Finding {
        rule_id: diag
            .code
            .as_ref()
            .map_or_else(|| "clippy".to_string(), |c| c.code.clone()),
        family: None,
        engine: Some(RuleBackend::Adapter),
        severity,
        message: diag.message.clone(),
        primary,
        secondary,
        help: None,
        evidence: None,
        confidence: None,
        tags: vec!["clippy".to_string()],
        labels,
        notes,
        fixes,
    })
}

fn span_to_report_span(span: &cargo_metadata::diagnostic::DiagnosticSpan) -> Span {
    let file = PathBuf::from(&span.file_name);
    Span::new(
        &file,
        Location {
            line: span.line_start as u32,
            column: span.column_start as u32,
        },
        Location {
            line: span.line_end as u32,
            column: span.column_end as u32,
        },
    )
}

fn collect_labels(diag: &cargo_metadata::diagnostic::Diagnostic) -> Vec<FindingLabel> {
    let mut labels = Vec::new();
    for span in &diag.spans {
        labels.push(FindingLabel {
            kind: if span.is_primary {
                FindingLabelKind::Primary
            } else {
                FindingLabelKind::Secondary
            },
            span: span_to_report_span(span),
            message: span.label.clone(),
        });
    }
    for child in &diag.children {
        for span in &child.spans {
            labels.push(FindingLabel {
                kind: if span.is_primary {
                    FindingLabelKind::Primary
                } else {
                    FindingLabelKind::Secondary
                },
                span: span_to_report_span(span),
                message: span.label.clone().or_else(|| Some(child.message.clone())),
            });
        }
    }
    labels
}

fn collect_notes(diag: &cargo_metadata::diagnostic::Diagnostic) -> Vec<FindingNote> {
    let mut notes = Vec::new();
    for child in &diag.children {
        notes.push(FindingNote {
            kind: map_note_kind(child.level),
            message: child.message.clone(),
        });
    }
    notes
}

fn map_note_kind(level: DiagnosticLevel) -> FindingNoteKind {
    match level {
        DiagnosticLevel::Help => FindingNoteKind::Help,
        DiagnosticLevel::Note | DiagnosticLevel::FailureNote => FindingNoteKind::Note,
        _ => FindingNoteKind::Info,
    }
}
