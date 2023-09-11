import { createContext, createSignal, useContext } from "solid-js";

import type { AppError } from "@/types/errors";

import LibraryManager from "@/models/LibraryManager";

export default class AppModel {
	appError = createSignal<AppError | null>(null);
	libraryManager = new LibraryManager();
}

export const AppModelContext = createContext<AppModel>(undefined, { name: "AppModelContext" });
export const useAppModel = () => useContext(AppModelContext)!;
