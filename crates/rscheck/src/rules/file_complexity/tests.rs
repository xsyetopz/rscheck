use super::FileComplexityRule;
use crate::analysis::{SourceFile, Workspace};
use crate::config::{Level, Policy, RuleSettings};
use crate::emit::ReportEmitter;
use crate::rules::{Rule, RuleContext};
use std::path::PathBuf;

fn ws_with_single_file(code: &str) -> Workspace {
    let root = PathBuf::from(".");
    let path = root.join("rscheck_test.rs");
    let ast = syn::parse_file(code).ok();
    Workspace {
        root,
        files: vec![SourceFile {
            path,
            text: code.to_string(),
            ast,
            parse_error: None,
        }],
    }
}

#[test]
fn emits_finding_and_records_metrics_when_over_threshold() {
    let ws = ws_with_single_file(
        r#"
fn f(x: i32) -> i32 {
    if x > 0 { 1 } else { 2 }
}
"#,
    );

    let mut cfg = Policy::default();
    cfg.rules.insert(
        "shape.file_complexity".to_string(),
        RuleSettings {
            level: Some(Level::Warn),
            options: toml::toml! {
                mode = "cyclomatic"
                max_file = 1
                max_fn = 1
                count_question = false
                match_arms = true
            },
        },
    );
    let mut emitter = ReportEmitter::new();
    FileComplexityRule.run(&ws, &RuleContext { policy: &cfg }, &mut emitter);

    assert!(!emitter.metrics.is_empty());
    assert!(!emitter.findings.is_empty());
}
