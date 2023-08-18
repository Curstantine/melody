/* @refresh reload */
import { ErrorBoundary, render } from "solid-js/web";

import "virtual:uno.css";
import "@/index.css";
import "@unocss/reset/tailwind.css";

import App from "@/App";

const root = document.getElementById("root");

render(() => (
	<ErrorBoundary fallback={<div>Something went wrong</div>}>
		<App />
	</ErrorBoundary>
), root!);
