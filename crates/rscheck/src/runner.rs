use crate::analysis::Workspace;
use crate::config::{EngineMode, Policy};
use crate::emit::ReportEmitter;
use crate::report::{AdapterRun, Report};
use crate::rules::{self, RuleBackend, RuleContext};
use crate::semantic::SemanticBackendStatus;

pub struct Runner;

impl Runner {
    pub fn run(ws: &Workspace, policy: &Policy) -> Result<Report, RunError> {
        let mut report = Report {
            rule_catalog: rules::rule_catalog_entries(),
            ..Report::default()
        };

        let semantic_status = SemanticBackendStatus::probe();
        if policy.engine.semantic == EngineMode::Require && !semantic_status.is_available() {
            return Err(RunError::SemanticBackendRequired(
                semantic_status
                    .reason
                    .unwrap_or_else(|| "semantic backend unavailable".to_string()),
            ));
        }

        let mut emitter = ReportEmitter::new();
        let ctx = RuleContext { policy };
        for rule in rules::enabled_rules(policy) {
            let info = rule.info();
            if info.backend == RuleBackend::Semantic && !semantic_status.is_available() {
                report.summary.skipped_rules.push(info.id.to_string());
                continue;
            }
            if !report.summary.engine_used.contains(&info.backend) {
                report.summary.engine_used.push(info.backend);
            }
            rule.run(ws, &ctx, &mut emitter);
        }

        report.findings = emitter.findings;
        report.metrics.per_file = emitter.metrics;
        report.summary.semantic_backend_available = semantic_status.is_available();
        report.summary.adapter_runs.push(AdapterRun {
            name: "clippy".to_string(),
            enabled: policy.adapters.clippy.enabled,
        });
        Ok(report)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("{0}")]
    SemanticBackendRequired(String),
}
