use super::{ConfigError, Level, Policy};
use std::fs;

#[test]
fn rejects_legacy_human_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".rscheck.toml");
    fs::write(&path, "[output]\nformat = \"human\"\n").unwrap();

    let err = Policy::from_path(&path).unwrap_err();
    assert!(matches!(err, ConfigError::LegacyKey { .. }));
    assert!(err.to_string().contains("text"));
}

#[test]
fn merges_extended_policy() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.toml");
    let child = dir.path().join("child.toml");

    fs::write(
        &base,
        "version = 3\n[rules.\"shape.file_complexity\"]\nlevel = \"warn\"\nmax_file = 10\n",
    )
    .unwrap();
    fs::write(
        &child,
        "version = 3\nextends = [\"base.toml\"]\n[rules.\"shape.file_complexity\"]\nmax_fn = 2\n",
    )
    .unwrap();

    let policy = Policy::from_path(&child).unwrap();
    let settings = policy.rule_settings("shape.file_complexity", None, Level::Warn);
    assert_eq!(settings.level, Some(Level::Warn));
    assert_eq!(
        settings
            .options
            .get("max_file")
            .and_then(toml::Value::as_integer),
        Some(10)
    );
    assert_eq!(
        settings
            .options
            .get("max_fn")
            .and_then(toml::Value::as_integer),
        Some(2)
    );
}
