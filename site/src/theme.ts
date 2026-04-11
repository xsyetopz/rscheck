import { createTheme } from "@mantine/core";

export const theme = createTheme({
	primaryColor: "gray",
	fontFamily:
		'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
	fontFamilyMonospace:
		'"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, monospace',
	headings: {
		fontFamily:
			'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
		fontWeight: "600",
	},
	defaultRadius: "md",
});
