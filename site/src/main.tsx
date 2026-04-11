import "@mantine/core/styles.css";
import "@mantine/code-highlight/styles.css";
import "./styles.css";

import { MantineProvider } from "@mantine/core";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { theme } from "./theme";

const root = document.getElementById("root");
if (!root) throw new Error("Root element not found");

createRoot(root).render(
	<MantineProvider theme={theme}>
		<App />
	</MantineProvider>,
);
