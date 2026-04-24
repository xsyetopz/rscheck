use super::{
    RustcChannel, ToolCommand, ToolchainProbe, parse_channel, semantic_status_for_probe,
    stderr_reason,
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
        cargo: super::ToolCommand::direct("cargo"),
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

#[test]
fn tool_command_selector_uses_cargo_proxy_shape() {
    let command = ToolCommand::with_selector("cargo", "nightly");

    assert_eq!(command.program, "cargo");
    assert_eq!(command.prefix_args, vec!["+nightly"]);
}

#[test]
fn tool_command_rustup_run_uses_non_proxy_fallback_shape() {
    let command = ToolCommand::rustup_run("nightly", "cargo");

    assert_eq!(command.program, "rustup");
    assert_eq!(command.prefix_args, vec!["run", "nightly", "cargo"]);
}
