use crate::analysis::Workspace;
use crate::config::BannedDependenciesConfig;
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub struct BannedDependenciesRule;

impl BannedDependenciesRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "architecture.banned_dependencies",
            family: RuleFamily::Architecture,
            backend: RuleBackend::Syntax,
            summary: "Blocks configured module, crate, type, or function path prefixes.",
            default_level: BannedDependenciesConfig::default().level,
            schema: "level, banned_prefixes",
            config_example: "[rules.\"architecture.banned_dependencies\"]\nlevel = \"deny\"\nbanned_prefixes = [\"std::sync::Mutex\", \"crate::legacy\"]",
            fixable: false,
        }
    }
}

impl Rule for BannedDependenciesRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<BannedDependenciesConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            if !cfg.level.enabled() || cfg.banned_prefixes.is_empty() {
                continue;
            }
            let Some(ast) = &file.ast else { continue };
            let mut visitor = DependencyVisitor {
                file: &file.path,
                banned_prefixes: &cfg.banned_prefixes,
                severity: cfg.level.to_severity(),
                out,
            };
            visitor.visit_file(ast);
        }
    }
}

struct DependencyVisitor<'a> {
    file: &'a std::path::Path,
    banned_prefixes: &'a [String],
    severity: crate::report::Severity,
    out: &'a mut dyn Emitter,
}

impl DependencyVisitor<'_> {
    fn check_path(&mut self, span: proc_macro2::Span, path: &syn::Path) {
        let text = path.to_token_stream().to_string().replace(' ', "");
        if let Some(prefix) = self
            .banned_prefixes
            .iter()
            .find(|prefix| text.starts_with(prefix.as_str()))
        {
            self.out.emit(Finding {
                rule_id: BannedDependenciesRule::static_info().id.to_string(),
                family: Some(BannedDependenciesRule::static_info().family),
                engine: Some(BannedDependenciesRule::static_info().backend),
                severity: self.severity,
                message: format!("banned dependency path: {text}"),
                primary: Some(Span::from_pm_span(self.file, span)),
                secondary: Vec::new(),
                help: Some(format!("Remove or replace dependency on `{prefix}`.")),
                evidence: None,
                confidence: None,
                tags: vec!["dependencies".to_string()],
                fixes: Vec::new(),
            });
        }
    }
}

impl<'ast> Visit<'ast> for DependencyVisitor<'_> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if let Some(path) = use_tree_path(&node.tree) {
            self.check_path(node.span(), &path);
        }
        syn::visit::visit_item_use(self, node);
    }

    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        self.check_path(node.span(), &node.path);
        syn::visit::visit_type_path(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        self.check_path(node.span(), &node.path);
        syn::visit::visit_expr_path(self, node);
    }
}

fn use_tree_path(tree: &syn::UseTree) -> Option<syn::Path> {
    match tree {
        syn::UseTree::Path(path) => {
            let mut segments = syn::punctuated::Punctuated::new();
            segments.push(path.ident.clone().into());
            let mut tail = use_tree_path(&path.tree)?;
            segments.extend(tail.segments);
            tail.segments = segments;
            Some(tail)
        }
        syn::UseTree::Name(name) => Some(syn::Path::from(name.ident.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
