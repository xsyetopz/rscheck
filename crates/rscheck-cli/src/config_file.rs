use rscheck::config::{ConfigError, MigrationError, MigrationResult, Policy, migrate_policy_text};
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_from(path: &Path) -> Result<Policy, ConfigError> {
    Policy::from_path(path)
}

pub fn write_default_config(path: &Path) -> Result<(), ConfigError> {
    let config = Policy::default_with_rules(rscheck::rules::default_rule_settings());
    let toml = toml::to_string_pretty(&config).map_err(ConfigError::Serialize)?;
    fs::write(path, toml).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })
}

pub fn migrate_from(path: &Path, write: bool) -> Result<MigrationResult, MigrationError> {
    let migration = migrate_policy_text(path)?;
    if write && migration.changed {
        fs::write(path, &migration.text).map_err(|source| MigrationError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(migration)
}

pub fn workspace_root() -> Result<PathBuf, cargo_metadata::Error> {
    let metadata = cargo_metadata::MetadataCommand::new().no_deps().exec()?;
    Ok(PathBuf::from(metadata.workspace_root.as_std_path()))
}

pub fn default_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".rscheck.toml")
}

#[cfg(test)]
mod tests;
