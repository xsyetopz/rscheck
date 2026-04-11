import "@mantine/core/styles.css";
import "@mantine/code-highlight/styles.css";
import "./styles.css";

import { localStorageColorSchemeManager, MantineProvider } from "@mantine/core";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { theme } from "./theme";

const root = document.getElementById("root");
if (!root) throw new Error("Root element not found");

const colorSchemeManager = localStorageColorSchemeManager({
	key: "rscheck-color-scheme",
});

createRoot(root).render(
	<MantineProvider
		theme={theme}
		colorSchemeManager={colorSchemeManager}
		defaultColorScheme="auto"
	>
		<App />
	</MantineProvider>,
);
