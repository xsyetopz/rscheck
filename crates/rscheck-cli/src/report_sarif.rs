use rscheck::report::{Finding, Report, Severity};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifLog {
    pub version: &'static str,
    pub schema: &'static str,
    pub runs: Vec<Run>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub tool: Tool,
    pub results: Vec<ResultItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub driver: Driver,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Driver {
    pub name: &'static str,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultItem {
    pub rule_id: String,
    pub level: &'static str,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<Location>>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub physical_location: PhysicalLocation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalLocation {
    pub artifact_location: ArtifactLocation,
    pub region: Region,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

pub fn to_sarif(report: &Report) -> SarifLog {
    let rules = if report.rule_catalog.is_empty() {
        let mut unique_rules: BTreeSet<String> = BTreeSet::new();
        for f in &report.findings {
            unique_rules.insert(String::from(f.rule_id()));
        }
        unique_rules
            .iter()
            .map(|id| Rule {
                id: id.clone(),
                name: id.clone(),
            })
            .collect::<Vec<_>>()
    } else {
        report
            .rule_catalog
            .iter()
            .map(|entry| Rule {
                id: entry.id.clone(),
                name: entry.summary.clone(),
            })
            .collect::<Vec<_>>()
    };

    SarifLog {
        version: "2.1.0",
        schema: "https://json.schemastore.org/sarif-2.1.0.json",
        runs: vec![Run {
            tool: Tool {
                driver: Driver {
                    name: "rscheck",
                    rules,
                },
            },
            results: report.findings.iter().map(finding_to_result).collect(),
        }],
    }
}

fn finding_to_result(f: &Finding) -> ResultItem {
    ResultItem {
        rule_id: String::from(f.rule_id()),
        level: match f.severity() {
            Severity::Info => "note",
            Severity::Warn => "warning",
            Severity::Deny => "error",
        },
        message: Message {
            text: String::from(f.message()),
        },
        locations: f.primary().map(|span| {
            Vec::from([Location {
                physical_location: PhysicalLocation {
                    artifact_location: ArtifactLocation {
                        uri: Clone::clone(&span.file),
                    },
                    region: Region {
                        start_line: span.start.line,
                        start_column: span.start.column,
                        end_line: span.end.line,
                        end_column: span.end.column,
                    },
                },
            }])
        }),
    }
}
