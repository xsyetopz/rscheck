use crate::config::Level;
use crate::rules::CustomPatternRule;
use crate::test_support::run_single_file_rule;

#[test]
fn test_custom_pattern_matching_regex_returns_warning() {
    let mut pattern = toml::Table::new();
    pattern.insert(
        "name".to_string(),
        toml::Value::String("todo_marker".to_string()),
    );
    pattern.insert("regex".to_string(), toml::Value::String("TODO".to_string()));
    pattern.insert(
        "message".to_string(),
        toml::Value::String("remove TODO markers".to_string()),
    );
    let mut options = toml::Table::new();
    options.insert(
        "patterns".to_string(),
        toml::Value::Array(vec![toml::Value::Table(pattern)]),
    );

    let findings = run_single_file_rule(
        &CustomPatternRule,
        CustomPatternRule::static_info().id,
        Level::Warn,
        options,
        "fn demo() {} // TODO remove
",
    );

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].message(), "remove TODO markers");
}

#[test]
fn test_custom_pattern_without_patterns_returns_no_findings() {
    let findings = run_single_file_rule(
        &CustomPatternRule,
        CustomPatternRule::static_info().id,
        Level::Warn,
        toml::Table::new(),
        "fn demo() {} // TODO remove
",
    );

    assert!(findings.is_empty());
}
