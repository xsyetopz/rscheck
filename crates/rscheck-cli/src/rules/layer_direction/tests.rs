use crate::analysis::Workspace;
use crate::config::{Level, Policy, RuleSettings};
use crate::emit::ReportEmitter;
use crate::rules::{LayerDirectionRule, Rule, RuleContext};
use std::fs;

#[test]
fn flags_layer_violation() {
    let dir = tempfile::tempdir().unwrap();
    let api_dir = dir.path().join("src/api");
    let domain_dir = dir.path().join("src/domain");
    fs::create_dir_all(&api_dir).unwrap();
    fs::create_dir_all(&domain_dir).unwrap();
    fs::write(api_dir.join("mod.rs"), "use crate::infra::db::Client;\n").unwrap();
    fs::create_dir_all(dir.path().join("src/infra/db")).unwrap();
    fs::write(domain_dir.join("mod.rs"), "pub struct Domain;\n").unwrap();
    fs::write(
        dir.path().join("src/infra/db/mod.rs"),
        "pub struct Client;\n",
    )
    .unwrap();

    let ws = Workspace::new(dir.path().to_path_buf())
        .load_files(&Policy::default())
        .unwrap();
    let mut policy = Policy::default();
    policy.rules.insert(
        "architecture.layer_direction".to_string(),
        RuleSettings {
            level: Some(Level::Deny),
            options: toml::toml! {
                layers = [
                    { name = "api", include = ["**/src/api/**"], may_depend_on = ["domain"] },
                    { name = "domain", include = ["**/src/domain/**"], may_depend_on = [] },
                    { name = "infra", include = ["**/src/infra/**"], may_depend_on = ["domain"] }
                ]
            },
        },
    );

    let mut emitter = ReportEmitter::new();
    LayerDirectionRule.run(&ws, &RuleContext { policy: &policy }, &mut emitter);
    assert_eq!(emitter.findings.len(), 1);
}
