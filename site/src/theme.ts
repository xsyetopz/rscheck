import { createTheme } from "@mantine/core";

export const theme = createTheme({
	primaryColor: "lime",
	fontFamily: "'Space Grotesk', sans-serif",
	fontFamilyMonospace: "'JetBrains Mono', monospace",
	headings: {
		fontFamily: "'Space Grotesk', sans-serif",
		fontWeight: "700",
	},
	colors: {
		dark: [
			"#c9ccd1",
			"#afb3ba",
			"#9196a0",
			"#6d7380",
			"#484f5d",
			"#313745",
			"#262b37",
			"#1b202b",
			"#11151d",
			"#090c12",
		],
	},
	other: {
		canvas: "#f4f2ec",
		panel: "#fffdf8",
		border: "#d8d2c4",
		ink: "#111317",
		softInk: "#5f655f",
		accent: "#b8ff45",
	},
});
