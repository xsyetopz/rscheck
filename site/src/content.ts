export const installSnippet = `cargo install --git https://github.com/xsyetopz/rscheck --locked rscheck-cli
rscheck init
rscheck check --format text`;

export const quickStartSteps = [
	"Run rscheck init to create a v3 policy file.",
	"Run rscheck check --format text.",
	"Enable [adapters.clippy] when you want Clippy findings in the same report.",
	'Set engine.toolchain = "stable", "nightly", or "auto" and engine.semantic to match the backend you expect.',
];

export const policySnippet = `version = 3

[engine]
semantic = "auto"
toolchain = "stable"

[adapters.clippy]
enabled = true
toolchain = "inherit"

[rules."architecture.qualified_module_paths"]
level = "deny"

[rules."shape.file_complexity"]
level = "warn"
max_file = 220
max_fn = 30

[rules."testing.external_test_modules"]
level = "deny"

[rules."design.naming_policy"]
level = "warn"

[[scope]]
include = ["crates/rscheck-cli/**"]

[scope.rules."architecture.banned_dependencies"]
level = "deny"
banned_prefixes = ["std::process::Command"]`;

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
			"naming, god-object, and API error constraints",
		],
	},
	{
		name: "Shape",
		points: [
			"file and function complexity thresholds",
			"duplicate logic detection",
			"responsibility split and test layout checks",
		],
	},
];

export const linkItems = [
	{ label: "GitHub repository", href: "https://github.com/xsyetopz/rscheck" },
	{
		label: "README: install",
		href: "https://github.com/xsyetopz/rscheck#install",
	},
	{
		label: "README: configuration",
		href: "https://github.com/xsyetopz/rscheck#configuration",
	},
];
