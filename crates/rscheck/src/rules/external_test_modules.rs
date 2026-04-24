use crate::analysis::{SourceFile, Workspace};
use crate::config::ExternalTestModulesConfig;
use crate::emit::Emitter;
use crate::report::{Finding, FindingLabel, FindingLabelKind};
use crate::rules::{Rule, RuleBackend, RuleContext, RuleFamily, RuleInfo};
use crate::span::Span;
use quote::ToTokens;
use std::ffi::OsStr;
use std::path::Path;
use syn::spanned::Spanned;

pub struct ExternalTestModulesRule;

impl ExternalTestModulesRule {
    pub fn static_info() -> RuleInfo {
        RuleInfo {
            id: "testing.external_test_modules",
            family: RuleFamily::Testing,
            backend: RuleBackend::Syntax,
            summary: "Requires test modules to live in sibling test files instead of inline blocks.",
            default_level: ExternalTestModulesConfig::default().level,
            schema: "level, module_name, expected_file",
            config_example: "[rules.\"testing.external_test_modules\"]\nlevel = \"deny\"\nmodule_name = \"tests\"\nexpected_file = \"tests.rs\"",
            fixable: false,
        }
    }
}

impl Rule for ExternalTestModulesRule {
    fn info(&self) -> RuleInfo {
        Self::static_info()
    }

    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter) {
        for file in &ws.files {
            let cfg = match ctx
                .policy
                .decode_rule::<ExternalTestModulesConfig>(Self::static_info().id, Some(&file.path))
            {
                Ok(cfg) => cfg,
                Err(_) => continue,
            };
            if !cfg.level.enabled() {
                continue;
            }
            let Some(ast) = &file.ast else { continue };
            for item in &ast.items {
                let syn::Item::Mod(module) = item else {
                    continue;
                };
                if module.ident != cfg.module_name || !has_cfg_test(&module.attrs) {
                    continue;
                }
                if module.content.is_some() {
                    emit_inline_module(out, file, &cfg, module);
                } else if !expected_test_file_exists(&file.path, &cfg.expected_file) {
                    emit_missing_file(out, file, &cfg, module);
                }
            }
        }
    }
}

fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg") && attr.meta.to_token_stream().to_string().contains("test")
    })
}

fn expected_test_file_exists(source_path: &Path, expected_file: &str) -> bool {
    let Some(parent) = source_path.parent() else {
        return false;
    };
    let file_stem = source_path.file_stem().and_then(OsStr::to_str);
    match file_stem {
        Some("lib" | "main" | "mod") | None => parent.join(expected_file).is_file(),
        Some(stem) => parent.join(stem).join(expected_file).is_file(),
    }
}

fn emit_inline_module(
    out: &mut dyn Emitter,
    file: &SourceFile,
    cfg: &ExternalTestModulesConfig,
    module: &syn::ItemMod,
) {
    let message = format!(
        "inline `#[cfg(test)] mod {}` block must move to `{}`",
        cfg.module_name, cfg.expected_file
    );
    emit_test_module_finding(
        out,
        file,
        cfg,
        module,
        message,
        format!(
            "Use `#[cfg(test)] mod {};` and place tests in `{}`.",
            cfg.module_name, cfg.expected_file
        ),
    );
}

fn emit_missing_file(
    out: &mut dyn Emitter,
    file: &SourceFile,
    cfg: &ExternalTestModulesConfig,
    module: &syn::ItemMod,
) {
    let message = format!(
        "external test module `{}` has no sibling `{}`",
        cfg.module_name, cfg.expected_file
    );
    emit_test_module_finding(
        out,
        file,
        cfg,
        module,
        message,
        format!("Create `{}` next to this source file.", cfg.expected_file),
    );
}

fn emit_test_module_finding(
    out: &mut dyn Emitter,
    file: &SourceFile,
    cfg: &ExternalTestModulesConfig,
    module: &syn::ItemMod,
    message: String,
    help: String,
) {
    let span = Span::from_pm_span(&file.path, module.span());
    out.emit(
        Finding::from_rule(
            ExternalTestModulesRule::static_info(),
            cfg.level.to_severity(),
            Clone::clone(&message),
        )
        .with_primary(span.clone())
        .with_help(help)
        .with_confidence(String::from("high"))
        .with_tags(Vec::from([String::from("tests"), String::from("layout")]))
        .with_labels(Vec::from([FindingLabel {
            kind: FindingLabelKind::Primary,
            span,
            message: Some(message),
        }])),
    );
}

#[cfg(test)]
mod tests;
