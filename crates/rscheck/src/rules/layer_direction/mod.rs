use crate::analysis::Workspace;
use crate::config::{LayerDirectionConfig, LayerRuleSet};
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use globset::{Glob, GlobSetBuilder};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub struct LayerDirectionRule;

impl LayerDirectionRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "architecture.layer_direction",
            family: RuleFamily::Architecture,
            backend: RuleBackend::Syntax,
            summary: "Checks path-scoped layer dependencies against configured direction rules.",
            default_level: LayerDirectionConfig::default().level,
            schema: "level, layers = [{ name, include, may_depend_on }]",
            config_example: "[rules.\"architecture.layer_direction\"]\nlevel = \"deny\"\nlayers = [{ name = \"api\", include = [\"src/api/**\"], may_depend_on = [\"domain\"] }]",
            fixable: false,
        }
    }
}

impl Rule for LayerDirectionRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<LayerDirectionConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            if !cfg.level.enabled() || cfg.layers.is_empty() {
                continue;
            }
            let Some(current) = match_layer(&cfg.layers, &file.path) else {
                continue;
            };
            let Some(ast) = &file.ast else { continue };
            let mut visitor = LayerVisitor {
                file: &file.path,
                current,
                layers: &cfg.layers,
                severity: cfg.level.to_severity(),
                out,
            };
            visitor.visit_file(ast);
        }
    }
}

struct LayerVisitor<'a> {
    file: &'a std::path::Path,
    current: &'a LayerRuleSet,
    layers: &'a [LayerRuleSet],
    severity: crate::report::Severity,
    out: &'a mut dyn Emitter,
}

impl LayerVisitor<'_> {
    fn check_path(&mut self, span: proc_macro2::Span, path: &syn::Path) {
        let text = path.to_token_stream().to_string().replace(' ', "");
        let Some(target) = self
            .layers
            .iter()
            .find(|layer| text.starts_with(&format!("crate::{}", layer.name)))
        else {
            return;
        };
        if target.name == self.current.name
            || self
                .current
                .may_depend_on
                .iter()
                .any(|name| name == &target.name)
        {
            return;
        }
        self.out.emit(Finding {
            rule_id: LayerDirectionRule::static_info().id.to_string(),
            family: Some(LayerDirectionRule::static_info().family),
            engine: Some(LayerDirectionRule::static_info().backend),
            severity: self.severity,
            message: format!(
                "layer `{}` cannot depend on `{}` through `{text}`",
                self.current.name, target.name
            ),
            primary: Some(Span::from_pm_span(self.file, span)),
            secondary: Vec::new(),
            help: Some("Move the dependency behind an allowed boundary.".to_string()),
            evidence: None,
            confidence: None,
            tags: vec!["architecture".to_string(), "layers".to_string()],
            fixes: Vec::new(),
        });
    }
}

impl<'ast> Visit<'ast> for LayerVisitor<'_> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if let Some(path) = use_tree_path(&node.tree) {
            self.check_path(node.span(), &path);
        }
        syn::visit::visit_item_use(self, node);
    }
}

fn match_layer<'a>(layers: &'a [LayerRuleSet], path: &std::path::Path) -> Option<&'a LayerRuleSet> {
    let candidate = path.to_string_lossy();
    layers
        .iter()
        .find(|layer| glob_matches(&layer.include, candidate.as_ref()))
}

fn glob_matches(patterns: &[String], candidate: &str) -> bool {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let Ok(glob) = Glob::new(pattern) else {
            continue;
        };
        builder.add(glob);
    }
    let Ok(set) = builder.build() else {
        return false;
    };
    set.is_match(candidate)
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
