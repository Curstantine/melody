/* @refresh reload */
import { Router } from "@solidjs/router";
import { render } from "solid-js/web";

import "virtual:uno.css";
import "@unocss/reset/tailwind-compat.css";

import "@/index.css";
import App from "@/App";

const root = document.getElementById("root");

render(() => (
	<Router>
		<App />
	</Router>
), root!);
