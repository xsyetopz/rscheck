mod absolute_filesystem_paths;
mod absolute_module_paths;
mod banned_dependencies;
mod custom_pattern;
mod duplicate_logic;
mod duplicate_types_alias;
mod external_test_modules;
mod file_complexity;
mod god_object;
mod hot_path_allocations;
mod layer_direction;
mod naming_policy;
mod public_api_errors;
mod srp_heuristic;
mod use_tree_path;

use crate::analysis::Workspace;
use crate::config::{Level, Policy, RuleSettings};
use crate::emit::Emitter;
use crate::report::RuleCatalogEntry;

pub use absolute_filesystem_paths::AbsoluteFilesystemPathsRule;
pub use absolute_module_paths::AbsoluteModulePathsRule;
pub use banned_dependencies::BannedDependenciesRule;
pub use custom_pattern::CustomPatternRule;
pub use duplicate_logic::DuplicateLogicRule;
pub use duplicate_types_alias::DuplicateTypesAliasCandidateRule;
pub use external_test_modules::ExternalTestModulesRule;
pub use file_complexity::FileComplexityRule;
pub use god_object::GodObjectRule;
pub use hot_path_allocations::HotPathAllocationsRule;
pub use layer_direction::LayerDirectionRule;
pub use naming_policy::NamingPolicyRule;
pub use public_api_errors::PublicApiErrorsRule;
pub use srp_heuristic::SrpHeuristicRule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleFamily {
    Architecture,
    Design,
    Shape,
    Portability,
    Testing,
    Performance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleBackend {
    Syntax,
    Semantic,
    Adapter,
}

#[derive(Debug, Clone, Copy)]
pub struct RuleInfo {
    pub id: &'static str,
    pub family: RuleFamily,
    pub backend: RuleBackend,
    pub summary: &'static str,
    pub default_level: Level,
    pub schema: &'static str,
    pub config_example: &'static str,
    pub fixable: bool,
}

pub struct RuleContext<'a> {
    pub policy: &'a Policy,
}

pub trait Rule {
    fn info(&self) -> RuleInfo;
    fn run(&self, ws: &Workspace, ctx: &RuleContext<'_>, out: &mut dyn Emitter);
}

fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(AbsoluteModulePathsRule),
        Box::new(AbsoluteFilesystemPathsRule),
        Box::new(FileComplexityRule),
        Box::new(DuplicateLogicRule),
        Box::new(DuplicateTypesAliasCandidateRule),
        Box::new(SrpHeuristicRule),
        Box::new(BannedDependenciesRule),
        Box::new(PublicApiErrorsRule),
        Box::new(LayerDirectionRule),
        Box::new(ExternalTestModulesRule),
        Box::new(NamingPolicyRule),
        Box::new(GodObjectRule),
        Box::new(HotPathAllocationsRule),
        Box::new(CustomPatternRule),
    ]
}

pub fn rule_catalog() -> Vec<RuleInfo> {
    all_rules().into_iter().map(|rule| rule.info()).collect()
}

pub fn rule_catalog_entries() -> Vec<RuleCatalogEntry> {
    rule_catalog()
        .into_iter()
        .map(|info| RuleCatalogEntry {
            id: info.id.to_string(),
            family: info.family,
            backend: info.backend,
            default_level: info.default_level,
            summary: info.summary.to_string(),
            fixable: info.fixable,
        })
        .collect()
}

pub fn enabled_rules(policy: &Policy) -> Vec<Box<dyn Rule>> {
    all_rules()
        .into_iter()
        .filter(|rule| {
            let rule_info = rule.info();
            policy.rule_enabled_anywhere(rule_info.id, rule_info.default_level)
        })
        .collect()
}

pub fn default_rule_settings() -> Vec<(String, RuleSettings)> {
    rule_catalog()
        .into_iter()
        .map(|info| {
            (
                info.id.to_string(),
                RuleSettings {
                    level: Some(info.default_level),
                    options: toml::Table::new(),
                },
            )
        })
        .collect()
}
