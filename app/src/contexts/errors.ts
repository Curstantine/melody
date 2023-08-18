import { createContext, type Signal, useContext } from "solid-js";

import type { AppError } from "@/types/errors";

export const AppErrorContext = createContext<Signal<AppError | null>>(
	[() => null, () => {}] as Signal<AppError | null>,
	{ name: "AppErrorContext" },
);
export const useAppErrorContext = () => useContext(AppErrorContext);
