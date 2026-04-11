export const installSnippet = `cargo install --git https://github.com/xsyetopz/rscheck --locked rscheck-cli
rscheck init
rscheck check --format text`;

export const policySnippet = `version = 2

[engine]
semantic = "auto"

[adapters.clippy]
enabled = true

[rules."architecture.qualified_module_paths"]
level = "deny"

[rules."shape.file_complexity"]
level = "warn"
max_file = 220
max_fn = 30

[[scope]]
include = ["crates/rscheck-cli/**"]

[scope.rules."architecture.banned_dependencies"]
level = "deny"
banned_prefixes = ["std::process::Command"]`;

export const featureCards = [
	{
		title: "Policy layers",
		body: "Stack base policy, team policy, and path-scoped overrides in one config instead of spreading decisions across ad-hoc lint toggles.",
	},
	{
		title: "Rule families",
		body: "Mix architecture, design, portability, and code-shape checks under one report instead of treating everything like style lint.",
	},
	{
		title: "Fix pipeline",
		body: "Carry machine-safe edits from rscheck rules and Clippy into one pass, with dry runs and write mode built into the CLI.",
	},
	{
		title: "Semantic runway",
		body: "The v2 engine already models stable syntax rules and an optional semantic backend so deeper Rust checks can land without another config rewrite.",
	},
];

export const families = [
	{
		name: "Architecture",
		points: [
			"qualified module path rules",
			"banned dependency prefixes",
			"layer direction checks",
		],
	},
	{
		name: "Design",
		points: [
			"public API error contracts",
			"repeated type alias candidates",
			"future construction and export constraints",
		],
	},
	{
		name: "Shape",
		points: [
			"file and function complexity thresholds",
			"duplicate logic detection",
			"responsibility split heuristics",
		],
	},
];
