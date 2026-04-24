use super::DuplicateTypesAliasCandidateRule;
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
fn respects_option_exclusion_and_suggests_alias_for_top_level_type() {
    let ws = ws_with_single_file(
        r#"
use std::collections::HashMap;

fn f(
    a: HashMap<String, Vec<(u32, u32)>>,
    b: HashMap<String, Vec<(u32, u32)>>,
    c: HashMap<String, Vec<(u32, u32)>>,
    d: Option<HashMap<String, Vec<(u32, u32)>>>,
    e: Option<HashMap<String, Vec<(u32, u32)>>>,
    f: Option<HashMap<String, Vec<(u32, u32)>>>,
) {}
"#,
    );

    let mut cfg = Policy::default();
    cfg.rules.insert(
        "design.repeated_type_aliases".to_string(),
        RuleSettings {
            level: Some(Level::Warn),
            options: toml::toml! {
                min_occurrences = 3
                min_len = 10
                exclude_outer = ["Option"]
            },
        },
    );
    let mut emitter = ReportEmitter::new();
    DuplicateTypesAliasCandidateRule.run(&ws, &RuleContext { policy: &cfg }, &mut emitter);

    assert_eq!(emitter.findings.len(), 1);
    assert!(emitter.findings[0].message().contains("type alias"));
}
