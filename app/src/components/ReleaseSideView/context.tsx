import { createContext, createSignal, type JSX, type Signal, useContext } from "solid-js";

import type { Person } from "@/types/backend/person";
import type { Release } from "@/types/backend/release";

export type ContextDataType = { releaseId: number; release: Release; artists: Record<number, Person> };
type ContextType = {
	visible: Signal<boolean>;
	data: Signal<ContextDataType | null>;
	sizer: Signal<number>;
	close: () => Promise<void>;
	open: (data: ContextDataType) => void;
};

const ReleaseSideViewContext = createContext<ContextType>();

type Props = { children: JSX.Element };
export function ReleaseSideViewProvider(props: Props) {
	const visible = createSignal<boolean>(false);
	const data = createSignal<ContextDataType | null>(null);
	const sizer = createSignal<number>(26);

	const [, setVisibility] = visible;
	const [, setViewData] = data;

	let closingTimer: number;

	const open = (data: ContextDataType) => {
		clearTimeout(closingTimer);
		setViewData(data);
		setVisibility(true);
	};

	const close = async () => {
		setVisibility(false);
		// await delay(1500);
		closingTimer = window.setTimeout(() => setViewData(null), 1500);
	};

	return (
		<ReleaseSideViewContext.Provider value={{ visible, data, sizer, open, close }}>
			{props.children}
		</ReleaseSideViewContext.Provider>
	);
}

/**
 * `resizer`: The size of the x axis of this side view in rem
 */
export const useReleaseSideViewData = () => useContext(ReleaseSideViewContext)!;
