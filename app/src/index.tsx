/* @refresh reload */
import { ErrorBoundary, render } from "solid-js/web";

import "virtual:uno.css";
import "@/index.css";
import "@unocss/reset/tailwind.css";

import App from "@/App";
import AppError from "@/components/errors/AppError";
import TitleBar from "@/components/TitleBar";

const root = document.getElementById("root");

render(() => (
	<ErrorBoundary fallback={AppError}>
		<TitleBar />
		<App />
	</ErrorBoundary>
), root!);
