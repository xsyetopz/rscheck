use crate::config::Level;
use crate::rules::NamingPolicyRule;
use crate::test_support::run_single_file_rule;

#[test]
fn test_naming_policy_generic_binding_returns_warning() {
    let findings = run_single_file_rule(
        &NamingPolicyRule,
        NamingPolicyRule::static_info().id,
        Level::Warn,
        toml::Table::new(),
        "fn demo() { let data = 1; }
",
    );

    assert_eq!(findings.len(), 1);
    assert!(findings[0].message().contains("data"));
}

#[test]
fn test_naming_policy_domain_name_returns_no_findings() {
    let findings = run_single_file_rule(
        &NamingPolicyRule,
        NamingPolicyRule::static_info().id,
        Level::Warn,
        toml::Table::new(),
        "fn load_policy() { let policy_count = 1; }
",
    );

    assert!(findings.is_empty());
}
