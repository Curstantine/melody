import { appWindow } from "@tauri-apps/api/window";
import { createSignal, onMount } from "solid-js";

import AppErrorDisplay from "@/components/AppErrorDisplay";
import TitleBar from "@/components/TitleBar";
import { AppErrorContext } from "@/contexts/errors";
import { LocalError } from "@/types/errors";
import { initialize as initializeTheme } from "@/utils/themes";

export default function App() {
	const [appError, setAppError] = createSignal<LocalError | null>(null);

	onMount(async () => {
		const themeResult = await initializeTheme();
		if (themeResult.isErr) {
			setAppError(themeResult.unwrapErr());
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
