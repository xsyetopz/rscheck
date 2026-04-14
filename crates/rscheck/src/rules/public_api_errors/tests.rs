use crate::config::Level;
use crate::rules::PublicApiErrorsRule;
use crate::test_support::run_single_file_rule;

#[test]
fn flags_disallowed_public_error_type() {
    let findings = run_single_file_rule(
        &PublicApiErrorsRule,
        "design.public_api_errors",
        Level::Deny,
        toml::toml! {
            allowed_error_types = ["crate::Error"]
        },
        "pub fn run() -> Result<(), anyhow::Error> { Err(anyhow::Error::msg(\"boom\")) }\n",
    );
    assert_eq!(findings.len(), 1);
}

#[test]
fn does_not_treat_prefix_match_as_allowed() {
    let findings = run_single_file_rule(
        &PublicApiErrorsRule,
        "design.public_api_errors",
        Level::Deny,
        toml::toml! {
            allowed_error_types = ["crate::Error"]
        },
        r#"
pub fn run() -> Result<(), crate::Errorish> {
    panic!()
}
"#,
    );
    assert_eq!(findings.len(), 1);
}
