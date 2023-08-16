/* @refresh reload */
import { init } from "@neutralinojs/lib";
import { ErrorBoundary, render } from "solid-js/web";

import "virtual:auth-info";
import "virtual:uno.css";

import "@unocss/reset/tailwind.css";
import "@/index.css";

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

init();
