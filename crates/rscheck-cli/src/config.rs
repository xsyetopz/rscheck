use globset::{Glob, GlobSetBuilder};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::report::Severity;

pub type RuleTable = toml::Table;

const CURRENT_POLICY_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    #[default]
    Allow,
    Warn,
    Deny,
}

impl Level {
    #[must_use]
    pub fn enabled(self) -> bool {
        !matches!(self, Self::Allow)
    }

    #[must_use]
    pub fn to_severity(self) -> Severity {
        match self {
            Self::Allow => Severity::Info,
            Self::Warn => Severity::Warn,
            Self::Deny => Severity::Deny,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EngineMode {
    #[default]
    Auto,
    Require,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToolchainMode {
    #[default]
    Current,
    Auto,
    Nightly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AdapterToolchainMode {
    #[default]
    Inherit,
    Current,
    Auto,
    Nightly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Sarif,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    #[serde(default)]
    pub semantic: EngineMode,
    #[serde(default)]
    pub toolchain: ToolchainMode,
    #[serde(default = "EngineConfig::default_nightly_toolchain")]
    pub nightly_toolchain: String,
}

impl EngineConfig {
    fn default_nightly_toolchain() -> String {
        "nightly".to_string()
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            semantic: EngineMode::Auto,
            toolchain: ToolchainMode::Current,
            nightly_toolchain: Self::default_nightly_toolchain(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "WorkspaceConfig::default_include")]
    pub include: Vec<String>,
    #[serde(default = "WorkspaceConfig::default_exclude")]
    pub exclude: Vec<String>,
}

impl WorkspaceConfig {
    fn default_include() -> Vec<String> {
        vec!["**/*.rs".to_string()]
    }

    fn default_exclude() -> Vec<String> {
        vec!["target/**".to_string(), ".git/**".to_string()]
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            include: Self::default_include(),
            exclude: Self::default_exclude(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    #[serde(default)]
    pub format: OutputFormat,
    #[serde(default)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClippyAdapterConfig {
    #[serde(default = "ClippyAdapterConfig::default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub toolchain: AdapterToolchainMode,
}

impl ClippyAdapterConfig {
    fn default_enabled() -> bool {
        true
    }
}

impl Default for ClippyAdapterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            args: Vec::new(),
            toolchain: AdapterToolchainMode::Inherit,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdaptersConfig {
    #[serde(default)]
    pub clippy: ClippyAdapterConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleSettings {
    #[serde(default)]
    pub level: Option<Level>,
    #[serde(flatten)]
    pub options: RuleTable,
}

impl RuleSettings {
    #[must_use]
    pub fn merge(&self, override_settings: &Self) -> Self {
        let mut options = self.options.clone();
        merge_tables(&mut options, &override_settings.options);
        Self {
            level: override_settings.level.or(self.level),
            options,
        }
    }

    #[must_use]
    pub fn with_default_level(mut self, default_level: Level) -> Self {
        if self.level.is_none() {
            self.level = Some(default_level);
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    #[serde(default = "Policy::default_version")]
    pub version: u32,
    #[serde(default)]
    pub extends: Vec<PathBuf>,
    #[serde(default)]
    pub engine: EngineConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub adapters: AdaptersConfig,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleSettings>,
    #[serde(rename = "scope", default)]
    pub scopes: Vec<ScopeConfig>,
}

impl Policy {
    fn default_version() -> u32 {
        CURRENT_POLICY_VERSION
    }

    #[must_use]
    pub fn default_with_rules(
        default_rules: impl IntoIterator<Item = (String, RuleSettings)>,
    ) -> Self {
        Self {
            rules: default_rules.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let table = load_policy_table(path)?;
        validate_legacy_shape(&table, path)?;
        let policy: Self = toml::from_str(
            &toml::to_string(&table).map_err(ConfigError::Serialize)?,
        )
        .map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        policy.validate(path)?;
        Ok(policy)
    }

    pub fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        if self.version != CURRENT_POLICY_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                path: path.to_path_buf(),
                version: self.version,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn rule_enabled_anywhere(&self, rule_id: &str, default_level: Level) -> bool {
        if self
            .rule_settings(rule_id, None, default_level)
            .level
            .unwrap_or(default_level)
            .enabled()
        {
            return true;
        }
        self.scopes.iter().any(|scope| {
            scope
                .rules
                .get(rule_id)
                .and_then(|settings| settings.level)
                .unwrap_or(default_level)
                .enabled()
        })
    }

    #[must_use]
    pub fn rule_settings(
        &self,
        rule_id: &str,
        file_path: Option<&Path>,
        default_level: Level,
    ) -> RuleSettings {
        let mut resolved = self.rules.get(rule_id).cloned().unwrap_or_default();
        for scope in &self.scopes {
            if !scope_matches(scope, file_path) {
                continue;
            }
            if let Some(scope_settings) = scope.rules.get(rule_id) {
                resolved = resolved.merge(scope_settings);
            }
        }
        resolved.with_default_level(default_level)
    }

    pub fn decode_rule<T>(&self, rule_id: &str, file_path: Option<&Path>) -> Result<T, ConfigError>
    where
        T: RuleOptions,
    {
        let resolved = self.rule_settings(rule_id, file_path, T::default_level());
        decode_rule_settings::<T>(&resolved).map_err(|message| ConfigError::RuleDecode {
            rule_id: rule_id.to_string(),
            message,
        })
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            version: CURRENT_POLICY_VERSION,
            extends: Vec::new(),
            engine: EngineConfig::default(),
            workspace: WorkspaceConfig::default(),
            output: OutputConfig::default(),
            adapters: AdaptersConfig::default(),
            rules: BTreeMap::new(),
            scopes: Vec::new(),
        }
    }
}

pub trait RuleOptions: DeserializeOwned {
    fn default_level() -> Level;
}

pub type Config = Policy;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {path}")]
    Read { path: PathBuf, source: io::Error },
    #[error("failed to parse config file: {path}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to serialize policy")]
    Serialize(#[source] toml::ser::Error),
    #[error("failed to write config file: {path}")]
    Write { path: PathBuf, source: io::Error },
    #[error("policy version {version} is not supported: {path}")]
    UnsupportedVersion { path: PathBuf, version: u32 },
    #[error("legacy config key `{key}` is not supported in v2: {path}. {message}")]
    LegacyKey {
        path: PathBuf,
        key: String,
        message: String,
    },
    #[error("failed to decode rule `{rule_id}`: {message}")]
    RuleDecode { rule_id: String, message: String },
    #[error("failed to build glob matcher: {pattern}")]
    Glob {
        pattern: String,
        #[source]
        source: globset::Error,
    },
}

fn decode_rule_settings<T>(settings: &RuleSettings) -> Result<T, String>
where
    T: RuleOptions,
{
    let mut table = settings.options.clone();
    if let Some(level) = settings.level {
        table.insert(
            "level".to_string(),
            toml::Value::String(level_string(level).to_string()),
        );
    }
    let text = toml::to_string(&table).map_err(|err| err.to_string())?;
    toml::from_str(&text).map_err(|err| err.to_string())
}

fn level_string(level: Level) -> &'static str {
    match level {
        Level::Allow => "allow",
        Level::Warn => "warn",
        Level::Deny => "deny",
    }
}

fn load_policy_table(path: &Path) -> Result<RuleTable, ConfigError> {
    let mut visiting = Vec::new();
    load_policy_table_inner(path, &mut visiting)
}

fn load_policy_table_inner(
    path: &Path,
    visiting: &mut Vec<PathBuf>,
) -> Result<RuleTable, ConfigError> {
    let canonical = path.to_path_buf();
    if visiting.contains(&canonical) {
        return Ok(RuleTable::new());
    }
    visiting.push(canonical);

    let text = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let table: RuleTable = toml::from_str(&text).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    let mut merged = RuleTable::new();
    if let Some(extends) = table.get("extends").and_then(toml::Value::as_array) {
        for entry in extends {
            let Some(relative) = entry.as_str() else {
                continue;
            };
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            let nested = load_policy_table_inner(&parent.join(relative), visiting)?;
            merge_tables(&mut merged, &nested);
        }
    }
    merge_tables(&mut merged, &table);
    let _ = visiting.pop();
    Ok(merged)
}

fn validate_legacy_shape(table: &RuleTable, path: &Path) -> Result<(), ConfigError> {
    if let Some(output) = table.get("output").and_then(toml::Value::as_table) {
        if output.get("with_clippy").is_some() {
            return Err(ConfigError::LegacyKey {
                path: path.to_path_buf(),
                key: "output.with_clippy".to_string(),
                message: "Use `[adapters.clippy].enabled`.".to_string(),
            });
        }
        if output.get("format").and_then(toml::Value::as_str) == Some("human") {
            return Err(ConfigError::LegacyKey {
                path: path.to_path_buf(),
                key: "output.format".to_string(),
                message: "Use `text` instead of `human`.".to_string(),
            });
        }
    }

    if let Some(rules) = table.get("rules").and_then(toml::Value::as_table) {
        for key in rules.keys() {
            if !key.contains('.') {
                return Err(ConfigError::LegacyKey {
                    path: path.to_path_buf(),
                    key: format!("rules.{key}"),
                    message: "Use dot rule IDs such as `shape.file_complexity`.".to_string(),
                });
            }
        }
    }

    if table.get("include").is_some() || table.get("exclude").is_some() {
        return Err(ConfigError::LegacyKey {
            path: path.to_path_buf(),
            key: "include/exclude".to_string(),
            message: "Move these under `[workspace]`.".to_string(),
        });
    }
    Ok(())
}

fn merge_tables(into: &mut RuleTable, overlay: &RuleTable) {
    for (key, value) in overlay {
        match (into.get_mut(key), value) {
            (Some(toml::Value::Table(dst)), toml::Value::Table(src)) => merge_tables(dst, src),
            _ => {
                into.insert(key.clone(), value.clone());
            }
        }
    }
}

fn scope_matches(scope: &ScopeConfig, file_path: Option<&Path>) -> bool {
    let Some(file_path) = file_path else {
        return false;
    };
    let display = file_path.to_string_lossy();

    if !scope.include.is_empty() && !globset_matches(&scope.include, display.as_ref()) {
        return false;
    }
    if !scope.exclude.is_empty() && globset_matches(&scope.exclude, display.as_ref()) {
        return false;
    }
    true
}

fn globset_matches(patterns: &[String], candidate: &str) -> bool {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let Ok(glob) = Glob::new(pattern) else {
            continue;
        };
        builder.add(glob);
    }
    let Ok(set) = builder.build() else {
        return false;
    };
    set.is_match(candidate)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsoluteModulePathsConfig {
    #[serde(default = "AbsoluteModulePathsConfig::default_level")]
    pub level: Level,
    #[serde(default)]
    pub allow_prefixes: Vec<String>,
    #[serde(default = "AbsoluteModulePathsConfig::default_roots")]
    pub roots: Vec<String>,
    #[serde(default = "AbsoluteModulePathsConfig::default_allow_crate_root_macros")]
    pub allow_crate_root_macros: bool,
    #[serde(default = "AbsoluteModulePathsConfig::default_allow_crate_root_consts")]
    pub allow_crate_root_consts: bool,
    #[serde(default = "AbsoluteModulePathsConfig::default_allow_crate_root_fn_calls")]
    pub allow_crate_root_fn_calls: bool,
}

impl AbsoluteModulePathsConfig {
    fn default_level() -> Level {
        Level::Deny
    }

    fn default_roots() -> Vec<String> {
        vec![
            "std".to_string(),
            "core".to_string(),
            "alloc".to_string(),
            "crate".to_string(),
        ]
    }

    fn default_allow_crate_root_macros() -> bool {
        true
    }

    fn default_allow_crate_root_consts() -> bool {
        true
    }

    fn default_allow_crate_root_fn_calls() -> bool {
        true
    }
}

impl Default for AbsoluteModulePathsConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            allow_prefixes: Vec::new(),
            roots: Self::default_roots(),
            allow_crate_root_macros: true,
            allow_crate_root_consts: true,
            allow_crate_root_fn_calls: true,
        }
    }
}

impl RuleOptions for AbsoluteModulePathsConfig {
    fn default_level() -> Level {
        Self::default_level()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsoluteFilesystemPathsConfig {
    #[serde(default = "AbsoluteFilesystemPathsConfig::default_level")]
    pub level: Level,
    #[serde(default)]
    pub allow_globs: Vec<String>,
    #[serde(default)]
    pub allow_regex: Vec<String>,
    #[serde(default)]
    pub check_comments: bool,
}

impl AbsoluteFilesystemPathsConfig {
    fn default_level() -> Level {
        Level::Warn
    }
}

impl Default for AbsoluteFilesystemPathsConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            allow_globs: Vec::new(),
            allow_regex: Vec::new(),
            check_comments: false,
        }
    }
}

impl RuleOptions for AbsoluteFilesystemPathsConfig {
    fn default_level() -> Level {
        Self::default_level()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplexityMode {
    Cyclomatic,
    PhysicalLoc,
    LogicalLoc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileComplexityConfig {
    #[serde(default = "FileComplexityConfig::default_level")]
    pub level: Level,
    #[serde(default = "FileComplexityConfig::default_mode")]
    pub mode: ComplexityMode,
    #[serde(default = "FileComplexityConfig::default_max_file")]
    pub max_file: u32,
    #[serde(default = "FileComplexityConfig::default_max_fn")]
    pub max_fn: u32,
    #[serde(default = "FileComplexityConfig::default_count_question")]
    pub count_question: bool,
    #[serde(default = "FileComplexityConfig::default_match_arms")]
    pub match_arms: bool,
}

impl FileComplexityConfig {
    fn default_level() -> Level {
        Level::Warn
    }
    fn default_mode() -> ComplexityMode {
        ComplexityMode::Cyclomatic
    }
    fn default_max_file() -> u32 {
        200
    }
    fn default_max_fn() -> u32 {
        25
    }
    fn default_count_question() -> bool {
        false
    }
    fn default_match_arms() -> bool {
        true
    }
}

impl Default for FileComplexityConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            mode: Self::default_mode(),
            max_file: 200,
            max_fn: 25,
            count_question: false,
            match_arms: true,
        }
    }
}

impl RuleOptions for FileComplexityConfig {
    fn default_level() -> Level {
        Self::default_level()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateLogicConfig {
    #[serde(default = "DuplicateLogicConfig::default_level")]
    pub level: Level,
    #[serde(default = "DuplicateLogicConfig::default_min_tokens")]
    pub min_tokens: usize,
    #[serde(default = "DuplicateLogicConfig::default_threshold")]
    pub threshold: f32,
    #[serde(default = "DuplicateLogicConfig::default_max_results")]
    pub max_results: usize,
    #[serde(default)]
    pub exclude_globs: Vec<String>,
    #[serde(default = "DuplicateLogicConfig::default_kgram")]
    pub kgram: usize,
}

impl DuplicateLogicConfig {
    fn default_level() -> Level {
        Level::Warn
    }
    fn default_min_tokens() -> usize {
        80
    }
    fn default_threshold() -> f32 {
        0.80
    }
    fn default_max_results() -> usize {
        200
    }
    fn default_kgram() -> usize {
        25
    }
}

impl Default for DuplicateLogicConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            min_tokens: 80,
            threshold: 0.80,
            max_results: 200,
            exclude_globs: Vec::new(),
            kgram: 25,
        }
    }
}

impl RuleOptions for DuplicateLogicConfig {
    fn default_level() -> Level {
        Self::default_level()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateTypesAliasConfig {
    #[serde(default = "DuplicateTypesAliasConfig::default_level")]
    pub level: Level,
    #[serde(default = "DuplicateTypesAliasConfig::default_min_occurrences")]
    pub min_occurrences: usize,
    #[serde(default = "DuplicateTypesAliasConfig::default_min_len")]
    pub min_len: usize,
    #[serde(default = "DuplicateTypesAliasConfig::default_exclude_outer")]
    pub exclude_outer: Vec<String>,
}

impl DuplicateTypesAliasConfig {
    fn default_level() -> Level {
        Level::Allow
    }
    fn default_min_occurrences() -> usize {
        3
    }
    fn default_min_len() -> usize {
        25
    }
    fn default_exclude_outer() -> Vec<String> {
        vec!["Option".to_string()]
    }
}

impl Default for DuplicateTypesAliasConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            min_occurrences: 3,
            min_len: 25,
            exclude_outer: Self::default_exclude_outer(),
        }
    }
}

impl RuleOptions for DuplicateTypesAliasConfig {
    fn default_level() -> Level {
        Self::default_level()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrpHeuristicConfig {
    #[serde(default)]
    pub level: Level,
    #[serde(default = "SrpHeuristicConfig::default_method_count")]
    pub method_count_threshold: usize,
}

impl SrpHeuristicConfig {
    fn default_method_count() -> usize {
        25
    }
}

impl Default for SrpHeuristicConfig {
    fn default() -> Self {
        Self {
            level: Level::Allow,
            method_count_threshold: 25,
        }
    }
}

impl RuleOptions for SrpHeuristicConfig {
    fn default_level() -> Level {
        Level::Allow
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannedDependenciesConfig {
    #[serde(default = "BannedDependenciesConfig::default_level")]
    pub level: Level,
    #[serde(default)]
    pub banned_prefixes: Vec<String>,
}

impl BannedDependenciesConfig {
    fn default_level() -> Level {
        Level::Allow
    }
}

impl Default for BannedDependenciesConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            banned_prefixes: Vec::new(),
        }
    }
}

impl RuleOptions for BannedDependenciesConfig {
    fn default_level() -> Level {
        Self::default_level()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicApiErrorsConfig {
    #[serde(default = "PublicApiErrorsConfig::default_level")]
    pub level: Level,
    #[serde(default = "PublicApiErrorsConfig::default_allowed_error_types")]
    pub allowed_error_types: Vec<String>,
}

impl PublicApiErrorsConfig {
    fn default_level() -> Level {
        Level::Allow
    }

    fn default_allowed_error_types() -> Vec<String> {
        vec![
            "crate::Error".to_string(),
            "crate::error::Error".to_string(),
        ]
    }
}

impl Default for PublicApiErrorsConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            allowed_error_types: Self::default_allowed_error_types(),
        }
    }
}

impl RuleOptions for PublicApiErrorsConfig {
    fn default_level() -> Level {
        Self::default_level()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerRuleSet {
    pub name: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub may_depend_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDirectionConfig {
    #[serde(default = "LayerDirectionConfig::default_level")]
    pub level: Level,
    #[serde(default)]
    pub layers: Vec<LayerRuleSet>,
}

impl LayerDirectionConfig {
    fn default_level() -> Level {
        Level::Allow
    }
}

impl Default for LayerDirectionConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            layers: Vec::new(),
        }
    }
}

impl RuleOptions for LayerDirectionConfig {
    fn default_level() -> Level {
        Self::default_level()
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => f.write_str("text"),
            Self::Json => f.write_str("json"),
            Self::Sarif => f.write_str("sarif"),
            Self::Html => f.write_str("html"),
        }
    }
}

#[cfg(test)]
mod tests {
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
            "version = 2\n[rules.\"shape.file_complexity\"]\nlevel = \"warn\"\nmax_file = 10\n",
        )
        .unwrap();
        fs::write(
            &child,
            "version = 2\nextends = [\"base.toml\"]\n[rules.\"shape.file_complexity\"]\nmax_fn = 2\n",
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
}
