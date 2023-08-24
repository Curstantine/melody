/* @refresh reload */
import { ErrorBoundary, render } from "solid-js/web";

import "virtual:uno.css";
import "@unocss/reset/tailwind-compat.css";

import "@/index.css";
import App from "@/App";

const root = document.getElementById("root");

render(() => (
	<ErrorBoundary fallback={err => err}>
		<App />
	</ErrorBoundary>
), root!);
