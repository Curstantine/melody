import { type Accessor, createContext, createSignal, type JSX, type Setter, useContext } from "solid-js";

import { Person } from "@/types/backend/person";
import type { Release } from "@/types/backend/release";

export type ContextType = { release: Release; artists: Record<number, Person> };
const ReleaseSideViewContext = createContext<[Accessor<ContextType | null>, Setter<ContextType | null>]>();

type Props = {
	children: JSX.Element;
};
export function ReleaseSideViewProvider(props: Props) {
	const [release, setRelease] = createSignal<ContextType | null>(null);

	return (
		<ReleaseSideViewContext.Provider value={[release, setRelease]}>
			{props.children}
		</ReleaseSideViewContext.Provider>
	);
}

export const useReleaseSideViewData = () => useContext(ReleaseSideViewContext)!;
