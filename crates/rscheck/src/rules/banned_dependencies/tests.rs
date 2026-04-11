use crate::analysis::Workspace;
use crate::config::{Level, Policy, RuleSettings};
use crate::emit::ReportEmitter;
use crate::rules::{BannedDependenciesRule, Rule, RuleContext};
use std::fs;

#[test]
fn flags_banned_path_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("lib.rs");
    fs::write(&file, "use std::sync::Mutex;\n").unwrap();

    let ws = Workspace::new(dir.path().to_path_buf())
        .load_files(&Policy::default())
        .unwrap();
    let mut policy = Policy::default();
    policy.rules.insert(
        "architecture.banned_dependencies".to_string(),
        RuleSettings {
            level: Some(Level::Deny),
            options: toml::toml! {
                banned_prefixes = ["std::sync::Mutex"]
            },
        },
    );

    let mut emitter = ReportEmitter::new();
    BannedDependenciesRule.run(&ws, &RuleContext { policy: &policy }, &mut emitter);
    assert_eq!(emitter.findings.len(), 1);
}
