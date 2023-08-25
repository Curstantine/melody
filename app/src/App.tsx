import { Route, Router, Routes } from "@solidjs/router";
import { appWindow } from "@tauri-apps/api/window";
import { onMount } from "solid-js";

import type { AppError } from "@/types/errors";
import { initialize as initializeTheme } from "@/utils/themes";

import AppErrorDisplay from "@/components/AppErrorDisplay";
import TitleBar from "@/components/TitleBar";

import { AppModelContext } from "@/contexts/models";
import AppModel from "@/models/App";
import Home from "@/pages/setup";

export default function App() {
	const appModel = new AppModel();

	onMount(async () => {
		const themeResult = await initializeTheme();
		if (themeResult.isErr()) {
			const error: AppError = { dismissible: true, error: themeResult.unwrapErr() };
			appModel.appError.set(error);
		}

		appWindow.show();
	});

	return (
		<Router>
			<AppModelContext.Provider value={appModel}>
				<TitleBar />
				<AppErrorDisplay />
				<Routes>
					<Route path="/" component={Home} />
				</Routes>
			</AppModelContext.Provider>
		</Router>
	);
}
