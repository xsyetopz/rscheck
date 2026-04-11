use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticBackendAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticBackendStatus {
    pub availability: SemanticBackendAvailability,
    pub reason: Option<String>,
}

impl SemanticBackendStatus {
    #[must_use]
    pub fn probe() -> Self {
        #[cfg(feature = "nightly")]
        {
            Self {
                availability: SemanticBackendAvailability::Available,
                reason: None,
            }
        }

        #[cfg(not(feature = "nightly"))]
        {
            Self {
                availability: SemanticBackendAvailability::Unavailable,
                reason: Some("semantic backend not built with the `nightly` feature".to_string()),
            }
        }
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        self.availability == SemanticBackendAvailability::Available
    }
}
