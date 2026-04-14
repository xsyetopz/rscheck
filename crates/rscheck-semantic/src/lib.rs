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
    pub runtime: String,
    pub reason: Option<String>,
}

impl SemanticBackendStatus {
    #[must_use]
    pub fn available(runtime: &str) -> Self {
        Self {
            availability: SemanticBackendAvailability::Available,
            runtime: runtime.to_string(),
            reason: None,
        }
    }

    #[must_use]
    pub fn unavailable(runtime: &str, reason: impl Into<String>) -> Self {
        Self {
            availability: SemanticBackendAvailability::Unavailable,
            runtime: runtime.to_string(),
            reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn probe() -> Self {
        Self::probe_for_runtime("current")
    }

    #[must_use]
    pub fn probe_for_runtime(runtime: &str) -> Self {
        #[cfg(feature = "nightly")]
        {
            Self {
                availability: SemanticBackendAvailability::Available,
                runtime: runtime.to_string(),
                reason: None,
            }
        }

        #[cfg(not(feature = "nightly"))]
        {
            Self {
                availability: SemanticBackendAvailability::Unavailable,
                runtime: runtime.to_string(),
                reason: Some(format!(
                    "semantic backend unavailable on `{runtime}` because rscheck-semantic was built without the `nightly` feature"
                )),
            }
        }
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        self.availability == SemanticBackendAvailability::Available
    }
}
