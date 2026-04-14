use annotate_snippets::{AnnotationKind, Group, Level, Renderer, Snippet};
use rscheck::fix::line_col_to_byte_offset;
use rscheck::report::{
    Finding, FindingLabel, FindingLabelKind, FindingNoteKind, FixSafety, Report, Severity,
};
use std::collections::BTreeMap;
use std::env::current_dir;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

pub fn render_text_report(report: &Report) -> String {
    TextReportRenderer::new().render(report)
}

struct TextReportRenderer {
    renderer: Renderer,
    cwd: PathBuf,
    source_cache: BTreeMap<String, Option<String>>,
}

impl TextReportRenderer {
    fn new() -> Self {
        Self {
            renderer: Renderer::plain(),
            cwd: current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            source_cache: BTreeMap::new(),
        }
    }

    fn render(&mut self, report: &Report) -> String {
        let mut out = String::new();
        for finding in &report.findings {
            out.push_str(&self.render_finding(finding));
            out.push('\n');
        }

        if let Some(toolchain) = &report.summary.toolchain {
            out.push_str(&format!(
                "toolchain: requested={}, resolved={}, semantic={}, nightly_available={}\n",
                toolchain.requested,
                toolchain.resolved,
                toolchain.semantic,
                toolchain.nightly_available
            ));
            if let Some(reason) = &toolchain.reason {
                out.push_str(&format!("note: {reason}\n"));
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

    fn render_finding(&mut self, finding: &Finding) -> String {
        let labels = normalize_labels(finding);
        let snippet_groups = self.build_snippet_groups(&labels);
        if snippet_groups.is_empty() {
            return fallback_line(finding);
        }

        let mut message = Group::with_title(
            level_for(finding.severity)
                .primary_title(format!("[{}] {}", finding.rule_id, finding.message)),
        );
        for group in &snippet_groups {
            let mut snippet = Snippet::source(&group.source)
                .line_start(1)
                .path(group.display_path.as_str());
            for label in &group.labels {
                let annotation = match label.kind {
                    FindingLabelKind::Primary => AnnotationKind::Primary,
                    FindingLabelKind::Secondary => AnnotationKind::Context,
                };
                snippet = snippet.annotation(
                    annotation
                        .span(label.range.clone())
                        .label(label.message.as_str()),
                );
            }
            message = message.element(snippet);
        }

        let mut out = self.renderer.render(&[message]).to_string();
        append_text_notes(&mut out, finding);
        out
    }

    fn build_snippet_groups(&mut self, labels: &[FindingLabel]) -> Vec<RenderedSnippetGroup> {
        let mut grouped: BTreeMap<String, Vec<FindingLabel>> = BTreeMap::new();
        for label in labels {
            grouped
                .entry(label.span.file.clone())
                .or_default()
                .push(label.clone());
        }

        let mut snippets = Vec::new();
        for (file, file_labels) in grouped {
            let Some(source) = self.read_source(&file).clone() else {
                continue;
            };
            let display_path = display_path(&self.cwd, Path::new(&file));
            let mut rendered_labels = Vec::new();
            for label in file_labels {
                let Ok(byte_start) = line_col_to_byte_offset(
                    &source,
                    label.span.start.line,
                    label.span.start.column,
                ) else {
                    continue;
                };
                let Ok(byte_end) =
                    line_col_to_byte_offset(&source, label.span.end.line, label.span.end.column)
                else {
                    continue;
                };
                rendered_labels.push(RenderedLabel {
                    kind: label.kind,
                    range: byte_start..byte_end.max(byte_start.saturating_add(1)),
                    message: label.message.unwrap_or_default(),
                });
            }

            if rendered_labels.is_empty() {
                continue;
            }

            snippets.push(RenderedSnippetGroup {
                source,
                display_path,
                labels: rendered_labels,
            });
        }

        snippets
    }

    fn read_source(&mut self, file: &str) -> &Option<String> {
        self.source_cache
            .entry(file.to_string())
            .or_insert_with(|| {
                let path = Path::new(file);
                fs::read_to_string(path).ok()
            })
    }
}

#[derive(Clone)]
struct RenderedSnippetGroup {
    source: String,
    display_path: String,
    labels: Vec<RenderedLabel>,
}

#[derive(Clone)]
struct RenderedLabel {
    kind: FindingLabelKind,
    range: Range<usize>,
    message: String,
}

fn normalize_labels(finding: &Finding) -> Vec<FindingLabel> {
    if !finding.labels.is_empty() {
        return finding.labels.clone();
    }

    let mut labels = Vec::new();
    if let Some(primary) = &finding.primary {
        labels.push(FindingLabel {
            kind: FindingLabelKind::Primary,
            span: primary.clone(),
            message: Some(finding.message.clone()),
        });
    }
    for secondary in &finding.secondary {
        labels.push(FindingLabel {
            kind: FindingLabelKind::Secondary,
            span: secondary.clone(),
            message: None,
        });
    }
    labels
}

fn append_text_notes(out: &mut String, finding: &Finding) {
    if let Some(help) = &finding.help {
        out.push_str(&format!("help: {help}\n"));
    }
    if let Some(evidence) = &finding.evidence {
        out.push_str(&format!("note: {evidence}\n"));
    }
    if let Some(confidence) = &finding.confidence {
        out.push_str(&format!("info: confidence={confidence}\n"));
    }
    for note in &finding.notes {
        let prefix = match note.kind {
            FindingNoteKind::Help => "help",
            FindingNoteKind::Note => "note",
            FindingNoteKind::Info => "info",
        };
        out.push_str(&format!("{prefix}: {}\n", note.message));
    }
    for fix in &finding.fixes {
        let safety = match fix.safety {
            FixSafety::Safe => "safe",
            FixSafety::Unsafe => "unsafe",
        };
        out.push_str(&format!("suggestion[{safety}]: {}\n", fix.message));
    }
}

fn fallback_line(finding: &Finding) -> String {
    let severity = match finding.severity {
        Severity::Info => "info",
        Severity::Warn => "warning",
        Severity::Deny => "error",
    };
    match &finding.primary {
        Some(span) => format!(
            "{severity}[{}]: {} at {}:{}:{}\n",
            finding.rule_id, finding.message, span.file, span.start.line, span.start.column
        ),
        None => format!("{severity}[{}]: {}\n", finding.rule_id, finding.message),
    }
}

fn level_for(severity: Severity) -> Level<'static> {
    match severity {
        Severity::Info => Level::INFO,
        Severity::Warn => Level::WARNING,
        Severity::Deny => Level::ERROR,
    }
}

fn display_path(cwd: &Path, path: &Path) -> String {
    path.strip_prefix(cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::render_text_report;
    use rscheck::report::{Finding, FindingLabel, FindingLabelKind, Report, Severity};
    use rscheck::span::{Location, Span};

    #[test]
    fn renders_fallback_when_source_missing() {
        let report = Report {
            findings: vec![Finding {
                rule_id: "demo.rule".to_string(),
                family: None,
                engine: None,
                severity: Severity::Warn,
                message: "warning text".to_string(),
                primary: Some(Span {
                    file: "missing.rs".to_string(),
                    start: Location { line: 1, column: 1 },
                    end: Location { line: 1, column: 3 },
                }),
                secondary: Vec::new(),
                help: None,
                evidence: None,
                confidence: None,
                tags: Vec::new(),
                labels: Vec::new(),
                notes: Vec::new(),
                fixes: Vec::new(),
            }],
            ..Report::default()
        };

        let text = render_text_report(&report);
        assert!(text.contains("warning[demo.rule]: warning text"));
    }

    #[test]
    fn prefers_structured_labels() {
        let report = Report {
            findings: vec![Finding {
                rule_id: "demo.rule".to_string(),
                family: None,
                engine: None,
                severity: Severity::Warn,
                message: "warning text".to_string(),
                primary: None,
                secondary: Vec::new(),
                help: None,
                evidence: None,
                confidence: None,
                tags: Vec::new(),
                labels: vec![FindingLabel {
                    kind: FindingLabelKind::Primary,
                    span: Span {
                        file: "missing.rs".to_string(),
                        start: Location { line: 1, column: 1 },
                        end: Location { line: 1, column: 3 },
                    },
                    message: Some("label".to_string()),
                }],
                notes: Vec::new(),
                fixes: Vec::new(),
            }],
            ..Report::default()
        };

        let text = render_text_report(&report);
        assert!(text.contains("warning[demo.rule]: warning text"));
    }
}
