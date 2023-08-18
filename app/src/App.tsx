import { Route, Router, Routes } from "@solidjs/router";
import { appWindow } from "@tauri-apps/api/window";
import { createSignal, onMount } from "solid-js";

import { AppErrorContext } from "@/contexts/errors";
import type { AppError } from "@/types/errors";
import { initialize as initializeTheme } from "@/utils/themes";

import AppErrorDisplay from "@/components/AppErrorDisplay";
import TitleBar from "@/components/TitleBar";

import Home from "@/pages/home";

export default function App() {
	const [appError, setAppError] = createSignal<AppError | null>(null);

	onMount(async () => {
		const themeResult = await initializeTheme();
		if (themeResult.isErr) {
			const error: AppError = { dismissible: true, error: themeResult.unwrapErr() };
			setAppError(error);
		}

		appWindow.show();
	});

	return (
		<Router>
			<AppErrorContext.Provider value={[appError, setAppError]}>
				<TitleBar />
				<AppErrorDisplay />
				<Routes>
					<Route path="/" component={Home} />
				</Routes>
			</AppErrorContext.Provider>
		</Router>
	);
}
