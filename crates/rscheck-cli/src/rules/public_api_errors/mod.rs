use crate::analysis::Workspace;
use crate::config::PublicApiErrorsConfig;
use crate::emit::Emitter;
use crate::path_pattern::matches_path_prefix;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use quote::ToTokens;

pub struct PublicApiErrorsRule;

impl PublicApiErrorsRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "design.public_api_errors",
            family: RuleFamily::Design,
            backend: RuleBackend::Syntax,
            summary: "Checks public Result error types against an allow-list.",
            default_level: PublicApiErrorsConfig::default().level,
            schema: "level, allowed_error_types",
            config_example: "[rules.\"design.public_api_errors\"]\nlevel = \"deny\"\nallowed_error_types = [\"crate::Error\"]",
            fixable: false,
        }
    }
}

impl Rule for PublicApiErrorsRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<PublicApiErrorsConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            if !cfg.level.enabled() {
                continue;
            }
            let Some(ast) = &file.ast else { continue };
            for item in &ast.items {
                let syn::Item::Fn(item_fn) = item else {
                    continue;
                };
                if !matches!(item_fn.vis, syn::Visibility::Public(_)) {
                    continue;
                }
                let syn::ReturnType::Type(_, ty) = &item_fn.sig.output else {
                    continue;
                };
                let Some(error_ty) = extract_result_error_type(ty) else {
                    continue;
                };
                if cfg
                    .allowed_error_types
                    .iter()
                    .any(|allowed| matches_path_prefix(&error_ty, allowed))
                {
                    continue;
                }
                out.emit(Finding {
                    rule_id: Self::static_info().id.to_string(),
                    family: Some(Self::static_info().family),
                    engine: Some(Self::static_info().backend),
                    severity: cfg.level.to_severity(),
                    message: format!(
                        "public API returns disallowed error type `{error_ty}` in `{}`",
                        item_fn.sig.ident
                    ),
                    primary: Some(Span::from_pm_span(&file.path, item_fn.sig.ident.span())),
                    secondary: Vec::new(),
                    help: Some("Return an approved error type from this public API.".to_string()),
                    evidence: None,
                    confidence: None,
                    tags: vec!["api".to_string(), "errors".to_string()],
                    labels: Vec::new(),
                    notes: Vec::new(),
                    fixes: Vec::new(),
                });
            }
        }
    }
}

fn extract_result_error_type(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let mut ty_args = args.args.iter().filter_map(|arg| match arg {
        syn::GenericArgument::Type(ty) => Some(ty),
        _ => None,
    });
    let _ok = ty_args.next()?;
    let err = ty_args.next()?;
    Some(err.to_token_stream().to_string().replace(' ', ""))
}

#[cfg(test)]
mod tests;
