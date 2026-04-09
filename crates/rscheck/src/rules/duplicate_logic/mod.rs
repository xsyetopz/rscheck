use crate::analysis::Workspace;
use crate::config::{Config, DuplicateLogicConfig};
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleInfo};
use crate::span::Span;
use quote::ToTokens;
use similar::TextDiff;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use syn::spanned::Spanned;

pub struct DuplicateLogicRule {
    cfg: DuplicateLogicConfig,
}

impl DuplicateLogicRule {
    pub fn new(cfg: DuplicateLogicConfig) -> Self {
        Self { cfg }
    }

    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "rscheck::duplicate_logic",
            summary: "Finds duplicated logic between function bodies using token fingerprint similarity.",
        }
    }
}

impl Rule for DuplicateLogicRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, _config: &Config, out: &mut dyn Emitter) {
        let severity = self.cfg.level.to_severity();
        let exclude = build_exclude(&self.cfg.exclude_globs);

        let mut functions = Vec::new();
        for file in &ws.files {
            if exclude.is_match(file.path.to_string_lossy().as_ref()) {
                continue;
            }
            let Some(ast) = &file.ast else { continue };
            collect_functions(&file.path, ast, &mut functions);
        }

        let mut fingerprints: Vec<HashSet<u64>> = Vec::with_capacity(functions.len());
        for f in &functions {
            let set = fingerprint(&f.norm_tokens, self.cfg.kgram);
            fingerprints.push(set);
        }

        let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
        for (i, set) in fingerprints.iter().enumerate() {
            for h in set {
                index.entry(*h).or_default().push(i);
            }
        }

        let mut shared: HashMap<(usize, usize), u32> = HashMap::new();
        for (_h, ids) in index {
            if ids.len() < 2 {
                continue;
            }
            for a in 0..ids.len() {
                for b in (a + 1)..ids.len() {
                    let i = ids[a];
                    let j = ids[b];
                    let key = if i < j { (i, j) } else { (j, i) };
                    *shared.entry(key).or_insert(0) += 1;
                }
            }
        }

        let mut matches = Vec::new();
        for ((i, j), _shared) in shared {
            let a = &functions[i];
            let b = &functions[j];
            if a.norm_tokens.len() < self.cfg.min_tokens
                || b.norm_tokens.len() < self.cfg.min_tokens
            {
                continue;
            }
            let sa = &fingerprints[i];
            let sb = &fingerprints[j];
            if sa.is_empty() || sb.is_empty() {
                continue;
            }
            let inter = sa.intersection(sb).count() as f32;
            let union = (sa.len() + sb.len()) as f32 - inter;
            let sim = if union <= 0.0 { 0.0 } else { inter / union };
            if sim >= self.cfg.threshold {
                matches.push((sim, i, j));
            }
        }

        matches.sort_by(|a, b| b.0.total_cmp(&a.0));
        matches.truncate(self.cfg.max_results);

        for (sim, i, j) in matches {
            let a = &functions[i];
            let b = &functions[j];
            let evidence = side_by_side(&a.source, &b.source);
            out.emit(Finding {
                rule_id: Self::static_info().id.to_string(),
                severity,
                message: format!(
                    "duplicate logic: {:.0}% similarity between `{}` and `{}`",
                    sim * 100.0,
                    a.name,
                    b.name
                ),
                primary: Some(a.span.clone()),
                secondary: vec![b.span.clone()],
                help: Some(
                    "Extract a shared helper or refactor to remove duplication.".to_string(),
                ),
                evidence: Some(evidence),
                fixes: Vec::new(),
            });
        }
    }
}

fn build_exclude(globs: &[String]) -> globset::GlobSet {
    let mut b = globset::GlobSetBuilder::new();
    for g in globs {
        if let Ok(glob) = globset::Glob::new(g) {
            b.add(glob);
        }
    }
    b.build()
        .unwrap_or_else(|_| globset::GlobSetBuilder::new().build().unwrap())
}

#[derive(Clone)]
struct FnBody {
    name: String,
    source: String,
    norm_tokens: Vec<String>,
    span: Span,
}

