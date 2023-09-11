import { Route, Router, Routes } from "@solidjs/router";
import { appWindow } from "@tauri-apps/api/window";
import { onMount } from "solid-js";

import { AppModelContext } from "@/contexts/models";
import { initialize as initializeTheme } from "@/utils/themes";

import AppErrorDisplay from "@/components/AppErrorDisplay";
import TitleBar from "@/components/TitleBar";

import AppModel from "@/models/App";
import UIRoot from "@/pages";
import Setup from "@/pages/setup";
import { invoke } from "@/utils/tauri";

export default function App() {
	const appModel = new AppModel();

	onMount(async () => {
		const { appError: [, setAppError] } = appModel;

		const setup = await invoke<void>("setup");
		if (setup.isErr()) {
			setAppError({ dismissible: false, error: setup.unwrapErr() });
		}

		const themeResult = await initializeTheme();
		if (themeResult.isErr()) {
			setAppError({ dismissible: true, error: themeResult.unwrapErr() });
		}

		appWindow.show();
	});

	return (
		<Router>
			<AppModelContext.Provider value={appModel}>
				<TitleBar />
				<AppErrorDisplay />
				<Routes>
					<Route path="/" component={UIRoot} />
					<Route path="/setup" component={Setup} />
				</Routes>
			</AppModelContext.Provider>
		</Router>
	);
}
