use super::DuplicateLogicRule;
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
fn finds_similarity_between_two_functions() {
    let ws = ws_with_single_file(
        r#"
fn a(x: i32) -> i32 {
    if x > 10 { x + 1 } else { x + 2 }
}

fn b(y: i32) -> i32 {
    if y > 10 { y + 1 } else { y + 2 }
}
"#,
    );

    let mut cfg = Policy::default();
    cfg.rules.insert(
        "shape.duplicate_logic".to_string(),
        RuleSettings {
            level: Some(Level::Warn),
            options: toml::toml! {
                min_tokens = 10
                threshold = 0.5
                max_results = 10
                exclude_globs = []
                kgram = 5
            },
        },
    );
    let mut emitter = ReportEmitter::new();
    DuplicateLogicRule.run(&ws, &RuleContext { policy: &cfg }, &mut emitter);

    assert_eq!(emitter.findings.len(), 1);
    assert!(emitter.findings[0].message().contains("similarity"));
}
