import { createSignal, type Signal } from "solid-js";

import type { SignalObject } from "@/types/solid";

export function createSignalObject<T>(value: T): SignalObject<T> {
	const source = createSignal(value);
	return covertSignalToObject(source);
}

export function covertSignalToObject<T>(signal: Signal<T>): SignalObject<T> {
	return { get: signal[0], set: signal[1] };
}
