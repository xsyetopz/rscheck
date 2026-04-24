use serde::{Deserialize, Serialize};

use crate::config::Level;
use crate::rules::{RuleBackend, RuleFamily, RuleInfo};
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
    #[serde(flatten)]
    pub identity: FindingIdentity,
    #[serde(flatten)]
    pub diagnostic: FindingDiagnostic,
    #[serde(flatten)]
    pub location: FindingLocation,
    #[serde(flatten)]
    pub metadata: FindingMetadata,
    #[serde(flatten)]
    pub related: FindingRelated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingIdentity {
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<RuleFamily>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<RuleBackend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingDiagnostic {
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindingLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<Span>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary: Vec<Span>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindingMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindingRelated {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<FindingLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<FindingNote>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixes: Vec<Fix>,
}

impl Finding {
    pub fn new(rule_id: String, severity: Severity, message: String) -> Self {
        Self {
            identity: FindingIdentity {
                rule_id,
                family: None,
                engine: None,
            },
            diagnostic: FindingDiagnostic {
                severity,
                message,
                help: None,
            },
            location: FindingLocation::default(),
            metadata: FindingMetadata::default(),
            related: FindingRelated::default(),
        }
    }

    pub fn from_rule(rule_info: RuleInfo, severity: Severity, message: String) -> Self {
        Self::new(rule_info.id.to_string(), severity, message)
            .with_engine(rule_info.family, rule_info.backend)
    }

    #[must_use]
    pub fn with_engine(mut self, family: RuleFamily, backend: RuleBackend) -> Self {
        self.identity.family = Some(family);
        self.identity.engine = Some(backend);
        self
    }

    #[must_use]
    pub fn with_backend(mut self, backend: RuleBackend) -> Self {
        self.identity.engine = Some(backend);
        self
    }

    #[must_use]
    pub fn with_primary(mut self, primary: Span) -> Self {
        self.location.primary = Some(primary);
        self
    }

    #[must_use]
    pub fn with_secondary(mut self, secondary: Vec<Span>) -> Self {
        self.location.secondary = secondary;
        self
    }

    #[must_use]
    pub fn with_help(mut self, help: String) -> Self {
        self.diagnostic.help = Some(help);
        self
    }

    #[must_use]
    pub fn with_evidence(mut self, evidence: String) -> Self {
        self.metadata.evidence = Some(evidence);
        self
    }

    #[must_use]
    pub fn with_confidence(mut self, confidence: String) -> Self {
        self.metadata.confidence = Some(confidence);
        self
    }

    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.metadata.tags = tags;
        self
    }

    #[must_use]
    pub fn with_labels(mut self, labels: Vec<FindingLabel>) -> Self {
        self.related.labels = labels;
        self
    }

    #[must_use]
    pub fn with_notes(mut self, notes: Vec<FindingNote>) -> Self {
        self.related.notes = notes;
        self
    }

    #[must_use]
    pub fn with_fixes(mut self, fixes: Vec<Fix>) -> Self {
        self.related.fixes = fixes;
        self
    }
}

impl Finding {
    #[must_use]
    pub fn rule_id(&self) -> &str {
        &self.identity.rule_id
    }

    #[must_use]
    pub fn severity(&self) -> Severity {
        self.diagnostic.severity
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.diagnostic.message
    }

    #[must_use]
    pub fn primary(&self) -> Option<&Span> {
        self.location.primary.as_ref()
    }

    #[must_use]
    pub fn secondary(&self) -> &[Span] {
        &self.location.secondary
    }

    #[must_use]
    pub fn help(&self) -> Option<&str> {
        self.diagnostic.help.as_deref()
    }

    #[must_use]
    pub fn evidence(&self) -> Option<&str> {
        self.metadata.evidence.as_deref()
    }

    #[must_use]
    pub fn labels(&self) -> &[FindingLabel] {
        &self.related.labels
    }

    #[must_use]
    pub fn notes(&self) -> &[FindingNote] {
        &self.related.notes
    }

    #[must_use]
    pub fn fixes(&self) -> &[Fix] {
        &self.related.fixes
    }
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
            .map(Finding::severity)
            .max_by_key(|s| s.exit_code())
            .unwrap_or(Severity::Info)
    }
}
