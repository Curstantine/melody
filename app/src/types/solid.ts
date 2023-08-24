import type { Accessor, Setter } from "solid-js";

export type SignalObject<T> = {
	get: Accessor<T>;
	set: Setter<T>;
};
