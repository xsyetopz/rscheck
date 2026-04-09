use super::AbsoluteFilesystemPathsRule;
use crate::analysis::{SourceFile, Workspace};
use crate::config::AbsoluteFilesystemPathsConfig;
use crate::config::Config;
use crate::emit::ReportEmitter;
use crate::rules::Rule;
use std::path::PathBuf;

fn ws_with_single_file(code: &str) -> Workspace {
    let root = PathBuf::from(".");
    let path = root.join("rscheck_test.rs");
    let ast = syn::parse_file(code).ok();
    Workspace {
        root,
        files: vec![SourceFile {
            path,
            text: code.to_string(),
            ast,
            parse_error: None,
        }],
    }
}

#[test]
fn flags_unix_absolute_path_in_literal() {
    let ws = ws_with_single_file(
        r#"
fn f() {
    let _p = "/etc/passwd";
    let _q = "rel/path";
}
"#,
    );

    let cfg = Config::default();
    let mut emitter = ReportEmitter::new();
    AbsoluteFilesystemPathsRule::new(AbsoluteFilesystemPathsConfig::default()).run(
        &ws,
        &cfg,
        &mut emitter,
    );

    assert_eq!(emitter.findings.len(), 1);
    assert!(emitter.findings[0].message.contains("/etc/passwd"));
}

#[test]
fn does_not_flag_comment_marker_string_literals() {
    let ws = ws_with_single_file(
        r#"
fn f(line: &str) -> bool {
    line.trim_start().starts_with("//!") || line.trim_start().starts_with("/*!")
}
"#,
    );

    let cfg = Config::default();
    let mut emitter = ReportEmitter::new();
    AbsoluteFilesystemPathsRule::new(AbsoluteFilesystemPathsConfig::default()).run(
        &ws,
        &cfg,
        &mut emitter,
    );

    assert!(emitter.findings.is_empty());
}
