import { createSignal } from "solid-js";

import type BackendError from "@/errors/backend";
import type { LibraryCreateParameters, LibraryGenericActionPayload } from "@/types/backend";
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

		const unlisten = await listen<LibraryGenericActionPayload>(
			"library_scan",
			(event) => setPayload(event.payload),
		);

		const creationResult = await invoke<LibraryCreateParameters>("create_library", {
			name,
			scanLocations,
		});

		if (creationResult.isErr()) {
			const [, setError] = this.error;
			setError(creationResult.unwrapErr());
		}

		unlisten();
	}
}
