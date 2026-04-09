use serde::{Deserialize, Serialize};
use std::path::Path;

use proc_macro2::Span as PmSpan;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    pub start: Location,
    pub end: Location,
}

impl Span {
    #[must_use]
    pub fn new(file: &Path, start: Location, end: Location) -> Self {
        Self {
            file: file.to_string_lossy().to_string(),
            start,
            end,
        }
    }

    #[must_use]
    pub fn from_pm_span(file: &Path, span: PmSpan) -> Self {
        let start = span.start();
        let end = span.end();
        Self::new(
            file,
            Location {
                line: start.line as u32,
                column: (start.column as u32).saturating_add(1),
            },
            Location {
                line: end.line as u32,
                column: (end.column as u32).saturating_add(1),
            },
        )
    }
}
