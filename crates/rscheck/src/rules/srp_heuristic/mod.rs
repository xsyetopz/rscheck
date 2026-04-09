use crate::analysis::Workspace;
use crate::config::{Config, SrpHeuristicConfig};
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleInfo};
use crate::span::Span;
use syn::spanned::Spanned;

pub struct SrpHeuristicRule {
    cfg: SrpHeuristicConfig,
}

impl SrpHeuristicRule {
    pub fn new(cfg: SrpHeuristicConfig) -> Self {
        Self { cfg }
    }

    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "rscheck::srp_heuristic",
            summary: "Experimental SRP heuristic: flags impl blocks with many methods (off by default).",
        }
    }
}

impl Rule for SrpHeuristicRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, _config: &Config, out: &mut dyn Emitter) {
        let severity = self.cfg.level.to_severity();
        for file in &ws.files {
            let Some(ast) = &file.ast else { continue };
            for item in &ast.items {
                let syn::Item::Impl(imp) = item else { continue };
                let methods = imp
                    .items
                    .iter()
                    .filter(|i| matches!(i, syn::ImplItem::Fn(_)))
                    .count();
                if methods <= self.cfg.method_count_threshold {
                    continue;
                }
                out.emit(Finding {
                    rule_id: Self::static_info().id.to_string(),
                    severity,
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
                    fixes: Vec::new(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
