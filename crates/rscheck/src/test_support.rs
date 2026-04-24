#![cfg(test)]

use crate::analysis::Workspace;
use crate::config::{Level, Policy, RuleSettings};
use crate::emit::ReportEmitter;
use crate::report::Finding;
use crate::rules::{Rule, RuleContext};
use std::fs;

pub(crate) fn workspace_from_code(code: &str) -> Workspace {
    workspace_from_files(&[("lib.rs", code)])
}

pub(crate) fn workspace_from_files(files: &[(&str, &str)]) -> Workspace {
    let dir = tempfile::tempdir().expect("tempdir");
    for (relative_path, code) in files {
        let file = dir.path().join(relative_path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).expect("create test source parent");
        }
        fs::write(file, code).expect("write test source");
    }
    let root = dir.keep();
    Workspace::new(root)
        .load_files(&Policy::default())
        .expect("load test workspace")
}

pub(crate) fn single_rule_policy(rule_id: &str, level: Level, options: toml::Table) -> Policy {
    let mut policy = Policy::default();
    policy.rules.insert(
        rule_id.to_string(),
        RuleSettings {
            level: Some(level),
            options,
        },
    );
    policy
}

pub(crate) fn run_single_file_rule(
    rule: &dyn Rule,
    rule_id: &str,
    level: Level,
    options: toml::Table,
    code: &str,
) -> Vec<Finding> {
    let workspace = workspace_from_code(code);
    let policy = single_rule_policy(rule_id, level, options);
    let mut emitter = ReportEmitter::new();
    rule.run(&workspace, &RuleContext { policy: &policy }, &mut emitter);
    emitter.findings
}
