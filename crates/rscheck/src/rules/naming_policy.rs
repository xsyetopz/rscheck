use crate::analysis::Workspace;
use crate::config::NamingPolicyConfig;
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use std::collections::BTreeSet;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub struct NamingPolicyRule;

impl NamingPolicyRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "design.naming_policy",
            family: RuleFamily::Design,
            backend: RuleBackend::Syntax,
            summary: "Flags generic names and banned design suffixes that hide responsibility.",
            default_level: NamingPolicyConfig::default().level,
            schema: "level, banned_names, banned_suffixes",
            config_example: "[rules.\"design.naming_policy\"]\nlevel = \"warn\"\nbanned_names = [\"data\", \"result\"]\nbanned_suffixes = [\"Manager\", \"Helper\"]",
            fixable: false,
        }
    }
}

impl Rule for NamingPolicyRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<NamingPolicyConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            if !cfg.level.enabled() {
                continue;
            }
            let Some(ast) = &file.ast else { continue };
            let banned_names = BTreeSet::from_iter(cfg.banned_names.iter().cloned());
            let mut visitor = NamingVisitor {
                file: &file.path,
                cfg: &cfg,
                banned_names,
                out,
            };
            visitor.visit_file(ast);
        }
    }
}

struct NamingVisitor<'a> {
    file: &'a Path,
    cfg: &'a NamingPolicyConfig,
    banned_names: BTreeSet<String>,
    out: &'a mut dyn Emitter,
}

impl NamingVisitor<'_> {
    fn check_ident(&mut self, ident: &syn::Ident, span: proc_macro2::Span, subject: &str) {
        let name = ident.to_string();
        if self.banned_names.contains(&name) {
            self.emit(span, format!("{subject} name `{name}` is too generic"));
            return;
        }
        if let Some(suffix) = self
            .cfg
            .banned_suffixes
            .iter()
            .find(|suffix| name.ends_with(*suffix))
        {
            self.emit(
                span,
                format!("{subject} name `{name}` uses banned suffix `{suffix}`"),
            );
        }
    }

    fn emit(&mut self, span: proc_macro2::Span, message: String) {
        self.out.emit(
            Finding::from_rule(
                NamingPolicyRule::static_info(),
                self.cfg.level.to_severity(),
                message,
            )
            .with_primary(Span::from_pm_span(self.file, span))
            .with_help(String::from(
                "Use a domain-specific name that states the responsibility.",
            ))
            .with_confidence(String::from("medium"))
            .with_tags(Vec::from([String::from("naming")])),
        );
    }
}

impl<'ast> Visit<'ast> for NamingVisitor<'_> {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.check_ident(&node.ident, node.span(), "struct");
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.check_ident(&node.ident, node.span(), "enum");
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_ident(&node.sig.ident, node.sig.ident.span(), "function");
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let syn::Pat::Ident(binding) = &node.pat {
            self.check_ident(&binding.ident, binding.ident.span(), "binding");
        }
        syn::visit::visit_local(self, node);
    }
}

#[cfg(test)]
mod tests;
