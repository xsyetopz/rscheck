export const installSnippet = `cargo install --git https://github.com/xsyetopz/rscheck --locked rscheck-cli
rscheck init
rscheck check --format text`;

export const quickStartSteps = [
	"Run rscheck init to create a v2 policy file.",
	"Run rscheck check --format text.",
	"Enable [adapters.clippy] when you want Clippy findings in the same report.",
	'Set engine.semantic = "auto", "require", or "off" to match the backend you expect.',
];

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
			"construction and export constraints",
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