fn collect_functions(path: &Path, ast: &syn::File, out: &mut Vec<FnBody>) {
    for item in &ast.items {
        match item {
            syn::Item::Fn(f) => {
                let name = f.sig.ident.to_string();
                let source = f.to_token_stream().to_string();
                let norm_tokens = normalize_tokens(f.block.to_token_stream());
                let span = Span::from_pm_span(path, f.span());
                out.push(FnBody {
                    name,
                    source,
                    norm_tokens,
                    span,
                });
            }
            syn::Item::Impl(imp) => {
                for it in &imp.items {
                    let syn::ImplItem::Fn(f) = it else { continue };
                    let name = f.sig.ident.to_string();
                    let source = f.to_token_stream().to_string();
                    let norm_tokens = normalize_tokens(f.block.to_token_stream());
                    let span = Span::from_pm_span(path, f.span());
                    out.push(FnBody {
                        name,
                        source,
                        norm_tokens,
                        span,
                    });
                }
            }
            _ => {}
        }
    }
}

fn normalize_tokens(ts: proc_macro2::TokenStream) -> Vec<String> {
    fn walk(out: &mut Vec<String>, stream: proc_macro2::TokenStream) {
        for tt in stream {
            match tt {
                proc_macro2::TokenTree::Group(g) => {
                    let (l, r) = match g.delimiter() {
                        proc_macro2::Delimiter::Parenthesis => ("(", ")"),
                        proc_macro2::Delimiter::Brace => ("{", "}"),
                        proc_macro2::Delimiter::Bracket => ("[", "]"),
                        proc_macro2::Delimiter::None => ("", ""),
                    };
                    if !l.is_empty() {
                        out.push(l.to_string());
                    }
                    walk(out, g.stream());
                    if !r.is_empty() {
                        out.push(r.to_string());
                    }
                }
                proc_macro2::TokenTree::Ident(_) => out.push("ID".to_string()),
                proc_macro2::TokenTree::Punct(p) => out.push(p.as_char().to_string()),
                proc_macro2::TokenTree::Literal(l) => out.push(classify_literal(&l.to_string())),
            }
        }
    }

    let mut out = Vec::new();
    walk(&mut out, ts);
    out
}

fn classify_literal(s: &str) -> String {
    if s.starts_with('"') || s.starts_with("r\"") || s.starts_with("r#") || s.starts_with("b\"") {
        return "STR".to_string();
    }
    if s.chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == '-')
    {
        return "INT".to_string();
    }
    "LIT".to_string()
}

fn fingerprint(tokens: &[String], k: usize) -> HashSet<u64> {
    if k == 0 || tokens.len() < k {
        return HashSet::new();
    }
    let mut out = HashSet::new();
    for win in tokens.windows(k) {
        let mut hasher = DefaultHasher::new();
        for t in win {
            t.hash(&mut hasher);
        }
        out.insert(hasher.finish());
    }
    out
}

fn side_by_side(a: &str, b: &str) -> String {
    let diff = TextDiff::from_lines(a, b);
    let mut left = Vec::new();
    let mut right = Vec::new();
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Delete => {
                left.push(change.to_string());
                right.push(String::new());
            }
            similar::ChangeTag::Insert => {
                left.push(String::new());
                right.push(change.to_string());
            }
            similar::ChangeTag::Equal => {
                left.push(change.to_string());
                right.push(change.to_string());
            }
        }
    }

    let width = left.iter().map(|s| s.len()).max().unwrap_or(0).min(120);
    let mut out = String::new();
    out.push_str("A | B\n");
    out.push_str("---|---\n");
    for (l, r) in left.iter().zip(right.iter()) {
        let l = trim_line(l);
        let r = trim_line(r);
        out.push_str(&format!("{:width$} | {}\n", l, r, width = width));
    }
    out
}

fn trim_line(s: &str) -> String {
    let mut t = s.replace('\t', "    ");
    if t.ends_with('\n') {
        t.pop();
    }
    t
}

#[cfg(test)]
mod tests;
