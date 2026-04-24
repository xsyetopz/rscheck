use crate::toolchain::ToolCommand;
use cargo_metadata::{Message, diagnostic::DiagnosticLevel};
use rscheck::report::{
    Finding, FindingLabel, FindingLabelKind, FindingNote, FindingNoteKind, Fix, FixSafety,
    Severity, TextEdit,
};
use rscheck::rules::RuleBackend;
use rscheck::span::{Location, Span};
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::process::Stdio;

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
    cargo_command: &ToolCommand,
    extra_args: &[String],
) -> Result<Vec<Finding>, CargoError> {
    let mut cmd = cargo_command.command();
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
        fixes.push(clippy_fix(diag, span, repl, idx));
    }

    let mut finding = Finding::new(clippy_rule_id(diag), severity, Clone::clone(&diag.message))
        .with_backend(RuleBackend::Adapter)
        .with_secondary(secondary)
        .with_tags(Vec::from([String::from("clippy")]))
        .with_labels(labels)
        .with_notes(notes)
        .with_fixes(fixes);
    if let Some(primary) = primary {
        finding = finding.with_primary(primary);
    }
    Some(finding)
}

fn clippy_rule_id(diag: &cargo_metadata::diagnostic::Diagnostic) -> String {
    diag.code
        .as_ref()
        .map_or_else(|| String::from("clippy"), |c| Clone::clone(&c.code))
}

fn clippy_fix(
    diag: &cargo_metadata::diagnostic::Diagnostic,
    span: &cargo_metadata::diagnostic::DiagnosticSpan,
    replacement: &str,
    idx: usize,
) -> Fix {
    let safety = match span.suggestion_applicability {
        Some(cargo_metadata::diagnostic::Applicability::MachineApplicable) => FixSafety::Safe,
        _ => FixSafety::Unsafe,
    };
    Fix {
        id: format!(
            "{}::clippy_suggestion::{idx}",
            diag.code.as_ref().map_or("clippy", |c| c.code.as_str())
        ),
        safety,
        message: span
            .label
            .as_ref()
            .map_or_else(|| String::from("apply suggestion"), Clone::clone),
        edits: Vec::from([TextEdit {
            file: Clone::clone(&span.file_name),
            byte_start: span.byte_start,
            byte_end: span.byte_end,
            replacement: String::from(replacement),
        }]),
    }
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
        labels.push(label_from_span(span));
    }
    for child in &diag.children {
        for span in &child.spans {
            labels.push(label_from_child(span, child));
        }
    }
    labels
}

fn label_from_span(span: &cargo_metadata::diagnostic::DiagnosticSpan) -> FindingLabel {
    FindingLabel {
        kind: label_kind(span),
        span: span_to_report_span(span),
        message: Clone::clone(&span.label),
    }
}

fn label_from_child(
    span: &cargo_metadata::diagnostic::DiagnosticSpan,
    child: &cargo_metadata::diagnostic::Diagnostic,
) -> FindingLabel {
    FindingLabel {
        kind: label_kind(span),
        span: span_to_report_span(span),
        message: span.label.as_ref().map_or_else(
            || Some(Clone::clone(&child.message)),
            |message| Some(Clone::clone(message)),
        ),
    }
}

fn label_kind(span: &cargo_metadata::diagnostic::DiagnosticSpan) -> FindingLabelKind {
    if span.is_primary {
        FindingLabelKind::Primary
    } else {
        FindingLabelKind::Secondary
    }
}

fn collect_notes(diag: &cargo_metadata::diagnostic::Diagnostic) -> Vec<FindingNote> {
    let mut notes = Vec::new();
    for child in &diag.children {
        notes.push(note_from_child(child));
    }
    notes
}

fn note_from_child(child: &cargo_metadata::diagnostic::Diagnostic) -> FindingNote {
    FindingNote {
        kind: map_note_kind(child.level),
        message: Clone::clone(&child.message),
    }
}

fn map_note_kind(level: DiagnosticLevel) -> FindingNoteKind {
    match level {
        DiagnosticLevel::Help => FindingNoteKind::Help,
        DiagnosticLevel::Note | DiagnosticLevel::FailureNote => FindingNoteKind::Note,
        _ => FindingNoteKind::Info,
    }
}
