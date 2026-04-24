use std::fs;

use super::{MigrationError, migrate_policy_text};
use crate::config::{Level, Policy};

#[test]
fn test_migrate_policy_v2_config_returns_v3_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".rscheck.toml");
    fs::write(
        &path,
        r#"
version = 2

[engine]
semantic = "auto"
toolchain = "current"

[output]
format = "human"
with_clippy = false

[rules.file_complexity]
level = "warn"
max_file = 32

[[scope]]
include = ["crates/rscheck-cli/**"]

[scope.rules.srp_heuristic]
level = "deny"
"#,
    )
    .unwrap();

    let migration = migrate_policy_text(&path).unwrap();

    assert!(migration.changed);
    assert!(migration.text.contains("version = 3"));
    assert!(migration.text.contains("toolchain = \"stable\""));
    assert!(migration.text.contains("nightly_toolchain = \"nightly\""));
    assert!(migration.text.contains("[adapters.clippy]"));
    assert!(migration.text.contains("enabled = false"));
    assert!(migration.text.contains("[rules.\"shape.file_complexity\"]"));
    assert!(
        migration
            .text
            .contains("[[scope]]\ninclude = [\"crates/rscheck-cli/**\"]")
    );
    assert!(
        migration
            .text
            .contains("[scope.rules.\"shape.responsibility_split\"]")
    );
    assert!(!migration.text.contains("with_clippy"));
    assert!(!migration.text.contains("human"));

    let policy: Policy = toml::from_str(&migration.text).unwrap();
    policy.validate(&path).unwrap();
    let settings = policy.rule_settings("shape.file_complexity", None, Level::Warn);
    assert_eq!(settings.level, Some(Level::Warn));
}

#[test]
fn test_migrate_policy_already_v3_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".rscheck.toml");
    fs::write(&path, "version = 3\n").unwrap();

    let migration = migrate_policy_text(&path).unwrap();

    assert!(!migration.changed);
    assert!(migration.changes.is_empty());
    let policy: Policy = toml::from_str(&migration.text).unwrap();
    policy.validate(&path).unwrap();
}

#[test]
fn test_migrate_policy_top_level_workspace_keys_move_under_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".rscheck.toml");
    fs::write(
        &path,
        r#"
version = 2
include = ["src/**"]
exclude = ["target/**"]
"#,
    )
    .unwrap();

    let migration = migrate_policy_text(&path).unwrap();

    assert!(migration.text.contains("[workspace]"));
    assert!(migration.text.contains("include = [\"src/**\"]"));
    assert!(migration.text.contains("exclude = [\"target/**\"]"));
}

#[test]
fn test_migrate_policy_unknown_rule_key_reports_manual_step() {
    let error = manual_migration_error(
        r#"
version = 2

[rules.unknown_rule]
level = "warn"
"#,
    );

    assert!(matches!(error, MigrationError::Manual { .. }));
    assert!(error.to_string().contains("unknown legacy rule id"));
}

#[test]
fn test_migrate_policy_conflicting_workspace_key_reports_manual_step() {
    let error = manual_migration_error(
        r#"
version = 2
include = ["src/**"]

[workspace]
include = ["crates/**"]
"#,
    );

    assert!(matches!(error, MigrationError::Manual { .. }));
    assert!(error.to_string().contains("workspace.include"));
}

#[test]
fn test_migrate_policy_future_version_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".rscheck.toml");
    fs::write(&path, "version = 99\n").unwrap();

    let error = migrate_policy_text(&path).unwrap_err();

    assert!(matches!(error, MigrationError::UnsupportedVersion { .. }));
}

fn manual_migration_error(config: &str) -> MigrationError {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".rscheck.toml");
    fs::write(&path, config).unwrap();
    migrate_policy_text(&path).unwrap_err()
}
