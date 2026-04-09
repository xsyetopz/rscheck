use serde::{Deserialize, Serialize};

use crate::report::Severity;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsoluteModulePathsConfig {
    #[serde(default = "AbsoluteModulePathsConfig::default_level")]
    pub level: Level,
    #[serde(default)]
    pub allow_prefixes: Vec<String>,
}

impl AbsoluteModulePathsConfig {
    fn default_level() -> Level {
        Level::Deny
    }
}

impl Default for AbsoluteModulePathsConfig {
    fn default() -> Self {
        Self {
            level: Level::Deny,
            allow_prefixes: Vec::new(),
        }
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
            level: Level::Warn,
            allow_globs: Vec::new(),
            allow_regex: Vec::new(),
            check_comments: false,
        }
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
            level: Level::Warn,
            mode: ComplexityMode::Cyclomatic,
            max_file: 200,
            max_fn: 25,
            count_question: false,
            match_arms: true,
        }
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
            level: Level::Warn,
            min_tokens: 80,
            threshold: 0.80,
            max_results: 200,
            exclude_globs: Vec::new(),
            kgram: 25,
        }
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
            level: Level::Allow,
            min_occurrences: 3,
            min_len: 25,
            exclude_outer: vec!["Option".to_string()],
        }
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulesConfig {
    #[serde(default)]
    pub absolute_module_paths: AbsoluteModulePathsConfig,
    #[serde(default)]
    pub absolute_filesystem_paths: AbsoluteFilesystemPathsConfig,
    #[serde(default)]
    pub file_complexity: FileComplexityConfig,
    #[serde(default)]
    pub duplicate_logic: DuplicateLogicConfig,
    #[serde(default)]
    pub duplicate_types_alias_candidate: DuplicateTypesAliasConfig,
    #[serde(default)]
    pub srp_heuristic: SrpHeuristicConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub rules: RulesConfig,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rules: RulesConfig::default(),
            include: vec!["**/*.rs".to_string()],
            exclude: vec!["target/**".to_string(), ".git/**".to_string()],
        }
    }
}
