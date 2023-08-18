import { createContext, type Signal, useContext } from "solid-js";

import type { LocalError } from "@/types/errors";

export const AppErrorContext = createContext<Signal<LocalError | null>>(
	[() => null, () => {}] as Signal<LocalError | null>,
	{ name: "AppErrorContext" },
);
export const useAppErrorContext = () => useContext(AppErrorContext);
