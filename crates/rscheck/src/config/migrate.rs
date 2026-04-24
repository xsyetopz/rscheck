use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{CURRENT_POLICY_VERSION, ConfigError, Policy, RuleTable, validate_legacy_shape};

const LEGACY_OUTPUT_FORMAT: &str = "human";
const CURRENT_OUTPUT_FORMAT: &str = "text";
const LEGACY_TOOLCHAIN: &str = "current";
const STABLE_TOOLCHAIN: &str = "stable";
const INHERIT_TOOLCHAIN: &str = "inherit";
const DEFAULT_NIGHTLY_TOOLCHAIN: &str = "nightly";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationResult {
    pub changed: bool,
    pub text: String,
    pub changes: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("failed to read config file: {path}")]
    Read { path: PathBuf, source: io::Error },
    #[error("failed to parse config file: {path}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to serialize migrated config")]
    Serialize(#[source] toml::ser::Error),
    #[error("failed to write config file: {path}")]
    Write { path: PathBuf, source: io::Error },
    #[error("policy version {version} cannot be migrated: {path}")]
    UnsupportedVersion { path: PathBuf, version: u32 },
    #[error("config requires manual migration: {path}\n{message}")]
    Manual { path: PathBuf, message: String },
    #[error(transparent)]
    InvalidMigratedPolicy(#[from] ConfigError),
}

#[must_use]
fn rule_id_mapping(legacy_key: &str) -> Option<&'static str> {
    match legacy_key {
        "absolute_module_paths" | "architecture.absolute_module_paths" => {
            Some("architecture.qualified_module_paths")
        }
        "absolute_filesystem_paths" | "portability.absolute_filesystem_paths" => {
            Some("portability.absolute_literal_paths")
        }
        "banned_dependencies" => Some("architecture.banned_dependencies"),
        "custom_pattern" => Some("pattern.custom"),
        "duplicate_logic" => Some("shape.duplicate_logic"),
        "duplicate_types_alias" | "design.duplicate_types_alias" => {
            Some("design.repeated_type_aliases")
        }
        "external_test_modules" => Some("testing.external_test_modules"),
        "file_complexity" => Some("shape.file_complexity"),
        "god_object" => Some("design.god_object"),
        "hot_path_allocations" => Some("perf.hot_path_allocations"),
        "layer_direction" => Some("architecture.layer_direction"),
        "naming_policy" => Some("design.naming_policy"),
        "public_api_errors" => Some("design.public_api_errors"),
        "srp_heuristic" | "shape.srp_heuristic" => Some("shape.responsibility_split"),
        _ => None,
    }
}

