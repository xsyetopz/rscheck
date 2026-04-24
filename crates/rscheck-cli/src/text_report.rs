use annotate_snippets::{
    AnnotationKind, Group, Level, Patch, Renderer, Snippet, renderer::DecorStyle,
};
use rscheck::fix::line_col_to_byte_offset;
use rscheck::report::{
    Finding, FindingLabel, FindingLabelKind, FindingNoteKind, FixSafety, Report, Severity,
};
use std::collections::BTreeMap;
use std::env::current_dir;
use std::fmt::Write;
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
            renderer: Renderer::plain().decor_style(DecorStyle::Unicode),
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
                let _ = writeln!(&mut out, "- {rule}");
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

        let mut message = Group::with_title(level_for(finding.severity()).primary_title(format!(
            "[{}] {}",
            finding.rule_id(),
            finding.message()
        )));
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
                        .span(label_range(label))
                        .label(label.message.as_str()),
                );
            }
            message = message.element(snippet);
        }

        let mut report_groups = vec![message];
        report_groups.extend(self.build_patch_groups(finding));
        let mut out = self.renderer.render(&report_groups).to_string();
        append_text_notes(&mut out, finding);
        out
    }

    fn build_patch_groups(&mut self, finding: &Finding) -> Vec<Group<'static>> {
        let mut groups = Vec::new();
        for fix in finding.fixes() {
            let title = suggestion_title(finding.severity(), &fix.message);
            let mut group = Group::with_title(title);
            for edit in &fix.edits {
                let Some(source) = self.read_source_owned(&edit.file) else {
                    continue;
                };
                let display_path = display_path(&self.cwd, Path::new(&edit.file));
                let patch = patch_for_edit(edit);
                group = group.element(
                    Snippet::source(source)
                        .line_start(1)
                        .path(display_path)
                        .patch(patch),
                );
            }
            groups.push(group);
        }
        groups
    }

    fn build_snippet_groups(&mut self, labels: &[FindingLabel]) -> Vec<RenderedSnippetGroup> {
        let mut grouped: BTreeMap<String, Vec<FindingLabel>> = BTreeMap::new();
        for label in labels {
            insert_grouped_label(&mut grouped, label);
        }

        let mut snippets = Vec::new();
        for (file, file_labels) in grouped {
            let Some(source) = self.read_source_owned(&file) else {
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

    fn read_source_owned(&mut self, file: &str) -> Option<String> {
        Clone::clone(self.read_source(file))
    }
}

fn label_range(label: &RenderedLabel) -> Range<usize> {
    Clone::clone(&label.range)
}

fn suggestion_title(severity: Severity, message: &str) -> annotate_snippets::Title<'static> {
    level_for(severity).secondary_title(format!("suggestion: {message}"))
}

fn patch_for_edit(edit: &rscheck::report::TextEdit) -> Patch<'static> {
    Patch::new(
        usize::try_from(edit.byte_start).unwrap_or(usize::MAX)
            ..usize::try_from(edit.byte_end).unwrap_or(usize::MAX),
        Clone::clone(&edit.replacement),
    )
}

fn insert_grouped_label(grouped: &mut BTreeMap<String, Vec<FindingLabel>>, label: &FindingLabel) {
    grouped
        .entry(Clone::clone(&label.span.file))
        .or_default()
        .push(Clone::clone(label));
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
    if !finding.labels().is_empty() {
        return Vec::from(finding.labels());
    }

    let mut labels = Vec::new();
    if let Some(primary) = finding.primary() {
        labels.push(primary_label(primary, finding.message()));
    }
    for secondary in finding.secondary() {
        labels.push(secondary_label(secondary));
    }
    labels
}

fn primary_label(primary: &rscheck::span::Span, message: &str) -> FindingLabel {
    FindingLabel {
        kind: FindingLabelKind::Primary,
        span: Clone::clone(primary),
        message: Some(String::from(message)),
    }
}

fn secondary_label(secondary: &rscheck::span::Span) -> FindingLabel {
    FindingLabel {
        kind: FindingLabelKind::Secondary,
        span: Clone::clone(secondary),
        message: None,
    }
}

fn append_text_notes(out: &mut String, finding: &Finding) {
    if let Some(help) = finding.help() {
        out.push_str(&format!("help: {help}\n"));
    }
    if let Some(evidence) = finding.evidence() {
        out.push_str(&format!("note: {evidence}\n"));
    }
    if let Some(confidence) = finding.metadata.confidence.as_deref() {
        out.push_str(&format!("info: confidence={confidence}\n"));
    }
    for note in finding.notes() {
        let prefix = match note.kind {
            FindingNoteKind::Help => "help",
            FindingNoteKind::Note => "note",
            FindingNoteKind::Info => "info",
        };
        let _ = writeln!(out, "{prefix}: {}", note.message);
    }
    for fix in finding.fixes() {
        let safety = match fix.safety {
            FixSafety::Safe => "safe",
            FixSafety::Unsafe => "unsafe",
        };
        let _ = writeln!(out, "suggestion[{safety}]: {}", fix.message);
    }
}

fn fallback_line(finding: &Finding) -> String {
    let severity = match finding.severity() {
        Severity::Info => "info",
        Severity::Warn => "warning",
        Severity::Deny => "error",
    };
    match finding.primary() {
        Some(span) => format!(
            "{severity}[{}]: {} at {}:{}:{}\n",
            finding.rule_id(),
            finding.message(),
            span.file,
            span.start.line,
            span.start.column
        ),
        None => format!("{severity}[{}]: {}\n", finding.rule_id(), finding.message()),
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
mod tests;
