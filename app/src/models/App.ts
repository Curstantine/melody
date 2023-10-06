import { useNavigate } from "@solidjs/router";
import { appWindow } from "@tauri-apps/api/window";
import { createContext, createSignal, useContext } from "solid-js";

import type { ActionableError } from "@/types/errors";
import { invoke } from "@/utils/tauri";
import { initialize as initializeTheme } from "@/utils/themes";

import LibraryManager from "@/models/LibraryManager";

export default class AppModel {
	appError = createSignal<ActionableError | null>(null);
	libraryManager = new LibraryManager();
	navigate = useNavigate();

	constructor() {
		this.initialize.bind(this);
	}

	public async initialize() {
		const [, setAppError] = this.appError;

		const setup = await invoke<void>("setup");
		if (setup.isErr()) {
			setAppError({ dismissible: false, error: setup.unwrapErr() });
		}

		const themeResult = await initializeTheme();
		if (themeResult.isErr()) {
			setAppError({ dismissible: true, error: themeResult.unwrapErr() });
		}

		const namesResult = await invoke<string[]>("get_library_names");
		if (namesResult.isOk()) {
			const names = namesResult.unwrap();
			if (names.length === 0) this.navigate("/setup/create");
			else this.navigate("/home");
		} else {
			setAppError({ dismissible: true, error: namesResult.unwrapErr() });
		}

		if (import.meta.env.DEV) {
			// @ts-expect-error - expose function to allow to navigate to dev showcase
			window.__appModel__.goToDevShowcase = () => {
				this.navigate("/dev/showcase");
			};
		}

		appWindow.show();
	}

	public setAppError(error: ActionableError["error"], dismissible = true, actions?: ActionableError["actions"]) {
		const [, setAppError] = this.appError;
		setAppError({ error, dismissible, actions });
	}
}

export const AppModelContext = createContext<AppModel>(undefined, { name: "AppModelContext" });
export const useAppModel = () => useContext(AppModelContext)!;
