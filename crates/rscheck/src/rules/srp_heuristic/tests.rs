use super::SrpHeuristicRule;
use crate::analysis::{SourceFile, Workspace};
use crate::config::{Config, Level, SrpHeuristicConfig};
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
fn flags_large_impl_block() {
    let ws = ws_with_single_file(
        r#"
struct S;
impl S {
    fn a(&self) {}
    fn b(&self) {}
    fn c(&self) {}
}
"#,
    );

    let cfg = Config::default();
    let mut emitter = ReportEmitter::new();
    SrpHeuristicRule::new(SrpHeuristicConfig {
        level: Level::Warn,
        method_count_threshold: 2,
    })
    .run(&ws, &cfg, &mut emitter);

    assert_eq!(emitter.findings.len(), 1);
}
