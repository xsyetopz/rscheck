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
pub(crate) struct ToolCommand {
    program: String,
    prefix_args: Vec<String>,
}

impl ToolCommand {
    fn direct(program: &str) -> Self {
        Self {
            program: program.to_string(),
            prefix_args: Vec::new(),
        }
    }

    fn with_selector(program: &str, selector: &str) -> Self {
        Self {
            program: program.to_string(),
            prefix_args: vec![format!("+{selector}")],
        }
    }

    fn rustup_run(selector: &str, program: &str) -> Self {
        Self {
            program: "rustup".to_string(),
            prefix_args: vec!["run".to_string(), selector.to_string(), program.to_string()],
        }
    }

    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.prefix_args);
        command
    }
}

#[derive(Debug, Clone)]
struct ToolchainProbe {
    runtime_label: String,
    cargo: ToolCommand,
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

    pub(crate) fn clippy_command(
        &self,
        mode: AdapterToolchainMode,
    ) -> Result<ToolCommand, ToolchainError> {
        Ok(self.clippy_probe(mode)?.cargo.clone())
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
            AdapterToolchainMode::Stable => Ok(&self.current),
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
        ToolchainMode::Stable => Ok(current.clone()),
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
    let (cargo, rustc) = resolve_tool_commands(selector, runtime_label)?;
    let channel = probe_rustc_channel(&rustc, runtime_label)?;
    Ok(ToolchainProbe {
        runtime_label: runtime_label.to_string(),
        cargo,
        channel,
    })
}

fn resolve_tool_commands(
    selector: Option<&str>,
    runtime_label: &str,
) -> Result<(ToolCommand, ToolCommand), ToolchainError> {
    let Some(selector) = selector else {
        let cargo = ToolCommand::direct("cargo");
        verify_tool_command(&cargo, runtime_label)?;
        return Ok((cargo, ToolCommand::direct("rustc")));
    };

    let cargo_selector = ToolCommand::with_selector("cargo", selector);
    if verify_tool_command(&cargo_selector, runtime_label).is_ok() {
        return Ok((
            cargo_selector,
            ToolCommand::with_selector("rustc", selector),
        ));
    }

    let cargo_rustup = ToolCommand::rustup_run(selector, "cargo");
    verify_tool_command(&cargo_rustup, runtime_label)?;
    Ok((cargo_rustup, ToolCommand::rustup_run(selector, "rustc")))
}

fn verify_tool_command(command: &ToolCommand, runtime_label: &str) -> Result<(), ToolchainError> {
    let mut command = command.command();
    command.arg("-V");
    run_probe(command, runtime_label)
}

fn probe_rustc_channel(
    rustc: &ToolCommand,
    runtime_label: &str,
) -> Result<RustcChannel, ToolchainError> {
    let mut command = rustc.command();
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
        ToolchainMode::Stable => "stable",
        ToolchainMode::Auto => "auto",
        ToolchainMode::Nightly => "nightly",
    }
}

#[cfg(test)]
mod tests;
