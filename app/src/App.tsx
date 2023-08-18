import { appWindow } from "@tauri-apps/api/window";
import { createSignal, onMount } from "solid-js";

import { AppErrorContext } from "@/contexts/errors";
import type { AppError } from "@/types/errors";
import { initialize as initializeTheme } from "@/utils/themes";

import AppErrorDisplay from "@/components/AppErrorDisplay";
import TitleBar from "@/components/TitleBar";

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
		<AppErrorContext.Provider value={[appError, setAppError]}>
			<TitleBar />
			<AppErrorDisplay />
			<span>no</span>
		</AppErrorContext.Provider>
	);
}
