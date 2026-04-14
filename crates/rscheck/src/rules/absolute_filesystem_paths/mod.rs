use crate::analysis::Workspace;
use crate::config::AbsoluteFilesystemPathsConfig;
use crate::emit::Emitter;
use crate::report::{
    Finding, FindingLabel, FindingLabelKind, FindingNote, FindingNoteKind, Severity,
};
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::{Location, Span};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::RegexSet;
use std::iter;
use std::path::Path;
use syn::visit::Visit;

pub struct AbsoluteFilesystemPathsRule;

impl AbsoluteFilesystemPathsRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "portability.absolute_literal_paths",
            family: RuleFamily::Portability,
            backend: RuleBackend::Syntax,
            summary: "Flags absolute filesystem paths inside string literals (Unix/Windows/UNC).",
            default_level: AbsoluteFilesystemPathsConfig::default().level,
            schema: "level, allow_globs, allow_regex, check_comments",
            config_example: "[rules.\"portability.absolute_literal_paths\"]\nlevel = \"warn\"\ncheck_comments = false",
            fixable: false,
        }
    }
}

impl Rule for AbsoluteFilesystemPathsRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx.policy.decode_rule::<AbsoluteFilesystemPathsConfig>(
                Self::static_info().id,
                Some(&file.path),
            ) {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            let Some(ast) = &file.ast else { continue };
            let allow_globs = build_allow_globs(&cfg.allow_globs);
            let allow_regex = build_allow_regex(&cfg.allow_regex);
            let mut v = Visitor {
                file_path: &file.path,
                allow_globs: &allow_globs,
                allow_regex: &allow_regex,
                severity: cfg.level.to_severity(),
                out,
            };
            v.visit_file(ast);

            if cfg.check_comments {
                scan_line_comments(
                    &file.path,
                    &file.text,
                    &allow_globs,
                    &allow_regex,
                    cfg.level.to_severity(),
                    out,
                );
            }
        }
    }
}

struct Visitor<'a> {
    file_path: &'a Path,
    allow_globs: &'a GlobSet,
    allow_regex: &'a RegexSet,
    severity: Severity,
    out: &'a mut dyn Emitter,
}

impl Visitor<'_> {
    fn allowed(&self, value: &str) -> bool {
        self.allow_globs.is_match(value) || self.allow_regex.is_match(value)
    }

    fn check_str(&mut self, span: proc_macro2::Span, value: &str) {
        let Some(kind) = absolute_kind(value) else {
            return;
        };
        if self.allowed(value) {
            return;
        }
        self.out.emit(Finding {
            rule_id: AbsoluteFilesystemPathsRule::static_info().id.to_string(),
            family: Some(AbsoluteFilesystemPathsRule::static_info().family),
            engine: Some(AbsoluteFilesystemPathsRule::static_info().backend),
            severity: self.severity,
            message: format!("absolute filesystem path ({kind}): {value}"),
            primary: Some(Span::from_pm_span(self.file_path, span)),
            secondary: Vec::new(),
            help: Some(
                "Prefer relative paths or build paths via `PathBuf` at runtime.".to_string(),
            ),
            evidence: None,
            confidence: None,
            tags: vec!["paths".to_string()],
            labels: vec![FindingLabel {
                kind: FindingLabelKind::Primary,
                span: Span::from_pm_span(self.file_path, span),
                message: Some(format!("{kind} path literal")),
            }],
            notes: vec![FindingNote {
                kind: FindingNoteKind::Help,
                message: "Prefer relative paths or build paths via `PathBuf` at runtime."
                    .to_string(),
            }],
            fixes: Vec::new(),
        });
    }
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        self.check_str(node.span(), &node.value());
        syn::visit::visit_lit_str(self, node);
    }
}

fn absolute_kind(s: &str) -> Option<&'static str> {
    if s.starts_with('/') {
        if is_non_filesystem_unix_literal(s) {
            return None;
        }
        return Some("unix");
    }
    if s.len() >= 3 {
        let bytes = s.as_bytes();
        if bytes[1] == b':'
            && (bytes[2] == b'\\' || bytes[2] == b'/')
            && bytes[0].is_ascii_alphabetic()
        {
            return Some("windows-drive");
        }
    }
    if s.starts_with(r"\\") {
        let rest = s.trim_start_matches('\\');
        let segments = rest.split('\\').filter(|p| !p.is_empty()).take(2).count();
        if segments >= 2 {
            return Some("unc");
        }
        return None;
    }
    None
}

fn is_non_filesystem_unix_literal(value: &str) -> bool {
    if value.starts_with("//") || value.starts_with("/*") {
        return true;
    }
    if value.trim_start_matches('/').is_empty() {
        return true;
    }
    if value.contains("://")
        || value.starts_with("/api/")
        || value.starts_with("/graphql")
        || value.starts_with("/oauth/")
        || value.starts_with("/v1/")
        || value.starts_with("/v2/")
        || value.starts_with("/:")
    {
        return true;
    }
    if value.contains("/{")
        || value.contains("/:")
        || value.contains('?')
        || value.contains('#')
        || value.starts_with("^/")
        || value.ends_with("/$")
    {
        return true;
    }

    false
}

fn scan_line_comments(
    file_path: &Path,
    text: &str,
    allow_globs: &GlobSet,
    allow_regex: &RegexSet,
    severity: Severity,
    out: &mut dyn Emitter,
) {
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//") {
            continue;
        }
        for part in trimmed.split_whitespace() {
            let candidate = trim_comment_punctuation(part);
            if absolute_kind(candidate).is_none() {
                continue;
            }
            if allow_globs.is_match(candidate) || allow_regex.is_match(candidate) {
                continue;
            }
            out.emit(Finding {
                rule_id: AbsoluteFilesystemPathsRule::static_info().id.to_string(),
                family: Some(AbsoluteFilesystemPathsRule::static_info().family),
                engine: Some(AbsoluteFilesystemPathsRule::static_info().backend),
                severity,
                message: format!("absolute filesystem path in comment: {candidate}"),
                primary: Some(Span::new(
                    file_path,
                    Location {
                        line: (idx as u32).saturating_add(1),
                        column: 1,
                    },
                    Location {
                        line: (idx as u32).saturating_add(1),
                        column: 1,
                    },
                )),
                secondary: Vec::new(),
                help: None,
                evidence: None,
                confidence: None,
                tags: vec!["paths".to_string(), "comments".to_string()],
                labels: Vec::new(),
                notes: Vec::new(),
                fixes: Vec::new(),
            });
        }
    }
}

fn trim_comment_punctuation(part: &str) -> &str {
    part.trim_matches(|ch: char| matches!(ch, '`' | '"' | '\'' | ',' | '.' | ';' | ')' | '('))
        .trim_end_matches(':')
}

fn build_allow_globs(patterns: &[String]) -> GlobSet {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(glob) = Glob::new(p) {
            b.add(glob);
        }
    }
    b.build()
        .unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap())
}

fn build_allow_regex(patterns: &[String]) -> RegexSet {
    RegexSet::new(patterns.iter().map(String::as_str))
        .unwrap_or_else(|_| RegexSet::new(iter::empty::<&'static str>()).unwrap())
}

#[cfg(test)]
mod tests;
