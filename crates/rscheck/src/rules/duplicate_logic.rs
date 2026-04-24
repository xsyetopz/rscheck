use crate::analysis::Workspace;
use crate::config::DuplicateLogicConfig;
use crate::emit::Emitter;
use crate::report::{Finding, Severity};
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use quote::ToTokens;
use similar::TextDiff;
use std::collections::{HashMap, HashSet, hash_map::DefaultHasher};
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::path::Path;
use syn::spanned::Spanned;

pub struct DuplicateLogicRule;

impl DuplicateLogicRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "shape.duplicate_logic",
            family: RuleFamily::Shape,
            backend: RuleBackend::Syntax,
            summary: "Reports similar function bodies with token fingerprints.",
            default_level: DuplicateLogicConfig::default().level,
            schema: "level, min_tokens, threshold, max_results, exclude_globs, kgram",
            config_example: "[rules.\"shape.duplicate_logic\"]\nlevel = \"warn\"\nmin_tokens = 80\nthreshold = 0.8",
            fixable: false,
        }
    }
}

impl Rule for DuplicateLogicRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        let cfg = match ctx
            .policy
            .decode_rule::<DuplicateLogicConfig>(Self::static_info().id, None)
        {
            Ok(cfg) => cfg,
            Err(_) => return,
        };
        let severity = cfg.level.to_severity();
        let exclude = build_exclude(&cfg.exclude_globs);

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
            let set = fingerprint(&f.norm_tokens, cfg.kgram);
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
            if a.norm_tokens.len() < cfg.min_tokens || b.norm_tokens.len() < cfg.min_tokens {
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
            if sim >= cfg.threshold {
                matches.push((sim, i, j));
            }
        }

        matches.sort_by(|a, b| b.0.total_cmp(&a.0));
        matches.truncate(cfg.max_results);

        for (sim, i, j) in matches {
            let a = &functions[i];
            let b = &functions[j];
            out.emit(duplicate_logic_finding(severity, sim, a, b));
        }
    }
}

fn duplicate_logic_finding(severity: Severity, sim: f32, a: &FnBody, b: &FnBody) -> Finding {
    Finding::from_rule(
        DuplicateLogicRule::static_info(),
        severity,
        format!(
            "duplicate logic: {:.0}% similarity between `{}` and `{}`",
            sim * 100.0,
            a.name,
            b.name
        ),
    )
    .with_primary(Clone::clone(&a.span))
    .with_secondary(Vec::from([Clone::clone(&b.span)]))
    .with_help(String::from("Extract shared code or delete one copy."))
    .with_evidence(side_by_side(&a.source, &b.source))
    .with_tags(Vec::from([String::from("duplication")]))
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
                out.push(fn_body_from_item(path, f));
            }
            syn::Item::Impl(imp) => {
                for it in &imp.items {
                    let syn::ImplItem::Fn(f) = it else { continue };
                    out.push(fn_body_from_impl(path, f));
                }
            }
            _ => {}
        }
    }
}

fn fn_body_from_item(path: &Path, f: &syn::ItemFn) -> FnBody {
    FnBody {
        name: f.sig.ident.to_string(),
        source: f.to_token_stream().to_string(),
        norm_tokens: normalize_tokens(f.block.to_token_stream()),
        span: Span::from_pm_span(path, f.span()),
    }
}

fn fn_body_from_impl(path: &Path, f: &syn::ImplItemFn) -> FnBody {
    FnBody {
        name: f.sig.ident.to_string(),
        source: f.to_token_stream().to_string(),
        norm_tokens: normalize_tokens(f.block.to_token_stream()),
        span: Span::from_pm_span(path, f.span()),
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
                        out.push(String::from(l));
                    }
                    walk(out, g.stream());
                    if !r.is_empty() {
                        out.push(String::from(r));
                    }
                }
                proc_macro2::TokenTree::Ident(_) => out.push(String::from("ID")),
                proc_macro2::TokenTree::Punct(p) => out.push(punct_token(p)),
                proc_macro2::TokenTree::Literal(l) => out.push(classify_literal(&literal_text(&l))),
            }
        }
    }

    let mut out = Vec::new();
    walk(&mut out, ts);
    out
}

fn punct_token(punct: proc_macro2::Punct) -> String {
    punct.as_char().to_string()
}

fn literal_text(literal: &proc_macro2::Literal) -> String {
    literal.to_string()
}

fn classify_literal(s: &str) -> String {
    if s.starts_with('"') || s.starts_with("r\"") || s.starts_with("r#") || s.starts_with("b\"") {
        return String::from("STR");
    }
    if s.chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == '-')
    {
        return String::from("INT");
    }
    String::from("LIT")
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
                left.push(change_text(&change));
                right.push(String::new());
            }
            similar::ChangeTag::Insert => {
                left.push(String::new());
                right.push(change_text(&change));
            }
            similar::ChangeTag::Equal => {
                let text = change_text(&change);
                left.push(Clone::clone(&text));
                right.push(text);
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
        let _ = writeln!(&mut out, "{l:width$} | {r}");
    }
    out
}

fn change_text(change: &similar::Change<&str>) -> String {
    change.to_string()
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
