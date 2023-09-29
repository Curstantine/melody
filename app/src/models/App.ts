import { useNavigate } from "@solidjs/router";
import { appWindow } from "@tauri-apps/api/window";
import { createContext, createSignal, useContext } from "solid-js";

import type { AppError } from "@/types/errors";
import { invoke } from "@/utils/tauri";
import { initialize as initializeTheme } from "@/utils/themes";

import LibraryManager from "@/models/LibraryManager";

export default class AppModel {
	appError = createSignal<AppError | null>(null);
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

		appWindow.show();
	}

	public setAppError(error: AppError["error"], dismissible = true) {
		const [, setAppError] = this.appError;
		setAppError({ dismissible, error });
	}
}

export const AppModelContext = createContext<AppModel>(undefined, { name: "AppModelContext" });
export const useAppModel = () => useContext(AppModelContext)!;
