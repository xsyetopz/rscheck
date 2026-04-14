use crate::analysis::Workspace;
use crate::config::AbsoluteModulePathsConfig;
use crate::emit::Emitter;
use crate::fix::{find_use_insertion_offset, line_col_to_byte_offset};
use crate::report::{
    Finding, FindingLabel, FindingLabelKind, FindingNote, FindingNoteKind, Fix, FixSafety,
    Severity, TextEdit,
};
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use quote::ToTokens;
use std::collections::HashSet;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub struct AbsoluteModulePathsRule;

impl AbsoluteModulePathsRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "architecture.qualified_module_paths",
            family: RuleFamily::Architecture,
            backend: RuleBackend::Syntax,
            summary: "Flags direct `std::`, `crate::`, and `::` paths in code.",
            default_level: AbsoluteModulePathsConfig::default().level,
            schema: "level, allow_prefixes, roots, allow_crate_root_macros, allow_crate_root_consts, allow_crate_root_fn_calls",
            config_example: "[rules.\"architecture.qualified_module_paths\"]\nlevel = \"deny\"\nroots = [\"std\", \"core\", \"alloc\", \"crate\"]",
            fixable: true,
        }
    }
}

impl Rule for AbsoluteModulePathsRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<AbsoluteModulePathsConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            let Some(ast) = &file.ast else { continue };
            let mut v = Visitor {
                file_path: &file.path,
                file_text: &file.text,
                allow_prefixes: &cfg.allow_prefixes,
                roots: &cfg.roots,
                allow_crate_root_macros: cfg.allow_crate_root_macros,
                allow_crate_root_consts: cfg.allow_crate_root_consts,
                allow_crate_root_fn_calls: cfg.allow_crate_root_fn_calls,
                severity: cfg.level.to_severity(),
                out,
            };
            v.visit_file(ast);
        }
    }
}

struct Visitor<'a> {
    file_path: &'a Path,
    file_text: &'a str,
    allow_prefixes: &'a [String],
    roots: &'a [String],
    allow_crate_root_macros: bool,
    allow_crate_root_consts: bool,
    allow_crate_root_fn_calls: bool,
    severity: Severity,
    out: &'a mut dyn Emitter,
}

