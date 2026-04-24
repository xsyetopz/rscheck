use crate::analysis::Workspace;
use crate::config::GodObjectConfig;
use crate::emit::Emitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use std::path::Path;
use syn::spanned::Spanned;

pub struct GodObjectRule;

impl GodObjectRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "design.god_object",
            family: RuleFamily::Design,
            backend: RuleBackend::Syntax,
            summary: "Flags large structs, enums, and impls that likely own unrelated responsibilities.",
            default_level: GodObjectConfig::default().level,
            schema: "level, max_fields, max_variants, max_methods",
            config_example: "[rules.\"design.god_object\"]\nlevel = \"warn\"\nmax_fields = 12\nmax_methods = 20",
            fixable: false,
        }
    }
}

impl Rule for GodObjectRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<GodObjectConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            if !cfg.level.enabled() {
                continue;
            }
            let Some(ast) = &file.ast else { continue };
            for item in &ast.items {
                check_item(&file.path, &cfg, item, out);
            }
        }
    }
}

fn check_item(file: &Path, cfg: &GodObjectConfig, item: &syn::Item, out: &mut dyn Emitter) {
    match item {
        syn::Item::Struct(item) => {
            let count = item.fields.len();
            if count > cfg.max_fields {
                emit(
                    out,
                    file,
                    cfg,
                    item.span(),
                    format!("struct `{}` has {count} fields", item.ident),
                );
            }
        }
        syn::Item::Enum(item) => {
            let count = item.variants.len();
            if count > cfg.max_variants {
                emit(
                    out,
                    file,
                    cfg,
                    item.span(),
                    format!("enum `{}` has {count} variants", item.ident),
                );
            }
        }
        syn::Item::Impl(item) => {
            let count = item
                .items
                .iter()
                .filter(|entry| matches!(entry, syn::ImplItem::Fn(_)))
                .count();
            if count > cfg.max_methods {
                emit(
                    out,
                    file,
                    cfg,
                    item.span(),
                    format!("impl block has {count} methods"),
                );
            }
        }
        _ => {}
    }
}

fn emit(
    out: &mut dyn Emitter,
    file: &Path,
    cfg: &GodObjectConfig,
    span: proc_macro2::Span,
    message: String,
) {
    out.emit(
        Finding::from_rule(
            GodObjectRule::static_info(),
            cfg.level.to_severity(),
            message,
        )
        .with_primary(Span::from_pm_span(file, span))
        .with_help(String::from(
            "Split storage, normalization, checking, diagnostics, and effects into focused types.",
        ))
        .with_confidence(String::from("medium"))
        .with_tags(Vec::from([String::from("srp"), String::from("shape")])),
    );
}

#[cfg(test)]
mod tests;
