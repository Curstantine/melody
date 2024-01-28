import {
	batch,
	createContext,
	createSignal,
	type JSX,
	type Signal,
	untrack,
	useContext,
	useTransition,
} from "solid-js";

import type { Person } from "@/types/backend/person";
import type { Release } from "@/types/backend/release";
import { Transition } from "solid-js/types/reactive/signal.js";

export type ContextDataType = { releaseId: number; release: Release; artists: Record<number, Person> };
type ContextType = {
	visible: Signal<boolean>;
	data: Signal<ContextDataType | null>;
	sizer: Signal<number>;
	transition: Transition;
	close: () => Promise<void>;
	open: (data: ContextDataType) => void;
};

const ReleaseSideViewContext = createContext<ContextType>();

type Props = { children: JSX.Element };
export function ReleaseSideViewProvider(props: Props) {
	const visible = createSignal<boolean>(false);
	const data = createSignal<ContextDataType | null>(null);
	const sizer = createSignal<number>(26);
	const transition = useTransition();

	const [isVisible, setVisibility] = visible;
	const [viewData, setViewData] = data;
	const [, start] = transition;

	let closingTimer: number;

	const open = (data: ContextDataType) => {
		const uViewData = untrack(viewData);
		const uIsVisible = untrack(isVisible);

		clearTimeout(closingTimer);

		if (uViewData !== null) {
			console.log("wao");
			if (!uIsVisible) setVisibility(true);
			start(() => setViewData(data));
		} else {
			batch(() => {
				setViewData(data);
				setVisibility(true);
			});
		}
	};

	const close = async () => {
		setVisibility(false);
		closingTimer = window.setTimeout(() => setViewData(null), 150);
	};

	return (
		<ReleaseSideViewContext.Provider value={{ visible, data, sizer, open, close, transition }}>
			{props.children}
		</ReleaseSideViewContext.Provider>
	);
}

/**
 * `resizer`: The size of the x axis of this side view in rem
 */
export const useReleaseSideViewData = () => useContext(ReleaseSideViewContext)!;
