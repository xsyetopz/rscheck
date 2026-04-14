use crate::config::Level;
use crate::rules::BannedDependenciesRule;
use crate::test_support::run_single_file_rule;

#[test]
fn flags_banned_path_prefix() {
    let findings = run_single_file_rule(
        &BannedDependenciesRule,
        "architecture.banned_dependencies",
        Level::Deny,
        toml::toml! {
            banned_prefixes = ["std::sync::Mutex"]
        },
        "use std::sync::Mutex;\n",
    );
    assert_eq!(findings.len(), 1);
}

#[test]
fn does_not_match_partial_path_prefix() {
    let findings = run_single_file_rule(
        &BannedDependenciesRule,
        "architecture.banned_dependencies",
        Level::Deny,
        toml::toml! {
            banned_prefixes = ["std::sync::Mutex"]
        },
        r#"
fn f(_: std::sync::MutexGuard<'static, u32>) {}
"#,
    );
    assert!(findings.is_empty());
}
