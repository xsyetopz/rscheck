use std::env::temp_dir;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::migrate_from;

#[test]
fn test_migrate_from_preview_keeps_file_unchanged() {
    let path = temp_config_path("preview");
    fs::write(&path, "version = 2\n").unwrap();

    let migration = migrate_from(&path, false).unwrap();

    assert!(migration.changed);
    assert_eq!(fs::read_to_string(&path).unwrap(), "version = 2\n");
    fs::remove_file(path).unwrap();
}

#[test]
fn test_migrate_from_write_updates_file() {
    let path = temp_config_path("write");
    fs::write(&path, "version = 2\n").unwrap();

    let migration = migrate_from(&path, true).unwrap();
    let text = fs::read_to_string(&path).unwrap();

    assert!(migration.changed);
    assert!(text.contains("version = 3"));
    fs::remove_file(path).unwrap();
}

fn temp_config_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    temp_dir().join(format!("rscheck-{label}-{nanos}.toml"))
}
