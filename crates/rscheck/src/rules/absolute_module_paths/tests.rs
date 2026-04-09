use super::AbsoluteModulePathsRule;
use crate::analysis::{SourceFile, Workspace};
use crate::config::AbsoluteModulePathsConfig;
use crate::config::Config;
use crate::emit::ReportEmitter;
use crate::rules::Rule;

fn ws_with_single_file(code: &str) -> Workspace {
    let root = std::path::PathBuf::from(".");
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
fn flags_leading_colon() {
    let ws = ws_with_single_file(
        r#"
use ::foo::bar;

fn f() {
    let _ = ::std::mem::size_of::<u8>();
}
"#,
    );

    let cfg = Config::default();
    let mut emitter = ReportEmitter::new();
    AbsoluteModulePathsRule::new(AbsoluteModulePathsConfig::default()).run(&ws, &cfg, &mut emitter);

    assert!(emitter.findings.len() >= 2);
}