impl Visitor<'_> {
    fn allowed(&self, path_str: &str) -> bool {
        self.allow_prefixes
            .iter()
            .any(|p| !p.is_empty() && path_str.starts_with(p))
    }

    fn emit_str(&mut self, span: proc_macro2::Span, path_str: String) {
        if self.allowed(&path_str) {
            return;
        }
        let fixes = Vec::new();
        self.out.emit(Finding {
            rule_id: AbsoluteModulePathsRule::static_info().id.to_string(),
            family: Some(AbsoluteModulePathsRule::static_info().family),
            engine: Some(AbsoluteModulePathsRule::static_info().backend),
            severity: self.severity,
            message: format!("qualified module path: {path_str}"),
            primary: Some(Span::from_pm_span(self.file_path, span)),
            secondary: Vec::new(),
            help: Some("Import the item and use the local name.".to_string()),
            evidence: None,
            confidence: None,
            tags: vec!["imports".to_string(), "style".to_string()],
            labels: vec![FindingLabel {
                kind: FindingLabelKind::Primary,
                span: Span::from_pm_span(self.file_path, span),
                message: Some("qualified path used here".to_string()),
            }],
            notes: vec![FindingNote {
                kind: FindingNoteKind::Help,
                message: "Import the item and use the local name.".to_string(),
            }],
            fixes,
        });
    }

    fn emit_path(&mut self, span: proc_macro2::Span, path: &syn::Path) {
        let path_str = path_to_string(path);
        if should_flag_path(&path_str, self.roots) {
            if self.allowed(&path_str) {
                return;
            }
            let fixes = self.build_fixes(span, path);
            self.out.emit(Finding {
                rule_id: AbsoluteModulePathsRule::static_info().id.to_string(),
                family: Some(AbsoluteModulePathsRule::static_info().family),
                engine: Some(AbsoluteModulePathsRule::static_info().backend),
                severity: self.severity,
                message: format!("qualified module path: {path_str}"),
                primary: Some(Span::from_pm_span(self.file_path, span)),
                secondary: Vec::new(),
                help: Some("Import the item and use the local name.".to_string()),
                evidence: None,
                confidence: None,
                tags: vec!["imports".to_string(), "style".to_string()],
                labels: vec![FindingLabel {
                    kind: FindingLabelKind::Primary,
                    span: Span::from_pm_span(self.file_path, span),
                    message: Some("qualified path used here".to_string()),
                }],
                notes: vec![FindingNote {
                    kind: FindingNoteKind::Help,
                    message: "Import the item and use the local name.".to_string(),
                }],
                fixes,
            });
        }
    }

    fn build_fixes(&self, span: proc_macro2::Span, path: &syn::Path) -> Vec<Fix> {
        if path.leading_colon.is_some() {
            return Vec::new();
        }

        let (import_path, replacement, imported_name) = match compute_import_and_replacement(path) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut safety = FixSafety::Safe;
        if name_conflicts(self.file_text, imported_name.as_deref()) {
            safety = FixSafety::Unsafe;
        }

        let start = span.start();
        let end = span.end();
        let byte_start = match line_col_to_byte_offset(
            self.file_text,
            start.line as u32,
            (start.column as u32).saturating_add(1),
        ) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let byte_end = match line_col_to_byte_offset(
            self.file_text,
            end.line as u32,
            (end.column as u32).saturating_add(1),
        ) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let mut edits = Vec::new();
        edits.push(TextEdit {
            file: self.file_path.to_string_lossy().to_string(),
            byte_start: byte_start as u32,
            byte_end: byte_end as u32,
            replacement: replacement.to_string(),
        });

        if !self.file_text.contains(&format!("use {import_path};")) {
            let insert_at = find_use_insertion_offset(self.file_text);
            edits.push(TextEdit {
                file: self.file_path.to_string_lossy().to_string(),
                byte_start: insert_at as u32,
                byte_end: insert_at as u32,
                replacement: format!("use {import_path};\n"),
            });
        }

        vec![Fix {
            id: format!("{}::import", AbsoluteModulePathsRule::static_info().id),
            safety,
            message: format!("Import `{import_path}` and use `{replacement}`."),
            edits,
        }]
    }
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if node.leading_colon.is_some() {
            if let Some(path_str) = use_tree_path_str(true, &node.tree) {
                self.emit_str(node.span(), path_str);
            }
        }
    }

    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        self.emit_path(node.span(), &node.path);
        syn::visit::visit_type_path(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        let path_str = path_to_string(&node.path);
        if should_flag_path(&path_str, self.roots)
            && !is_allowed_crate_root_const(&path_str, self.allow_crate_root_consts)
        {
            self.emit_path(node.span(), &node.path);
        }
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_pat(&mut self, node: &'ast syn::Pat) {
        if let syn::Pat::Path(p) = node {
            let path_str = path_to_string(&p.path);
            if should_flag_path(&path_str, self.roots)
                && !is_allowed_crate_root_const(&path_str, self.allow_crate_root_consts)
            {
                self.emit_path(p.span(), &p.path);
            }
        }
        syn::visit::visit_pat(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if self.allow_crate_root_fn_calls {
            if let syn::Expr::Path(func) = node.func.as_ref() {
                let path_str = path_to_string(&func.path);
                if is_allowed_crate_root_call(&path_str) {
                    for arg in &node.args {
                        self.visit_expr(arg);
                    }
                    return;
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let path_str = path_to_string(&node.path);
        if should_flag_path(&path_str, self.roots)
            && !(self.allow_crate_root_macros && is_allowed_crate_root_macro(&path_str))
        {
            self.emit_str(node.span(), path_str);
        }
        syn::visit::visit_macro(self, node);
    }
}

#[cfg(test)]
mod tests;

fn should_flag_path(path_str: &str, roots: &[String]) -> bool {
    if path_str.starts_with("::") {
        return true;
    }

    let first = path_str.split("::").next().unwrap_or("");
    if first.is_empty() {
        return false;
    }
    if !roots.iter().any(|r| r == first) {
        return false;
    }

    path_str.contains("::")
}

fn use_tree_path_str(leading_colon: bool, tree: &syn::UseTree) -> Option<String> {
    fn flatten(tree: &syn::UseTree, out: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(p) => {
                out.push(p.ident.to_string());
                flatten(&p.tree, out);
            }
            syn::UseTree::Name(n) => out.push(n.ident.to_string()),
            syn::UseTree::Rename(r) => out.push(r.ident.to_string()),
            syn::UseTree::Glob(_) => out.push("*".to_string()),
            syn::UseTree::Group(g) => {
                if g.items.len() == 1 {
                    flatten(&g.items[0], out);
                } else {
                    out.push("{...}".to_string());
                }
            }
        }
    }

    let mut parts = Vec::new();
    flatten(tree, &mut parts);
    if parts.is_empty() {
        return None;
    }

    let mut s = parts.join("::");
    if leading_colon {
        s = format!("::{s}");
    }
    Some(s)
}

fn path_to_string(path: &syn::Path) -> String {
    path.to_token_stream().to_string().replace(' ', "")
}

fn is_allowed_crate_root_call(path_str: &str) -> bool {
    is_two_segment_crate_root(path_str)
}

fn is_allowed_crate_root_macro(path_str: &str) -> bool {
    is_two_segment_crate_root(path_str)
}

fn is_allowed_crate_root_const(path_str: &str, enabled: bool) -> bool {
    if !enabled {
        return false;
    }
    let Some(ident) = two_segment_crate_root_ident(path_str) else {
        return false;
    };
    is_screaming_snake(&ident)
}

fn is_two_segment_crate_root(path_str: &str) -> bool {
    two_segment_crate_root_ident(path_str).is_some()
}

fn two_segment_crate_root_ident(path_str: &str) -> Option<String> {
    if path_str.starts_with("::") {
        return None;
    }
    let mut parts = path_str.split("::");
    let first = parts.next()?;
    let second = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if first != "crate" {
        return None;
    }
    if second.is_empty() {
        return None;
    }
    Some(second.to_string())
}

fn is_screaming_snake(ident: &str) -> bool {
    let mut chars = ident.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    for c in chars {
        if !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
            return false;
        }
    }
    true
}

fn compute_import_and_replacement(path: &syn::Path) -> Option<(String, String, Option<String>)> {
    if path.leading_colon.is_some() {
        return None;
    }

    let segs: Vec<&syn::PathSegment> = path.segments.iter().collect();
    if segs.len() < 2 {
        return None;
    }

    let idents: Vec<String> = segs.iter().map(|s| s.ident.to_string()).collect();
    if idents[0].is_empty() {
        return None;
    }

    let last_ident = idents.last()?;
    let penult_ident = &idents[idents.len() - 2];
    let last_is_type = last_ident
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase());
    let penult_is_type = penult_ident
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase());

    let last_tokens = segs[segs.len() - 1]
        .to_token_stream()
        .to_string()
        .replace(' ', "");
    let penult_tokens = segs[segs.len() - 2]
        .to_token_stream()
        .to_string()
        .replace(' ', "");

    if segs.len() == 2 {
        let import_path = idents.join("::");
        let replacement = last_tokens;
        return Some((import_path, replacement, Some(last_ident.to_string())));
    }

    if last_is_type {
        let import_path = idents.join("::");
        let replacement = last_tokens;
        return Some((import_path, replacement, Some(last_ident.to_string())));
    }

    if penult_is_type {
        let import_path = idents[..idents.len() - 1].join("::");
        let replacement = format!("{penult_tokens}::{last_tokens}");
        return Some((import_path, replacement, Some(penult_ident.to_string())));
    }

    // Value/function under a module: import the item directly and use the local name.
    let import_path = idents.join("::");
    let replacement = last_tokens;
    Some((import_path, replacement, Some(last_ident.to_string())))
}

