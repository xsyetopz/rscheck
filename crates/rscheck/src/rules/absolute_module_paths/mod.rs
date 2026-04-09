use crate::analysis::Workspace;
use crate::config::{AbsoluteModulePathsConfig, Config};
use crate::emit::Emitter;
use crate::report::{Finding, Severity};
use crate::rules::{Rule, RuleInfo};
use crate::span::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub struct AbsoluteModulePathsRule {
    cfg: AbsoluteModulePathsConfig,
}

impl AbsoluteModulePathsRule {
    pub fn new(cfg: AbsoluteModulePathsConfig) -> Self {
        Self { cfg }
    }

    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "rscheck::absolute_module_paths",
            summary: "Flags leading-`::` module paths anywhere in the codebase.",
        }
    }
}

impl Rule for AbsoluteModulePathsRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, _config: &Config, out: &mut dyn Emitter) {
        let severity = self.cfg.level.to_severity();
        for file in &ws.files {
            let Some(ast) = &file.ast else { continue };
            let mut v = Visitor {
                file_path: &file.path,
                allow_prefixes: &self.cfg.allow_prefixes,
                severity,
                out,
            };
            v.visit_file(ast);
        }
    }
}

struct Visitor<'a> {
    file_path: &'a std::path::Path,
    allow_prefixes: &'a [String],
    severity: Severity,
    out: &'a mut dyn Emitter,
}

impl Visitor<'_> {
    fn allowed(&self, path_str: &str) -> bool {
        self.allow_prefixes
            .iter()
            .any(|p| !p.is_empty() && path_str.starts_with(p))
    }

    fn emit(&mut self, span: proc_macro2::Span, path_str: String) {
        if self.allowed(&path_str) {
            return;
        }
        self.out.emit(Finding {
            rule_id: AbsoluteModulePathsRule::static_info().id.to_string(),
            severity: self.severity,
            message: format!("absolute module path: {path_str}"),
            primary: Some(Span::from_pm_span(self.file_path, span)),
            secondary: Vec::new(),
            help: Some(
                "Prefer relative paths (drop the leading `::`) unless required.".to_string(),
            ),
            evidence: None,
        });
    }
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if node.leading_colon.is_some() {
            let tree = node.tree.to_token_stream().to_string().replace(' ', "");
            self.emit(node.span(), format!("::{tree}"));
        }
        syn::visit::visit_item_use(self, node);
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        if node.leading_colon.is_some() {
            let path_str = node.to_token_stream().to_string().replace(' ', "");
            self.emit(node.span(), path_str);
        }
        syn::visit::visit_path(self, node);
    }
}

#[cfg(test)]
mod tests;
