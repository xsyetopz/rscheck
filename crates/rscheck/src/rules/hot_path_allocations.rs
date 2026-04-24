use crate::analysis::Workspace;
use crate::config::HotPathAllocationsConfig;
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub struct HotPathAllocationsRule;

impl HotPathAllocationsRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "perf.hot_path_allocations",
            family: RuleFamily::Performance,
            backend: RuleBackend::Syntax,
            summary: "Flags common allocation-producing calls and macros inside loops.",
            default_level: HotPathAllocationsConfig::default().level,
            schema: "level, methods, macros",
            config_example: "[rules.\"perf.hot_path_allocations\"]\nlevel = \"warn\"\nmethods = [\"clone\", \"to_string\", \"collect\"]",
            fixable: false,
        }
    }
}

impl Rule for HotPathAllocationsRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<HotPathAllocationsConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            if !cfg.level.enabled() {
                continue;
            }
            let Some(ast) = &file.ast else { continue };
            let mut visitor = HotPathVisitor {
                file: &file.path,
                cfg: &cfg,
                loop_depth: 0,
                out,
            };
            visitor.visit_file(ast);
        }
    }
}

struct HotPathVisitor<'a> {
    file: &'a Path,
    cfg: &'a HotPathAllocationsConfig,
    loop_depth: usize,
    out: &'a mut dyn Emitter,
}

impl HotPathVisitor<'_> {
    fn in_loop(&self) -> bool {
        self.loop_depth > 0
    }

    fn emit(&mut self, span: proc_macro2::Span, operation: String) {
        self.out.emit(
            Finding::from_rule(
                HotPathAllocationsRule::static_info(),
                self.cfg.level.to_severity(),
                format!("allocation-like operation `{operation}` inside loop"),
            )
            .with_primary(Span::from_pm_span(self.file, span))
            .with_help(String::from(
                "Move allocation out of the loop or reuse storage when profiling confirms this path matters.",
            ))
            .with_confidence(String::from("medium"))
            .with_tags(Vec::from([
                String::from("performance"),
                String::from("allocation"),
            ])),
        );
    }
}

impl<'ast> Visit<'ast> for HotPathVisitor<'_> {
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.loop_depth += 1;
        syn::visit::visit_expr_for_loop(self, node);
        self.loop_depth = self.loop_depth.saturating_sub(1);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.loop_depth += 1;
        syn::visit::visit_expr_loop(self, node);
        self.loop_depth = self.loop_depth.saturating_sub(1);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.loop_depth += 1;
        syn::visit::visit_expr_while(self, node);
        self.loop_depth = self.loop_depth.saturating_sub(1);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.in_loop() {
            let method = node.method.to_string();
            if self
                .cfg
                .methods
                .iter()
                .any(|candidate| candidate == &method)
            {
                self.emit(node.method.span(), format!(".{method}()"));
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if self.in_loop() {
            let name = node
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string());
            if let Some(name) =
                name.filter(|name| self.cfg.macros.iter().any(|candidate| candidate == name))
            {
                self.emit(node.path.span(), format!("{name}!"));
            }
        }
        syn::visit::visit_macro(self, node);
    }
}

#[cfg(test)]
mod tests;
