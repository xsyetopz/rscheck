use crate::analysis::Workspace;
use crate::config::{Level, Policy, RuleSettings};
use crate::emit::ReportEmitter;
use crate::rules::{PublicApiErrorsRule, Rule, RuleContext};
use std::fs;

#[test]
fn flags_disallowed_public_error_type() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("lib.rs");
    fs::write(
        &file,
        "pub fn run() -> Result<(), anyhow::Error> { unimplemented!() }\n",
    )
    .unwrap();

    let ws = Workspace::new(dir.path().to_path_buf())
        .load_files(&Policy::default())
        .unwrap();
    let mut policy = Policy::default();
    policy.rules.insert(
        "design.public_api_errors".to_string(),
        RuleSettings {
            level: Some(Level::Deny),
            options: toml::toml! {
                allowed_error_types = ["crate::Error"]
            },
        },
    );

    let mut emitter = ReportEmitter::new();
    PublicApiErrorsRule.run(&ws, &RuleContext { policy: &policy }, &mut emitter);
    assert_eq!(emitter.findings.len(), 1);
}
