import { useNavigate } from "@solidjs/router";
import { appWindow } from "@tauri-apps/api/window";
import { createContext, createSignal, useContext } from "solid-js";

import type { ActionableError } from "@/types/errors";
import { invoke } from "@/utils/tauri";
import { initialize as initializeTheme } from "@/utils/themes";

import { SHARED_PATHS } from "@/pages/(shared)";
import { SETUP_PATHS } from "@/pages/setup";
import { LibraryEntity } from "@/types/backend/library";

export default class AppModel {
	private navigate = useNavigate();

	currentLibraryId = createSignal<number>();
	appError = createSignal<ActionableError | null>(null);

	constructor() {
		this.initialize.bind(this);
		this.setAppError.bind(this);
	}

	public async initialize() {
		const [, setAppError] = this.appError;
		const [, setCurrentLibraryId] = this.currentLibraryId;

		const setup = await invoke<void>("setup");
		if (setup.isErr()) {
			setAppError({ dismissible: false, error: setup.unwrapErr() });
		}

		const themeResult = await initializeTheme();
		if (themeResult.isErr()) {
			setAppError({ dismissible: true, error: themeResult.unwrapErr() });
		}

		const result = await invoke<LibraryEntity[]>("get_libraries");
		if (result.isErr()) {
			setAppError({ dismissible: true, error: result.unwrapErr() });
		} else {
			const libraries = result.unwrap();
			if (libraries.length === 0) {
				// TODO: persist id across restarts
				setCurrentLibraryId(libraries[0].id);
				this.navigate(SETUP_PATHS.CREATE);
			} else {
				this.navigate(SHARED_PATHS.MUSIC);
			}
		}

		if (import.meta.env.DEV) {
			// @ts-expect-error - expose function to allow to navigate to dev showcase
			window.__APP__ = {
				goToDevShowcase: () => {
					this.navigate("/dev/showcase");
				},
			};
		}

		appWindow.show();
	}

	public setAppError(error: ActionableError["error"], dismissible = true, actions?: ActionableError["actions"]) {
		const [, setAppError] = this.appError;
		setAppError({ error, dismissible, actions });
	}

	public setCurrentLibraryId(id: number) {
		const [, setId] = this.currentLibraryId;
		setId(id);
	}
}

export const AppModelContext = createContext<AppModel>();
export const useAppModel = () => useContext(AppModelContext)!;
