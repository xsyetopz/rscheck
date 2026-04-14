use crate::analysis::Workspace;
use crate::config::SrpHeuristicConfig;
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
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
                out.emit(Finding {
                    rule_id: Self::static_info().id.to_string(),
                    family: Some(Self::static_info().family),
                    engine: Some(Self::static_info().backend),
                    severity: cfg.level.to_severity(),
                    message: format!(
                        "impl block has {methods} methods; consider splitting responsibilities"
                    ),
                    primary: Some(Span::from_pm_span(&file.path, imp.span())),
                    secondary: Vec::new(),
                    help: Some(
                        "This is a heuristic; verify SRP boundaries with domain context."
                            .to_string(),
                    ),
                    evidence: None,
                    confidence: None,
                    tags: vec!["design".to_string()],
                    labels: Vec::new(),
                    notes: Vec::new(),
                    fixes: Vec::new(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
