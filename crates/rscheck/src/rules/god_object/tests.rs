use crate::config::Level;
use crate::rules::GodObjectRule;
use crate::test_support::run_single_file_rule;

#[test]
fn test_god_object_large_struct_returns_warning() {
    let mut options = toml::Table::new();
    options.insert("max_fields".to_string(), toml::Value::Integer(2));
    let findings = run_single_file_rule(
        &GodObjectRule,
        GodObjectRule::static_info().id,
        Level::Warn,
        options,
        "struct Big { a: u8, b: u8, c: u8 }
",
    );

    assert_eq!(findings.len(), 1);
    assert!(findings[0].message().contains("3 fields"));
}

#[test]
fn test_god_object_small_struct_returns_no_findings() {
    let findings = run_single_file_rule(
        &GodObjectRule,
        GodObjectRule::static_info().id,
        Level::Warn,
        toml::Table::new(),
        "struct Small { policy_count: usize }
",
    );

    assert!(findings.is_empty());
}
