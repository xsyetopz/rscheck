use crate::analysis::Workspace;
use crate::config::{AbsoluteFilesystemPathsConfig, Config};
use crate::emit::Emitter;
use crate::report::{Finding, Severity};
use crate::rules::{Rule, RuleInfo};
use crate::span::{Location, Span};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::RegexSet;
use std::iter;
use std::path::Path;
use syn::visit::Visit;

pub struct AbsoluteFilesystemPathsRule {
    cfg: AbsoluteFilesystemPathsConfig,
    allow_globs: GlobSet,
    allow_regex: RegexSet,
}

impl AbsoluteFilesystemPathsRule {
    pub fn new(cfg: AbsoluteFilesystemPathsConfig) -> Self {
        Self {
            allow_globs: build_allow_globs(&cfg.allow_globs),
            allow_regex: build_allow_regex(&cfg.allow_regex),
            cfg,
        }
    }

    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "rscheck::absolute_filesystem_paths",
            summary: "Flags absolute filesystem paths inside string literals (Unix/Windows/UNC).",
        }
    }
}

impl Rule for AbsoluteFilesystemPathsRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, _config: &Config, out: &mut dyn Emitter) {
        let severity = self.cfg.level.to_severity();
        for file in &ws.files {
            let Some(ast) = &file.ast else { continue };
            let mut v = Visitor {
                file_path: &file.path,
                allow_globs: &self.allow_globs,
                allow_regex: &self.allow_regex,
                severity,
                out,
            };
            v.visit_file(ast);

            if self.cfg.check_comments {
                scan_line_comments(
                    &file.path,
                    &file.text,
                    &self.allow_globs,
                    &self.allow_regex,
                    severity,
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
            severity: self.severity,
            message: format!("absolute filesystem path ({kind}): {value}"),
            primary: Some(Span::from_pm_span(self.file_path, span)),
            secondary: Vec::new(),
            help: Some(
                "Prefer relative paths or build paths via `PathBuf` at runtime.".to_string(),
            ),
            evidence: None,
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
        // Avoid false positives for common Rust comment/doc markers that show up as string literals
        // (e.g. code that checks `trimmed.starts_with("//!")`).
        if s.starts_with("//") || s.starts_with("/*") {
            return None;
        }
        if s.trim_start_matches('/').is_empty() {
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
            if absolute_kind(part).is_none() {
                continue;
            }
            if allow_globs.is_match(part) || allow_regex.is_match(part) {
                continue;
            }
            out.emit(Finding {
                rule_id: AbsoluteFilesystemPathsRule::static_info().id.to_string(),
                severity,
                message: format!("absolute filesystem path in comment: {part}"),
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
                fixes: Vec::new(),
            });
        }
    }
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
