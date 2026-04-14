use crate::report::{FileMetrics, Finding};

pub trait Emitter {
    fn emit(&mut self, finding: Finding);

    fn record_metrics(&mut self, _metrics: FileMetrics) {}
}

#[derive(Default)]
pub struct ReportEmitter {
    pub findings: Vec<Finding>,
    pub metrics: Vec<FileMetrics>,
}

impl ReportEmitter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            findings: Vec::new(),
            metrics: Vec::new(),
        }
    }
}

impl Emitter for ReportEmitter {
    fn emit(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    fn record_metrics(&mut self, metrics: FileMetrics) {
        self.metrics.push(metrics);
    }
}
