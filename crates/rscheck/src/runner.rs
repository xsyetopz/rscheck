use crate::analysis::Workspace;
use crate::config::Config;
use crate::emit::ReportEmitter;
use crate::report::Report;
use crate::rules;

pub struct Runner;

impl Runner {
    #[must_use]
    pub fn run(ws: &Workspace, config: &Config) -> Report {
        let mut report = Report::default();

        let mut emitter = ReportEmitter::new();
        for rule in rules::enabled_rules(config) {
            rule.run(ws, config, &mut emitter);
        }

        report.findings = emitter.findings;
        report.metrics.per_file = emitter.metrics;
        report
    }
}
