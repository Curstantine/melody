import { createSignal } from "solid-js";

import type BackendError from "@/errors/backend";
import type {
	LibraryCommand,
	LibraryCreateParameters,
	LibraryEvent,
	LibraryGenericActionPayload,
} from "@/types/backend";
import { invoke, listen } from "@/utils/tauri";

import { useSetupView } from "@/pages/setup/index.model";

export default class SetupScanViewModel {
	payload = createSignal<LibraryGenericActionPayload | null>(null);
	error = createSignal<BackendError | null>(null);

	setupViewModel = useSetupView();

	constructor() {
		this.startScan = this.startScan.bind(this);
	}

	public async startScan(name: string, scanLocations: string[]) {
		const [, setPayload] = this.payload;
		const creationWorkflow = invoke<LibraryCommand, LibraryCreateParameters>("create_library", {
			name,
			scanLocations,
		});
		const unlisten = await listen<LibraryEvent, LibraryGenericActionPayload>("library_scan", (event) => {
			setPayload(event.payload);
			console.log(event);
		});

		const creationResult = await creationWorkflow;
		if (creationResult.isErr()) {
			const [, setError] = this.error;
			setError(creationResult.unwrapErr());
		}

		unlisten();
	}
}