fn name_conflicts(file_text: &str, name: Option<&str>) -> bool {
    let Some(name) = name else { return false };
    let Ok(ast) = syn::parse_file(file_text) else {
        return false;
    };

    let mut collector = TakenNameCollector {
        taken: HashSet::new(),
    };
    collector.visit_file(&ast);
    let taken = collector.taken;
    taken.contains(name)
}

struct TakenNameCollector {
    taken: HashSet<String>,
}

impl TakenNameCollector {
    fn insert_ident(&mut self, ident: &syn::Ident) {
        self.taken.insert(ident.to_string());
    }
}

impl<'ast> Visit<'ast> for TakenNameCollector {
    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.insert_ident(&node.ident);
        syn::visit::visit_item_const(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.insert_ident(&node.ident);
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.insert_ident(&node.sig.ident);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.insert_ident(&node.ident);
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.insert_ident(&node.ident);
        syn::visit::visit_item_static(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.insert_ident(&node.ident);
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.insert_ident(&node.ident);
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        self.insert_ident(&node.ident);
        syn::visit::visit_item_type(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        collect_use_names(&node.tree, &mut self.taken);
        syn::visit::visit_item_use(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        collect_pat_names(&node.pat, &mut self.taken);
        syn::visit::visit_local(self, node);
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.insert_ident(&node.ident);
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_generic_param(&mut self, node: &'ast syn::GenericParam) {
        match node {
            syn::GenericParam::Type(param) => self.insert_ident(&param.ident),
            syn::GenericParam::Const(param) => self.insert_ident(&param.ident),
            syn::GenericParam::Lifetime(_) => {}
        }
        syn::visit::visit_generic_param(self, node);
    }
}

fn collect_use_names(tree: &syn::UseTree, out: &mut HashSet<String>) {
    match tree {
        syn::UseTree::Path(p) => {
            collect_use_names(&p.tree, out);
        }
        syn::UseTree::Name(n) => {
            out.insert(n.ident.to_string());
        }
        syn::UseTree::Rename(r) => {
            out.insert(r.rename.to_string());
        }
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(g) => {
            for it in &g.items {
                collect_use_names(it, out);
            }
        }
    }
}

fn collect_pat_names(pat: &syn::Pat, out: &mut HashSet<String>) {
    match pat {
        syn::Pat::Ident(ident) => {
            out.insert(ident.ident.to_string());
        }
        syn::Pat::Or(or_pat) => {
            for case in &or_pat.cases {
                collect_pat_names(case, out);
            }
        }
        syn::Pat::Paren(paren) => collect_pat_names(&paren.pat, out),
        syn::Pat::Reference(reference) => collect_pat_names(&reference.pat, out),
        syn::Pat::Slice(slice) => {
            for elem in &slice.elems {
                collect_pat_names(elem, out);
            }
        }
        syn::Pat::Struct(struct_pat) => {
            for field in &struct_pat.fields {
                collect_pat_names(&field.pat, out);
            }
        }
        syn::Pat::Tuple(tuple) => {
            for elem in &tuple.elems {
                collect_pat_names(elem, out);
            }
        }
        syn::Pat::TupleStruct(tuple) => {
            for elem in &tuple.elems {
                collect_pat_names(elem, out);
            }
        }
        syn::Pat::Type(typed) => collect_pat_names(&typed.pat, out),
        _ => {}
    }
}
