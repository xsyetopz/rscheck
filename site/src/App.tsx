import { CodeHighlight } from "@mantine/code-highlight";
import {
	ActionIcon,
	Anchor,
	Badge,
	Button,
	Code,
	Container,
	Flex,
	Group,
	List,
	Paper,
	SimpleGrid,
	Stack,
	Text,
	ThemeIcon,
	Title,
} from "@mantine/core";
import {
	IconArrowRight,
	IconBrandGithub,
	IconHammer,
	IconHierarchy3,
	IconPlugConnected,
	IconRadar2,
} from "@tabler/icons-react";
import {
	families,
	featureCards,
	installSnippet,
	policySnippet,
} from "./content";

const repoUrl = "https://github.com/xsyetopz/rscheck";

export function App() {
	return (
		<div className="page-shell">
			<Container size="lg" py={32}>
				<Stack gap={48}>
					<Paper
						className="hero-panel"
						p={{ base: "xl", sm: "2rem" }}
						radius="xl"
					>
						<Stack gap="xl">
							<Group justify="space-between" align="flex-start">
								<Stack gap="sm" maw={760}>
									<Badge
										variant="filled"
										color="dark"
										radius="sm"
										style={{ alignSelf: "flex-start" }}
									>
										rscheck v2
									</Badge>
									<Title order={1} fz={{ base: 38, sm: 56 }} lh={1}>
										Static analysis for Rust teams that need policy, not a pile
										of toggles.
									</Title>
									<Text size="lg" c="dimmed" maw={640}>
										rscheck sits above compiler diagnostics and Clippy. It lets
										you enforce architecture lines, API contracts, code-shape
										rules, and scoped policy layers from one report.
									</Text>
								</Stack>

								<ActionIcon
									component="a"
									href={repoUrl}
									size={48}
									radius="xl"
									variant="default"
									aria-label="Open GitHub repository"
								>
									<IconBrandGithub size={24} />
								</ActionIcon>
							</Group>

							<Group gap="md">
								<Button
									component="a"
									href={`${repoUrl}#install`}
									size="md"
									rightSection={<IconArrowRight size={16} />}
								>
									Install rscheck
								</Button>
								<Button
									component="a"
									href={`${repoUrl}#configuration`}
									size="md"
									variant="default"
								>
									Read the config model
								</Button>
							</Group>

							<SimpleGrid cols={{ base: 1, sm: 3 }} spacing="md">
								<Paper className="metric-chip" p="md" radius="lg">
									<Text fw={700}>Layered policy</Text>
									<Text size="sm" c="dimmed">
										`extends`, path scopes, adapter controls, and dot rule IDs.
									</Text>
								</Paper>
								<Paper className="metric-chip" p="md" radius="lg">
									<Text fw={700}>Fix-capable runs</Text>
									<Text size="sm" c="dimmed">
										Dry runs, safe writes, and merged fixes from rscheck and
										Clippy.
									</Text>
								</Paper>
								<Paper className="metric-chip" p="md" radius="lg">
									<Text fw={700}>Semantic runway</Text>
									<Text size="sm" c="dimmed">
										Stable syntax rules today, semantic backend switch already
										in the policy model.
									</Text>
								</Paper>
							</SimpleGrid>
						</Stack>
					</Paper>

					<SimpleGrid cols={{ base: 1, md: 2 }} spacing="lg">
						<Paper className="section-card" p="xl" radius="xl">
							<Stack gap="md">
								<Group gap="sm">
									<ThemeIcon variant="light" color="lime" size={38} radius="md">
										<IconHammer size={20} />
									</ThemeIcon>
									<Title order={2}>Get it running</Title>
								</Group>
								<Text c="dimmed">
									The CLI stays small: initialize policy, run checks, choose a
									report format, and write fixes when the run is clean enough.
								</Text>
								<CodeHighlight
									code={installSnippet}
									language="bash"
									withCopyButton={false}
									radius="md"
								/>
							</Stack>
						</Paper>

						<Paper className="section-card" p="xl" radius="xl">
							<Stack gap="md">
								<Group gap="sm">
									<ThemeIcon variant="light" color="lime" size={38} radius="md">
										<IconPlugConnected size={20} />
									</ThemeIcon>
									<Title order={2}>Policy file, not lint soup</Title>
								</Group>
								<Text c="dimmed">
									V2 policy is built for real repo boundaries: engine mode,
									Clippy as an adapter, global rules, and path-scoped overrides.
								</Text>
								<CodeHighlight
									code={policySnippet}
									language="toml"
									withCopyButton={false}
									radius="md"
								/>
							</Stack>
						</Paper>
					</SimpleGrid>

					<Stack gap="lg">
						<Flex justify="space-between" align="end" gap="md" wrap="wrap">
							<div>
								<Title order={2}>What rscheck covers</Title>
								<Text c="dimmed" maw={620}>
									The current rule catalog is already split by intent. That is
									the point: architecture drift, API contract drift, and code
									shape drift are not the same problem.
								</Text>
							</div>
							<Group gap="xs">
								<Code>rscheck list-rules</Code>
								<Code>rscheck explain shape.file_complexity</Code>
							</Group>
						</Flex>

						<SimpleGrid cols={{ base: 1, md: 3 }} spacing="lg">
							{families.map((family) => (
								<Paper
									key={family.name}
									className="section-card"
									p="xl"
									radius="xl"
								>
									<Stack gap="sm">
										<Group gap="sm">
											<ThemeIcon
												variant="light"
												color="dark"
												size={34}
												radius="md"
											>
												{family.name === "Architecture" ? (
													<IconHierarchy3 size={18} />
												) : family.name === "Design" ? (
													<IconRadar2 size={18} />
												) : (
													<IconHammer size={18} />
												)}
											</ThemeIcon>
											<Title order={3}>{family.name}</Title>
										</Group>
										<List spacing="xs" size="sm">
											{family.points.map((point) => (
												<List.Item key={point}>{point}</List.Item>
											))}
										</List>
									</Stack>
								</Paper>
							))}
						</SimpleGrid>
					</Stack>

					<Paper
						className="rules-panel"
						p={{ base: "xl", sm: "2rem" }}
						radius="xl"
					>
						<Stack gap="lg">
							<div>
								<Title order={2} c="white">
									Why this exists
								</Title>
								<Text c="gray.4" maw={720}>
									Clippy is good at linting Rust code. It is not built to own
									your architecture policy. rscheck gives you a place to enforce
									team rules that sit above syntax hygiene: layer boundaries,
									disallowed dependencies, public API error contracts, and
									repo-specific complexity thresholds.
								</Text>
							</div>

							<SimpleGrid cols={{ base: 1, md: 2 }} spacing="lg">
								{featureCards.map((card) => (
									<Paper
										key={card.title}
										p="lg"
										radius="lg"
										style={{
											background: "rgba(255,255,255,0.04)",
											border: "1px solid rgba(255,255,255,0.08)",
										}}
									>
										<Stack gap="xs">
											<Text fw={700} c="white">
												{card.title}
											</Text>
											<Text size="sm" c="gray.4">
												{card.body}
											</Text>
										</Stack>
									</Paper>
								))}
							</SimpleGrid>
						</Stack>
					</Paper>

					<Paper className="footer-band" p="xl" radius="xl">
						<Stack gap="md">
							<Title order={3}>Use the repo as the source of truth</Title>
							<Text c="dimmed">
								The site is a front door. The working surface lives in the repo:
								current CLI docs, example policy files, and the evolving rule
								catalog.
							</Text>
							<Group gap="md">
								<Anchor href={repoUrl}>GitHub repository</Anchor>
								<Anchor href={`${repoUrl}#install`}>README install</Anchor>
								<Anchor href={`${repoUrl}#configuration`}>
									README configuration
								</Anchor>
							</Group>
						</Stack>
					</Paper>
				</Stack>
			</Container>
		</div>
	);
}
