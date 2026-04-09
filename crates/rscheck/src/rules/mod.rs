mod absolute_filesystem_paths;
mod absolute_module_paths;
mod duplicate_logic;
mod duplicate_types_alias;
mod file_complexity;
mod srp_heuristic;

use crate::analysis::Workspace;
use crate::config::Config;
use crate::emit::Emitter;

pub use absolute_filesystem_paths::AbsoluteFilesystemPathsRule;
pub use absolute_module_paths::AbsoluteModulePathsRule;
pub use duplicate_logic::DuplicateLogicRule;
pub use duplicate_types_alias::DuplicateTypesAliasCandidateRule;
pub use file_complexity::FileComplexityRule;
pub use srp_heuristic::SrpHeuristicRule;

#[derive(Debug, Clone)]
pub struct RuleInfo {
    pub id: &'static str,
    pub summary: &'static str,
}

pub trait Rule {
    fn info(&self) -> RuleInfo;
    fn run(&self, ws: &Workspace, config: &Config, out: &mut dyn Emitter);
}

pub fn all_rule_infos() -> Vec<RuleInfo> {
    vec![
        AbsoluteModulePathsRule::static_info(),
        AbsoluteFilesystemPathsRule::static_info(),
        FileComplexityRule::static_info(),
        DuplicateLogicRule::static_info(),
        DuplicateTypesAliasCandidateRule::static_info(),
        SrpHeuristicRule::static_info(),
    ]
}

pub fn enabled_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();

    if config.rules.absolute_module_paths.level.enabled() {
        rules.push(Box::new(AbsoluteModulePathsRule::new(
            config.rules.absolute_module_paths.clone(),
        )));
    }

    if config.rules.absolute_filesystem_paths.level.enabled() {
        rules.push(Box::new(AbsoluteFilesystemPathsRule::new(
            config.rules.absolute_filesystem_paths.clone(),
        )));
    }

    if config.rules.file_complexity.level.enabled() {
        rules.push(Box::new(FileComplexityRule::new(
            config.rules.file_complexity.clone(),
        )));
    }

    if config.rules.duplicate_logic.level.enabled() {
        rules.push(Box::new(DuplicateLogicRule::new(
            config.rules.duplicate_logic.clone(),
        )));
    }

    if config.rules.duplicate_types_alias_candidate.level.enabled() {
        rules.push(Box::new(DuplicateTypesAliasCandidateRule::new(
            config.rules.duplicate_types_alias_candidate.clone(),
        )));
    }

    if config.rules.srp_heuristic.level.enabled() {
        rules.push(Box::new(SrpHeuristicRule::new(
            config.rules.srp_heuristic.clone(),
        )));
    }

    rules
}
