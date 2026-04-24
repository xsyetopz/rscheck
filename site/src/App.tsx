import { CodeHighlight } from "@mantine/code-highlight";
import {
	ActionIcon,
	Anchor,
	Container,
	Group,
	List,
	Paper,
	Stack,
	Text,
	Title,
	Tooltip,
	useMantineColorScheme,
} from "@mantine/core";
import { IconBrightness, IconMoon, IconSun } from "@tabler/icons-react";
import {
	families,
	installSnippet,
	linkItems,
	policySnippet,
	quickStartSteps,
} from "./content";

const repoUrl = "https://github.com/xsyetopz/rscheck";

function ThemeModeControl() {
	const { colorScheme, setColorScheme } = useMantineColorScheme();
	const nextColorScheme =
		colorScheme === "auto"
			? "light"
			: colorScheme === "light"
				? "dark"
				: "auto";
	const label =
		colorScheme === "auto"
			? "Theme: auto"
			: colorScheme === "light"
				? "Theme: light"
				: "Theme: dark";
	const icon =
		colorScheme === "auto" ? (
			<IconBrightness size={18} />
		) : colorScheme === "light" ? (
			<IconSun size={18} />
		) : (
			<IconMoon size={18} />
		);

	return (
		<Tooltip label={`${label}. Click to switch to ${nextColorScheme}.`}>
			<ActionIcon
				aria-label={label}
				variant="default"
				size="lg"
				onClick={() => setColorScheme(nextColorScheme)}
			>
				{icon}
			</ActionIcon>
		</Tooltip>
	);
}

export function App() {
	return (
		<div className="page-shell">
			<Container size="md" py={{ base: 24, sm: 40 }}>
				<Stack gap="lg">
					<Group justify="space-between" align="flex-start" gap="md">
						<Stack gap={6}>
							<Title order={1}>rscheck</Title>
							<Text c="dimmed" maw={680}>
								Checks Rust workspaces against layered rules, scoped
								configuration, and checks that Clippy does not cover.
							</Text>
						</Stack>
						<ThemeModeControl />
					</Group>

					<Paper className="section-block" p="lg" radius="md">
						<Stack gap="sm">
							<Title order={2}>Install</Title>
							<CodeHighlight
								code={installSnippet}
								language="bash"
								radius="md"
								withCopyButton={false}
							/>
						</Stack>
					</Paper>

					<Paper className="section-block" p="lg" radius="md">
						<Stack gap="sm">
							<Title order={2}>Quick Start</Title>
							<List spacing="xs">
								{quickStartSteps.map((step) => (
									<List.Item key={step}>{step}</List.Item>
								))}
							</List>
						</Stack>
					</Paper>

					<Paper className="section-block" p="lg" radius="md">
						<Stack gap="sm">
							<Title order={2}>Policy</Title>
							<Text c="dimmed">
								A v3 policy file sets channel mode, adapters, root rules, and
								path-scoped overrides.
							</Text>
							<CodeHighlight
								code={policySnippet}
								language="toml"
								radius="md"
								withCopyButton={false}
							/>
						</Stack>
					</Paper>

					<Paper className="section-block" p="lg" radius="md">
						<Stack gap="sm">
							<Title order={2}>Rules</Title>
							{families.map((family) => (
								<div key={family.name}>
									<Text fw={600}>{family.name}</Text>
									<List spacing="xs" size="sm" mt={6}>
										{family.points.map((point) => (
											<List.Item key={point}>{point}</List.Item>
										))}
									</List>
								</div>
							))}
						</Stack>
					</Paper>

					<Paper className="section-block" p="lg" radius="md">
						<Stack gap="sm">
							<Title order={2}>Links</Title>
							<List spacing="xs" size="sm">
								{linkItems.map((item) => (
									<List.Item key={item.href}>
										<Anchor href={item.href}>{item.label}</Anchor>
									</List.Item>
								))}
							</List>
							<Text size="sm" c="dimmed">
								Repository: <Anchor href={repoUrl}>{repoUrl}</Anchor>
							</Text>
						</Stack>
					</Paper>
				</Stack>
			</Container>
		</div>
	);
}
