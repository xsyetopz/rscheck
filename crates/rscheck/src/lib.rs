pub mod analysis;
pub mod config;
pub mod emit;
pub mod fix;
mod path_pattern;
pub mod policy;
pub mod report;
pub mod rules;
pub mod runner;
pub mod semantic;
pub mod span;
#[cfg(test)]
pub(crate) mod test_support;
