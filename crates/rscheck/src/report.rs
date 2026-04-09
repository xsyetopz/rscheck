use serde::{Deserialize, Serialize};

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixes: Vec<Fix>,
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
