use crate::analysis::Workspace;
use crate::config::{CustomPattern, CustomPatternConfig};
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::{Location, Span};
use globset::{Glob, GlobSetBuilder};
use regex::Regex;
use std::path::Path;

pub struct CustomPatternRule;

impl CustomPatternRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "pattern.custom",
            family: RuleFamily::Design,
            backend: RuleBackend::Syntax,
            summary: "Runs configured text patterns with rscheck diagnostics and scopes.",
            default_level: CustomPatternConfig::default().level,
            schema: "level, patterns = [{ name, regex, message, include, exclude }]",
            config_example: "[rules.\"pattern.custom\"]\nlevel = \"warn\"\npatterns = [{ name = \"todo\", regex = \"TODO\", message = \"remove TODO\" }]",
            fixable: false,
        }
    }
}

impl Rule for CustomPatternRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<CustomPatternConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            if !cfg.level.enabled() {
                continue;
            }
            for pattern in &cfg.patterns {
                if !pattern_matches_file(pattern, &ws.root, &file.path) {
                    continue;
                }
                let Ok(regex) = Regex::new(&pattern.regex) else {
                    continue;
                };
                for hit in regex.find_iter(&file.text) {
                    let span = byte_range_to_span(&file.path, &file.text, hit.start(), hit.end());
                    emit_custom_pattern_finding(out, &cfg, pattern, span, hit.as_str());
                }
            }
        }
    }
}

fn emit_custom_pattern_finding(
    out: &mut dyn Emitter,
    cfg: &CustomPatternConfig,
    pattern: &CustomPattern,
    span: Span,
    matched_text: &str,
) {
    let message = pattern.message.as_ref().map_or_else(
        || format!("custom pattern `{}` matched", pattern.name),
        Clone::clone,
    );
    out.emit(
        Finding::new(
            format!("{}::{}", CustomPatternRule::static_info().id, pattern.name),
            cfg.level.to_severity(),
            message,
        )
        .with_engine(
            CustomPatternRule::static_info().family,
            CustomPatternRule::static_info().backend,
        )
        .with_primary(span)
        .with_evidence(String::from(matched_text))
        .with_confidence(String::from("high"))
        .with_tags(Vec::from([String::from("custom-pattern")])),
    );
}

fn pattern_matches_file(pattern: &CustomPattern, root: &Path, file: &Path) -> bool {
    let rel = file.strip_prefix(root).unwrap_or(file).to_string_lossy();
    if !pattern.include.is_empty()
        && !globset(&pattern.include).is_some_and(|set| set.is_match(rel.as_ref()))
    {
        return false;
    }
    !globset(&pattern.exclude).is_some_and(|set| set.is_match(rel.as_ref()))
}

fn globset(patterns: &[String]) -> Option<globset::GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern).ok()?);
    }
    builder.build().ok()
}

fn byte_range_to_span(file: &Path, text: &str, start: usize, end: usize) -> Span {
    let start_location = byte_to_location(text, start);
    let end_location = byte_to_location(text, end.max(start.saturating_add(1)));
    Span::new(file, start_location, end_location)
}

fn byte_to_location(text: &str, byte_index: usize) -> Location {
    let mut line = 1_u32;
    let mut column = 1_u32;
    for (idx, ch) in text.char_indices() {
        if idx >= byte_index {
            break;
        }
        if ch == '\n' {
            line = line.saturating_add(1);
            column = 1;
        } else {
            column = column.saturating_add(1);
        }
    }
    Location { line, column }
}

#[cfg(test)]
mod tests;
