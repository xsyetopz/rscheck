use super::AbsoluteModulePathsRule;
use crate::analysis::{SourceFile, Workspace};
use crate::config::{AbsoluteModulePathsConfig, Config};
use crate::emit::ReportEmitter;
use crate::fix::apply_text_edits;
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
fn flags_absolute_module_paths_and_allows_crate_root_specials() {
    let ws = ws_with_single_file(
        r#"
use ::foo::bar;

const MY_CONST: i32 = 1;

fn f() {
    let _ = ::std::mem::size_of::<u8>();
    let _p: &std::path::Path = std::path::Path::new("x");
    let _ = crate::MY_CONST;
    crate::static_function();
    crate::my_macro!();
}
"#,
    );

    let cfg = Config::default();
    let mut emitter = ReportEmitter::new();
    AbsoluteModulePathsRule::new(AbsoluteModulePathsConfig::default()).run(&ws, &cfg, &mut emitter);

    assert!(emitter.findings.iter().any(|f| f.message.contains("::std")));
    assert!(
        emitter
            .findings
            .iter()
            .any(|f| f.message.contains("std::path::Path"))
    );

    assert!(
        !emitter
            .findings
            .iter()
            .any(|f| f.message.contains("crate::MY_CONST"))
    );
    assert!(
        !emitter
            .findings
            .iter()
            .any(|f| f.message.contains("crate::static_function"))
    );
    assert!(
        !emitter
            .findings
            .iter()
            .any(|f| f.message.contains("crate::my_macro"))
    );
}

#[test]
fn emits_safe_fix_for_std_type_path() {
    let code = r#"
fn f() {
    let _p: std::path::PathBuf = std::path::PathBuf::from(".");
}
"#;
    let ws = ws_with_single_file(code);

    let cfg = Config::default();
    let mut emitter = ReportEmitter::new();
    AbsoluteModulePathsRule::new(AbsoluteModulePathsConfig::default()).run(&ws, &cfg, &mut emitter);

    let finding = emitter
        .findings
        .iter()
        .find(|f| f.message.contains("std::path::PathBuf"))
        .expect("expected PathBuf finding");
    assert!(!finding.fixes.is_empty());

    let fix = &finding.fixes[0];
    let new_code = apply_text_edits(code, &fix.edits).expect("apply edits");
    assert!(new_code.contains("use std::path::PathBuf;"));
    assert!(new_code.contains("let _p: PathBuf"));
    assert!(new_code.contains("PathBuf::from"));
}