pub fn migrate_policy_text(path: &Path) -> Result<MigrationResult, MigrationError> {
    let text = fs::read_to_string(path).map_err(|source| MigrationError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut table: RuleTable = toml::from_str(&text).map_err(|source| MigrationError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    let mut changes = Vec::new();
    let mut manual_steps = Vec::new();

    let source_version = migrate_version(&mut table, path, &mut changes)?;
    migrate_workspace_paths(&mut table, &mut changes, &mut manual_steps);
    migrate_output(&mut table, &mut changes, &mut manual_steps);
    migrate_toolchains(&mut table, source_version.adds_v3_defaults(), &mut changes);
    migrate_rule_table(&mut table, "rules", &mut changes, &mut manual_steps);
    migrate_scope_rule_tables(&mut table, &mut changes, &mut manual_steps);

    if !manual_steps.is_empty() {
        return Err(MigrationError::Manual {
            path: path.to_path_buf(),
            message: manual_steps.join("\n"),
        });
    }

    validate_migrated_policy(path, &table)?;
    let mut migrated_text = toml::to_string_pretty(&table).map_err(MigrationError::Serialize)?;
    if !migrated_text.ends_with('\n') {
        migrated_text.push('\n');
    }

    Ok(MigrationResult {
        changed: !changes.is_empty(),
        text: migrated_text,
        changes,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceVersion {
    Current,
    Legacy,
}

impl SourceVersion {
    fn adds_v3_defaults(self) -> bool {
        self == Self::Legacy
    }
}

fn migrate_version(
    table: &mut RuleTable,
    path: &Path,
    changes: &mut Vec<String>,
) -> Result<SourceVersion, MigrationError> {
    let version = table
        .get("version")
        .and_then(toml::Value::as_integer)
        .map(u32::try_from)
        .transpose()
        .ok()
        .flatten();

    match version {
        Some(CURRENT_POLICY_VERSION) => Ok(SourceVersion::Current),
        Some(2) | None => {
            table.insert(
                String::from("version"),
                toml::Value::Integer(i64::from(CURRENT_POLICY_VERSION)),
            );
            changes.push(String::from("version: 2 -> 3"));
            Ok(SourceVersion::Legacy)
        }
        Some(version) => Err(MigrationError::UnsupportedVersion {
            path: path.to_path_buf(),
            version,
        }),
    }
}

fn migrate_workspace_paths(
    table: &mut RuleTable,
    changes: &mut Vec<String>,
    manual_steps: &mut Vec<String>,
) {
    move_top_level_key(
        table,
        "include",
        &["workspace"],
        "workspace.include",
        changes,
        manual_steps,
    );
    move_top_level_key(
        table,
        "exclude",
        &["workspace"],
        "workspace.exclude",
        changes,
        manual_steps,
    );
}

fn migrate_output(
    table: &mut RuleTable,
    changes: &mut Vec<String>,
    manual_steps: &mut Vec<String>,
) {
    if replace_string(
        table,
        &["output"],
        "format",
        LEGACY_OUTPUT_FORMAT,
        CURRENT_OUTPUT_FORMAT,
    ) {
        changes.push(String::from("output.format: human -> text"));
    }

    move_nested_key(
        table,
        NestedMove {
            source_path: &["output"],
            source_key: "with_clippy",
            destination_path: &["adapters", "clippy"],
            destination_key: "enabled",
            destination_label: "adapters.clippy.enabled",
        },
        changes,
        manual_steps,
    );
}

fn migrate_toolchains(table: &mut RuleTable, add_defaults: bool, changes: &mut Vec<String>) {
    if add_defaults {
        ensure_nested_string(
            table,
            &["engine"],
            "toolchain",
            STABLE_TOOLCHAIN,
            "engine.toolchain: default stable",
            changes,
        );
        ensure_nested_string(
            table,
            &["engine"],
            "nightly_toolchain",
            DEFAULT_NIGHTLY_TOOLCHAIN,
            "engine.nightly_toolchain: default nightly",
            changes,
        );
    }
    if replace_string(
        table,
        &["engine"],
        "toolchain",
        LEGACY_TOOLCHAIN,
        STABLE_TOOLCHAIN,
    ) {
        changes.push(String::from("engine.toolchain: current -> stable"));
    }

    if add_defaults {
        ensure_nested_string(
            table,
            &["adapters", "clippy"],
            "toolchain",
            INHERIT_TOOLCHAIN,
            "adapters.clippy.toolchain: default inherit",
            changes,
        );
    }
    if replace_string(
        table,
        &["adapters", "clippy"],
        "toolchain",
        LEGACY_TOOLCHAIN,
        STABLE_TOOLCHAIN,
    ) {
        changes.push(String::from("adapters.clippy.toolchain: current -> stable"));
    }
}

fn migrate_rule_table(
    table: &mut RuleTable,
    key: &str,
    changes: &mut Vec<String>,
    manual_steps: &mut Vec<String>,
) {
    let Some(toml::Value::Table(rules)) = table.get_mut(key) else {
        return;
    };
    migrate_rules_in_table(rules, key, changes, manual_steps);
}

fn migrate_scope_rule_tables(
    table: &mut RuleTable,
    changes: &mut Vec<String>,
    manual_steps: &mut Vec<String>,
) {
    let Some(toml::Value::Array(scopes)) = table.get_mut("scope") else {
        return;
    };
    for (scope_index, scope) in scopes.iter_mut().enumerate() {
        let toml::Value::Table(scope_table) = scope else {
            continue;
        };
        let Some(toml::Value::Table(rules)) = scope_table.get_mut("rules") else {
            continue;
        };
        let context = scope_rules_context(scope_index);
        migrate_rules_in_table(rules, &context, changes, manual_steps);
    }
}

fn migrate_rules_in_table(
    rules: &mut RuleTable,
    context: &str,
    changes: &mut Vec<String>,
    manual_steps: &mut Vec<String>,
) {
    let legacy_keys: Vec<String> = rules
        .keys()
        .filter(|candidate_key| should_migrate_rule_key(candidate_key))
        .cloned()
        .collect();

    for legacy_key in legacy_keys {
        let Some(new_key) = rule_id_mapping(&legacy_key) else {
            manual_steps.push(unknown_rule_step(context, &legacy_key));
            continue;
        };
        if !move_table_key(rules, &legacy_key, new_key) {
            manual_steps.push(conflicting_rule_step(context, &legacy_key, new_key));
            continue;
        }
        changes.push(rule_move_change(context, &legacy_key, new_key));
    }
}

fn scope_rules_context(scope_index: usize) -> String {
    format!("scope[{scope_index}].rules")
}

fn unknown_rule_step(context: &str, legacy_key: &str) -> String {
    format!("{context}.{legacy_key}: unknown legacy rule id")
}

fn conflicting_rule_step(context: &str, legacy_key: &str, new_key: &str) -> String {
    format!("{context}.{legacy_key}: conflicts with existing {context}.{new_key}")
}

fn rule_move_change(context: &str, legacy_key: &str, new_key: &str) -> String {
    format!("{context}.{legacy_key} -> {context}.{new_key}")
}

fn should_migrate_rule_key(candidate_key: &str) -> bool {
    !candidate_key.contains('.') || rule_id_mapping(candidate_key).is_some()
}

fn move_top_level_key(
    table: &mut RuleTable,
    source_key: &str,
    destination_path: &[&str],
    destination_key: &str,
    changes: &mut Vec<String>,
    manual_steps: &mut Vec<String>,
) {
    let Some(source_value) = table.remove(source_key) else {
        return;
    };
    let destination_table = ensure_table_path(table, destination_path);
    if can_insert(destination_table.get(source_key), &source_value) {
        destination_table.insert(source_key.to_owned(), source_value);
        changes.push(format!("{source_key} -> {destination_key}"));
    } else {
        table.insert(source_key.to_owned(), source_value);
        manual_steps.push(format!(
            "{source_key}: conflicts with existing {destination_key}"
        ));
    }
}

struct NestedMove<'a> {
    source_path: &'a [&'a str],
    source_key: &'a str,
    destination_path: &'a [&'a str],
    destination_key: &'a str,
    destination_label: &'a str,
}

fn move_nested_key(
    table: &mut RuleTable,
    migration: NestedMove<'_>,
    changes: &mut Vec<String>,
    manual_steps: &mut Vec<String>,
) {
    let Some(source_table) = table_at_mut(table, migration.source_path) else {
        return;
    };
    let Some(source_value) = source_table.remove(migration.source_key) else {
        return;
    };
    let destination_table = ensure_table_path(table, migration.destination_path);
    if can_insert(
        destination_table.get(migration.destination_key),
        &source_value,
    ) {
        destination_table.insert(migration.destination_key.to_owned(), source_value);
        changes.push(format!(
            "{}.{} -> {destination_label}",
            migration.source_path.join("."),
            migration.source_key,
            destination_label = migration.destination_label
        ));
    } else {
        let source_table = ensure_table_path(table, migration.source_path);
        source_table.insert(migration.source_key.to_owned(), source_value);
        manual_steps.push(format!(
            "{}.{}: conflicts with existing {destination_label}",
            migration.source_path.join("."),
            migration.source_key,
            destination_label = migration.destination_label
        ));
    }
}

fn can_insert(existing_value: Option<&toml::Value>, source_value: &toml::Value) -> bool {
    existing_value.is_none_or(|destination_value| destination_value == source_value)
}

fn move_table_key(table: &mut RuleTable, source_key: &str, destination_key: &str) -> bool {
    let Some(source_value) = table.remove(source_key) else {
        return true;
    };
    if can_insert(table.get(destination_key), &source_value) {
        table.insert(destination_key.to_owned(), source_value);
        true
    } else {
        table.insert(source_key.to_owned(), source_value);
        false
    }
}

fn replace_string(
    table: &mut RuleTable,
    path: &[&str],
    key: &str,
    old_text: &str,
    new_text: &str,
) -> bool {
    let Some(destination_table) = table_at_mut(table, path) else {
        return false;
    };
    let Some(toml::Value::String(current_text)) = destination_table.get_mut(key) else {
        return false;
    };
    if current_text != old_text {
        return false;
    }
    *current_text = new_text.to_owned();
    true
}

fn ensure_nested_string(
    table: &mut RuleTable,
    path: &[&str],
    key: &str,
    text: &str,
    change: &str,
    changes: &mut Vec<String>,
) {
    let destination_table = ensure_table_path(table, path);
    if destination_table.contains_key(key) {
        return;
    }
    destination_table.insert(key.to_owned(), toml::Value::String(text.to_owned()));
    changes.push(change.to_owned());
}

fn table_at_mut<'a>(table: &'a mut RuleTable, path: &[&str]) -> Option<&'a mut RuleTable> {
    let mut current_table = table;
    for segment in path {
        let Some(toml::Value::Table(next_table)) = current_table.get_mut(*segment) else {
            return None;
        };
        current_table = next_table;
    }
    Some(current_table)
}

fn ensure_table_path<'a>(table: &'a mut RuleTable, path: &[&str]) -> &'a mut RuleTable {
    let mut current_table = table;
    for segment in path {
        current_table = ensure_child_table(current_table, segment);
    }
    current_table
}

fn ensure_child_table<'a>(table: &'a mut RuleTable, segment: &str) -> &'a mut RuleTable {
    let needs_table = !matches!(table.get(segment), Some(toml::Value::Table(_)));
    if needs_table {
        table.insert(segment_key(segment), toml::Value::Table(RuleTable::new()));
    }
    table
        .get_mut(segment)
        .and_then(toml::Value::as_table_mut)
        .expect("inserted table path segment")
}

fn segment_key(segment: &str) -> String {
    segment.to_owned()
}

fn validate_migrated_policy(path: &Path, table: &RuleTable) -> Result<(), MigrationError> {
    validate_legacy_shape(table, path)?;
    let policy: Policy = toml::from_str(
        &toml::to_string(table).map_err(MigrationError::Serialize)?,
    )
    .map_err(|source| MigrationError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    policy.validate(path)?;
    Ok(())
}

#[cfg(test)]
mod tests;
