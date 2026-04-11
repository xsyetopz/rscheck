use super::SrpHeuristicRule;
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
fn flags_large_impl_block() {
    let ws = ws_with_single_file(
        r#"
struct S;
impl S {
    fn a(&self) {}
    fn b(&self) {}
    fn c(&self) {}
}
"#,
    );

    let mut cfg = Policy::default();
    cfg.rules.insert(
        "shape.responsibility_split".to_string(),
        RuleSettings {
            level: Some(Level::Warn),
            options: toml::toml! {
                method_count_threshold = 2
            },
        },
    );
    let mut emitter = ReportEmitter::new();
    SrpHeuristicRule.run(&ws, &RuleContext { policy: &cfg }, &mut emitter);

    assert_eq!(emitter.findings.len(), 1);
}
