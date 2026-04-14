use rscheck::config::{AdapterToolchainMode, EngineMode, Policy, ToolchainMode};
use rscheck::report::ToolchainSummary;
use rscheck::semantic::SemanticBackendStatus;
use std::io;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustcChannel {
    Stable,
    Beta,
    Nightly,
    Dev,
    Unknown,
}

#[derive(Debug, Clone)]
struct ToolchainProbe {
    runtime_label: String,
    cargo_selector: Option<String>,
    channel: RustcChannel,
}

#[derive(Debug, Clone)]
pub struct ResolvedToolchain {
    current: ToolchainProbe,
    nightly: Option<ToolchainProbe>,
    semantic: ToolchainProbe,
    summary: ToolchainSummary,
}

impl ResolvedToolchain {
    pub fn semantic_status(&self) -> SemanticBackendStatus {
        semantic_status_for_probe(&self.semantic)
    }

    pub fn summary(&self) -> ToolchainSummary {
        self.summary.clone()
    }

    pub fn clippy_selector(
        &self,
        mode: AdapterToolchainMode,
    ) -> Result<Option<&str>, ToolchainError> {
        Ok(self.clippy_probe(mode)?.cargo_selector.as_deref())
    }

    pub fn clippy_runtime_label(
        &self,
        mode: AdapterToolchainMode,
    ) -> Result<String, ToolchainError> {
        Ok(self.clippy_probe(mode)?.runtime_label.clone())
    }

