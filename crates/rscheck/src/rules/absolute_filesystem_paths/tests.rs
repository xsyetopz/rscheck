use super::AbsoluteFilesystemPathsRule;
use crate::config::Level;
use crate::test_support::run_single_file_rule;

#[test]
fn flags_unix_absolute_path_in_literal() {
    let findings = run_single_file_rule(
        &AbsoluteFilesystemPathsRule,
        "portability.absolute_literal_paths",
        Level::Warn,
        toml::Table::new(),
        r#"
fn f() {
    let _p = "/etc/passwd";
    let _q = "rel/path";
}
"#,
    );
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("/etc/passwd"));
}

#[test]
fn does_not_flag_safe_literal_patterns() {
    for code in [
        r#"
fn f(line: &str) -> bool {
    line.trim_start().starts_with("//!") || line.trim_start().starts_with("/*!")
}
        "#,
        r#"
fn routes() {
    let _api = "/api/v1/users";
    let _templated = "/users/{id}";
}
"#,
    ] {
        let findings = run_single_file_rule(
            &AbsoluteFilesystemPathsRule,
            "portability.absolute_literal_paths",
            Level::Warn,
            toml::Table::new(),
            code,
        );
        assert!(findings.is_empty());
    }
}
