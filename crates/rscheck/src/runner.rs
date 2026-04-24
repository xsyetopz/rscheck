use crate::analysis::Workspace;
use crate::config::{EngineMode, Policy};
use crate::emit::ReportEmitter;
use crate::report::{AdapterRun, Report};
use crate::rules::{self, RuleBackend, RuleContext};
use crate::semantic::SemanticBackendStatus;

pub struct Runner;

impl Runner {
    pub fn run(ws: &Workspace, policy: &Policy) -> Result<Report, RunError> {
        let semantic_status = SemanticBackendStatus::probe();
        Self::run_with_semantic_status(ws, policy, semantic_status)
    }

    pub fn run_with_semantic_status(
        ws: &Workspace,
        policy: &Policy,
        semantic_status: SemanticBackendStatus,
    ) -> Result<Report, RunError> {
        let mut report = Report {
            rule_catalog: rules::rule_catalog_entries(),
            ..Report::default()
        };

        if policy.engine.semantic == EngineMode::Require && !semantic_status.is_available() {
            return Err(RunError::SemanticBackendRequired(
                semantic_status
                    .reason
                    .clone()
                    .unwrap_or_else(|| "semantic backend unavailable".to_string()),
            ));
        }

        let mut emitter = ReportEmitter::new();
        let ctx = RuleContext { policy };
        for rule in rules::enabled_rules(policy) {
            let rule_info = rule.info();
            if rule_info.backend == RuleBackend::Semantic && !semantic_status.is_available() {
                report.summary.skipped_rules.push(rule_id(rule_info.id));
                continue;
            }
            if !report.summary.engine_used.contains(&rule_info.backend) {
                report.summary.engine_used.push(rule_info.backend);
            }
            rule.run(ws, &ctx, &mut emitter);
        }

        report.findings = emitter.findings;
        report.metrics.per_file = emitter.metrics;
        report.summary.semantic_backend_available = semantic_status.is_available();
        report.summary.semantic_backend_reason = semantic_status.reason;
        report.summary.adapter_runs.push(AdapterRun {
            name: "clippy".to_string(),
            enabled: policy.adapters.clippy.enabled,
            toolchain: None,
            status: None,
        });
        Ok(report)
    }
}

fn rule_id(id: &str) -> String {
    id.to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("{0}")]
    SemanticBackendRequired(String),
}
