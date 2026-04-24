use crate::analysis::Workspace;
use crate::config::SrpHeuristicConfig;
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use std::path::Path;
use syn::spanned::Spanned;

pub struct SrpHeuristicRule;

impl SrpHeuristicRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "shape.responsibility_split",
            family: RuleFamily::Shape,
            backend: RuleBackend::Syntax,
            summary: "Flags impl blocks with many methods as a responsibility-split heuristic.",
            default_level: SrpHeuristicConfig::default().level,
            schema: "level, method_count_threshold",
            config_example: "[rules.\"shape.responsibility_split\"]\nlevel = \"warn\"\nmethod_count_threshold = 25",
            fixable: false,
        }
    }
}

impl Rule for SrpHeuristicRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<SrpHeuristicConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            let Some(ast) = &file.ast else { continue };
            for item in &ast.items {
                let syn::Item::Impl(imp) = item else { continue };
                let methods = imp
                    .items
                    .iter()
                    .filter(|i| matches!(i, syn::ImplItem::Fn(_)))
                    .count();
                if methods <= cfg.method_count_threshold {
                    continue;
                }
                emit_srp_finding(out, &cfg, &file.path, imp.span(), methods);
            }
        }
    }
}

fn emit_srp_finding(
    out: &mut dyn Emitter,
    cfg: &SrpHeuristicConfig,
    file_path: &Path,
    span: proc_macro2::Span,
    methods: usize,
) {
    out.emit(
        Finding::from_rule(
            SrpHeuristicRule::static_info(),
            cfg.level.to_severity(),
            format!("impl block has {methods} methods; consider splitting responsibilities"),
        )
        .with_primary(Span::from_pm_span(file_path, span))
        .with_help(String::from(
            "This is a heuristic; verify SRP boundaries with domain context.",
        ))
        .with_tags(Vec::from([String::from("design")])),
    );
}

#[cfg(test)]
mod tests;
