use crate::config::Level;
use crate::emit::ReportEmitter;
use crate::report::Finding;
use crate::rules::ExternalTestModulesRule;
use crate::rules::{Rule, RuleContext};
use crate::test_support::{run_single_file_rule, single_rule_policy, workspace_from_files};

#[test]
fn test_external_test_modules_inline_cfg_tests_returns_error() {
    let findings = run_single_file_rule(
        &ExternalTestModulesRule,
        ExternalTestModulesRule::static_info().id,
        Level::Deny,
        toml::Table::new(),
        "#[cfg(test)]
mod tests { #[test] fn works() {} }
",
    );

    assert_eq!(findings.len(), 1);
    assert!(findings[0].message().contains("inline"));
}

#[test]
fn test_external_test_modules_non_test_module_returns_no_findings() {
    let findings = run_single_file_rule(
        &ExternalTestModulesRule,
        ExternalTestModulesRule::static_info().id,
        Level::Deny,
        toml::Table::new(),
        "mod tests { fn helper() {} }
",
    );

    assert!(findings.is_empty());
}

#[test]
fn test_external_test_modules_existing_root_sibling_returns_no_findings() {
    let findings = run_external_test_rule_on_files(&[
        (
            "lib.rs",
            "#[cfg(test)]
mod tests;
",
        ),
        (
            "tests.rs",
            "#[test]
fn works() {}
",
        ),
    ]);

    assert!(findings.is_empty());
}

#[test]
fn test_external_test_modules_existing_named_sibling_returns_no_findings() {
    let findings = run_external_test_rule_on_files(&[
        (
            "foo.rs",
            "#[cfg(test)]
mod tests;
",
        ),
        (
            "foo/tests.rs",
            "#[test]
fn works() {}
",
        ),
    ]);

    assert!(findings.is_empty());
}

#[test]
fn test_external_test_modules_missing_external_file_returns_error() {
    let findings = run_external_test_rule_on_files(&[(
        "lib.rs",
        "#[cfg(test)]
mod tests;
",
    )]);

    assert_eq!(findings.len(), 1);
    assert!(findings[0].message().contains("no sibling"));
}

fn run_external_test_rule_on_files(files: &[(&str, &str)]) -> Vec<Finding> {
    let workspace = workspace_from_files(files);
    let policy = single_rule_policy(
        ExternalTestModulesRule::static_info().id,
        Level::Deny,
        toml::Table::new(),
    );
    let mut emitter = ReportEmitter::new();
    ExternalTestModulesRule.run(&workspace, &RuleContext { policy: &policy }, &mut emitter);
    emitter.findings
}
