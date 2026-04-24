use crate::config::Level;
use crate::rules::HotPathAllocationsRule;
use crate::test_support::run_single_file_rule;

#[test]
fn test_hot_path_allocations_clone_inside_loop_returns_warning() {
    let findings = run_single_file_rule(
        &HotPathAllocationsRule,
        HotPathAllocationsRule::static_info().id,
        Level::Warn,
        toml::Table::new(),
        "fn demo(xs: Vec<String>) { for x in xs { let y = x.clone(); drop(y); } }
",
    );

    assert_eq!(findings.len(), 1);
    assert!(findings[0].message().contains("clone"));
}

#[test]
fn test_hot_path_allocations_clone_outside_loop_returns_no_findings() {
    let findings = run_single_file_rule(
        &HotPathAllocationsRule,
        HotPathAllocationsRule::static_info().id,
        Level::Warn,
        toml::Table::new(),
        "fn demo(x: String) { let y = x.clone(); drop(y); }
",
    );

    assert!(findings.is_empty());
}
