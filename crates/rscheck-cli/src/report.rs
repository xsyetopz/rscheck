use serde::{Deserialize, Serialize};

use crate::config::Level;
use crate::rules::{RuleBackend, RuleFamily};
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Deny,
}

impl Severity {
    #[must_use]
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Info => 0,
            Self::Warn => 1,
            Self::Deny => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixSafety {
    Safe,
    Unsafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingLabelKind {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingLabel {
    pub kind: FindingLabelKind,
    pub span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingNoteKind {
    Note,
    Help,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingNote {
    pub kind: FindingNoteKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub file: String,
    pub byte_start: u32,
    pub byte_end: u32,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    pub id: String,
    pub safety: FixSafety,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<TextEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<RuleFamily>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<RuleBackend>,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<Span>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary: Vec<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<FindingLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<FindingNote>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixes: Vec<Fix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCatalogEntry {
    pub id: String,
    pub family: RuleFamily,
    pub backend: RuleBackend,
    pub default_level: Level,
    pub summary: String,
    #[serde(default)]
    pub fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterRun {
    pub name: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainSummary {
    pub requested: String,
    pub resolved: String,
    pub semantic: String,
    #[serde(default)]
    pub nightly_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunSummary {
    #[serde(default)]
    pub engine_used: Vec<RuleBackend>,
    #[serde(default)]
    pub semantic_backend_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_backend_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<ToolchainSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_rules: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapter_runs: Vec<AdapterRun>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    #[serde(default)]
    pub per_file: Vec<FileMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetrics {
    pub path: String,
    #[serde(default)]
    pub cyclomatic_sum: u32,
    #[serde(default)]
    pub cyclomatic_max_fn: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Report {
    #[serde(default)]
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub metrics: Metrics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_catalog: Vec<RuleCatalogEntry>,
    #[serde(default)]
    pub summary: RunSummary,
}

impl Report {
    pub fn worst_severity(&self) -> Severity {
        self.findings
            .iter()
            .map(|f| f.severity)
            .max_by_key(|s| s.exit_code())
            .unwrap_or(Severity::Info)
    }
}
