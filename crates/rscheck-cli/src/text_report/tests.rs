use super::render_text_report;
use rscheck::report::{Finding, FindingLabel, FindingLabelKind, Report, Severity};
use rscheck::span::{Location, Span};

#[test]
fn renders_fallback_when_source_missing() {
    let report = Report {
        findings: Vec::from([Finding::new(
            String::from("demo.rule"),
            Severity::Warn,
            String::from("warning text"),
        )
        .with_primary(Span {
            file: "missing.rs".to_string(),
            start: Location { line: 1, column: 1 },
            end: Location { line: 1, column: 3 },
        })]),
        ..Report::default()
    };

    let text = render_text_report(&report);
    assert!(text.contains("warning[demo.rule]: warning text"));
}

#[test]
fn prefers_structured_labels() {
    let report = Report {
        findings: Vec::from([Finding::new(
            String::from("demo.rule"),
            Severity::Warn,
            String::from("warning text"),
        )
        .with_labels(Vec::from([FindingLabel {
            kind: FindingLabelKind::Primary,
            span: Span {
                file: "missing.rs".to_string(),
                start: Location { line: 1, column: 1 },
                end: Location { line: 1, column: 3 },
            },
            message: Some(String::from("label")),
        }]))]),
        ..Report::default()
    };

    let text = render_text_report(&report);
    assert!(text.contains("warning[demo.rule]: warning text"));
}
