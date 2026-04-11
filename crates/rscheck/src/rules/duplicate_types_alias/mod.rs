use crate::analysis::Workspace;
use crate::config::DuplicateTypesAliasConfig;
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use quote::ToTokens;
use std::collections::BTreeMap;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub struct DuplicateTypesAliasCandidateRule;

impl DuplicateTypesAliasCandidateRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "design.repeated_type_aliases",
            family: RuleFamily::Design,
            backend: RuleBackend::Syntax,
            summary: "Finds repeated identical type annotations that could be extracted into a `type` alias.",
            default_level: DuplicateTypesAliasConfig::default().level,
            schema: "level, min_occurrences, min_len, exclude_outer",
            config_example: "[rules.\"design.repeated_type_aliases\"]\nlevel = \"warn\"\nmin_occurrences = 3",
            fixable: false,
        }
    }
}

impl Rule for DuplicateTypesAliasCandidateRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<DuplicateTypesAliasConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            let Some(ast) = &file.ast else { continue };
            let mut v = TypeCollector {
                exclude_outer: &cfg.exclude_outer,
                types: Vec::new(),
            };
            v.visit_file(ast);

            let mut map: BTreeMap<String, Vec<proc_macro2::Span>> = BTreeMap::new();
            for t in v.types {
                if t.value.len() < cfg.min_len {
                    continue;
                }
                map.entry(t.value).or_default().push(t.span);
            }

            for (ty, spans) in map {
                if spans.len() < cfg.min_occurrences {
                    continue;
                }
                let primary = spans
                    .first()
                    .copied()
                    .map(|s| Span::from_pm_span(&file.path, s));
                out.emit(Finding {
                    rule_id: Self::static_info().id.to_string(),
                    family: Some(Self::static_info().family),
                    engine: Some(Self::static_info().backend),
                    severity: cfg.level.to_severity(),
                    message: format!(
                        "type is repeated {} times; consider a type alias: {ty}",
                        spans.len()
                    ),
                    primary,
                    secondary: Vec::new(),
                    help: Some(
                        "Introduce `type Alias = ...;` and use it consistently.".to_string(),
                    ),
                    evidence: None,
                    confidence: None,
                    tags: vec!["types".to_string()],
                    fixes: Vec::new(),
                });
            }
        }
    }
}

struct TypeRef {
    value: String,
    span: proc_macro2::Span,
}

struct TypeCollector<'a> {
    exclude_outer: &'a [String],
    types: Vec<TypeRef>,
}

impl TypeCollector<'_> {
    fn is_excluded_outer(&self, ty: &syn::Type) -> bool {
        let syn::Type::Path(p) = ty else { return false };
        if p.qself.is_some() {
            return false;
        }
        let Some(last) = p.path.segments.last() else {
            return false;
        };
        self.exclude_outer
            .iter()
            .any(|s| s == &last.ident.to_string())
    }

    fn record(&mut self, ty: &syn::Type) {
        if self.is_excluded_outer(ty) {
            return;
        }
        let value = normalize_type(ty);
        self.types.push(TypeRef {
            value,
            span: ty.span(),
        });
    }
}

impl<'ast> Visit<'ast> for TypeCollector<'_> {
    fn visit_field(&mut self, node: &'ast syn::Field) {
        self.record(&node.ty);
    }

    fn visit_fn_arg(&mut self, node: &'ast syn::FnArg) {
        if let syn::FnArg::Typed(pat_type) = node {
            self.record(&pat_type.ty);
        }
    }

    fn visit_return_type(&mut self, node: &'ast syn::ReturnType) {
        if let syn::ReturnType::Type(_, ty) = node {
            self.record(ty);
        }
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let syn::Pat::Type(pat_type) = &node.pat {
            self.record(&pat_type.ty);
        }
        syn::visit::visit_local(self, node);
    }
}

fn normalize_type(ty: &syn::Type) -> String {
    let s = ty.to_token_stream().to_string();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests;