    fn clippy_probe(&self, mode: AdapterToolchainMode) -> Result<&ToolchainProbe, ToolchainError> {
        match mode {
            AdapterToolchainMode::Inherit => Ok(&self.semantic),
            AdapterToolchainMode::Current => Ok(&self.current),
            AdapterToolchainMode::Nightly => self.nightly.as_ref().ok_or_else(|| {
                ToolchainError::NightlyUnavailable("nightly cargo is unavailable".to_string())
            }),
            AdapterToolchainMode::Auto => Ok(self.nightly.as_ref().unwrap_or(&self.current)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolchainError {
    #[error("failed to probe toolchain command")]
    Probe(#[source] io::Error),
    #[error("toolchain probe failed for `{runtime}`: {reason}")]
    ProbeStatus { runtime: String, reason: String },
    #[error("{0}")]
    NightlyUnavailable(String),
}

pub fn resolve_toolchain(
    policy: &Policy,
    cli_override: Option<ToolchainMode>,
) -> Result<ResolvedToolchain, ToolchainError> {
    let requested = cli_override.unwrap_or(policy.engine.toolchain);
    let current = probe_current_toolchain()?;
    let nightly = probe_nightly_toolchain(&policy.engine.nightly_toolchain);
    let semantic = select_semantic_probe(
        requested,
        policy.engine.semantic,
        &current,
        nightly.as_ref().ok(),
    )?;
    let semantic_status = semantic_status_for_probe(&semantic);
    let nightly_available = nightly.is_ok();
    let nightly_probe = nightly.ok();

    Ok(ResolvedToolchain {
        current,
        nightly: nightly_probe,
        summary: ToolchainSummary {
            requested: toolchain_mode_name(requested).to_string(),
            resolved: semantic.runtime_label.clone(),
            semantic: if semantic_status.is_available() {
                "available".to_string()
            } else {
                "unavailable".to_string()
            },
            nightly_available,
            reason: semantic_status.reason.clone(),
        },
        semantic,
    })
}

fn select_semantic_probe(
    requested: ToolchainMode,
    semantic_mode: EngineMode,
    current: &ToolchainProbe,
    nightly: Option<&ToolchainProbe>,
) -> Result<ToolchainProbe, ToolchainError> {
    match requested {
        ToolchainMode::Current => Ok(current.clone()),
        ToolchainMode::Nightly => nightly.cloned().ok_or_else(|| {
            ToolchainError::NightlyUnavailable(
                "nightly toolchain was requested but cargo cannot run it".to_string(),
            )
        }),
        ToolchainMode::Auto => {
            if semantic_mode == EngineMode::Off {
                return Ok(current.clone());
            }
            Ok(nightly.cloned().unwrap_or_else(|| current.clone()))
        }
    }
}

fn probe_current_toolchain() -> Result<ToolchainProbe, ToolchainError> {
    probe_toolchain(None, "current")
}

fn probe_nightly_toolchain(name: &str) -> Result<ToolchainProbe, ToolchainError> {
    probe_toolchain(Some(name), name)
}

fn probe_toolchain(
    selector: Option<&str>,
    runtime_label: &str,
) -> Result<ToolchainProbe, ToolchainError> {
    verify_cargo_selector(selector, runtime_label)?;
    let channel = probe_rustc_channel(selector, runtime_label)?;
    Ok(ToolchainProbe {
        runtime_label: runtime_label.to_string(),
        cargo_selector: selector.map(|value| format!("+{value}")),
        channel,
    })
}

fn verify_cargo_selector(
    selector: Option<&str>,
    runtime_label: &str,
) -> Result<(), ToolchainError> {
    let mut command = Command::new("cargo");
    if let Some(selector) = selector {
        command.arg(format!("+{selector}"));
    }
    command.arg("-V");
    run_probe(command, runtime_label)
}

fn probe_rustc_channel(
    selector: Option<&str>,
    runtime_label: &str,
) -> Result<RustcChannel, ToolchainError> {
    let mut command = Command::new("rustc");
    if let Some(selector) = selector {
        command.arg(format!("+{selector}"));
    }
    command.arg("-vV");
    let stdout = run_probe_capture(command, runtime_label)?;
    Ok(parse_channel(&stdout))
}

fn run_probe(mut command: Command, runtime_label: &str) -> Result<(), ToolchainError> {
    let output = command.output().map_err(ToolchainError::Probe)?;
    if output.status.success() {
        return Ok(());
    }

    Err(ToolchainError::ProbeStatus {
        runtime: runtime_label.to_string(),
        reason: stderr_reason(&output.stderr),
    })
}

fn run_probe_capture(mut command: Command, runtime_label: &str) -> Result<String, ToolchainError> {
    let output = command.output().map_err(ToolchainError::Probe)?;
    if !output.status.success() {
        return Err(ToolchainError::ProbeStatus {
            runtime: runtime_label.to_string(),
            reason: stderr_reason(&output.stderr),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn stderr_reason(stderr: &[u8]) -> String {
    let trimmed = String::from_utf8_lossy(stderr).trim().to_string();
    if trimmed.is_empty() {
        "command exited unsuccessfully".to_string()
    } else {
        trimmed
    }
}

fn parse_channel(stdout: &str) -> RustcChannel {
    let Some(release_line) = stdout.lines().find(|line| line.starts_with("release: ")) else {
        return RustcChannel::Unknown;
    };
    let release = release_line.trim_start_matches("release: ");
    if release.contains("nightly") {
        RustcChannel::Nightly
    } else if release.contains("beta") {
        RustcChannel::Beta
    } else if release.contains("dev") {
        RustcChannel::Dev
    } else if release.contains('.') {
        RustcChannel::Stable
    } else {
        RustcChannel::Unknown
    }
}

fn semantic_status_for_probe(probe: &ToolchainProbe) -> SemanticBackendStatus {
    if probe.channel != RustcChannel::Nightly {
        return SemanticBackendStatus::unavailable(
            &probe.runtime_label,
            format!(
                "semantic backend requires nightly rustc; resolved toolchain `{}` is not nightly",
                probe.runtime_label
            ),
        );
    }

    let compiled_status = SemanticBackendStatus::probe_for_runtime(&probe.runtime_label);
    if compiled_status.is_available() {
        SemanticBackendStatus::available(&probe.runtime_label)
    } else {
        compiled_status
    }
}

fn toolchain_mode_name(mode: ToolchainMode) -> &'static str {
    match mode {
        ToolchainMode::Current => "current",
        ToolchainMode::Auto => "auto",
        ToolchainMode::Nightly => "nightly",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RustcChannel, ToolchainProbe, parse_channel, semantic_status_for_probe, stderr_reason,
    };

    #[test]
    fn parses_nightly_release() {
        let channel = parse_channel("release: 1.92.0-nightly\n");
        assert_eq!(channel, RustcChannel::Nightly);
    }

    #[test]
    fn semantic_status_requires_nightly_runtime() {
        let probe = ToolchainProbe {
            runtime_label: "current".to_string(),
            cargo_selector: None,
            channel: RustcChannel::Stable,
        };
        let status = semantic_status_for_probe(&probe);
        assert!(!status.is_available());
        assert!(
            status
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("requires nightly rustc"))
        );
    }

    #[test]
    fn keeps_stderr_reason_text() {
        let reason = stderr_reason(b"error: no such command: `+nightly`\n");
        assert!(reason.contains("no such command"));
    }
}
