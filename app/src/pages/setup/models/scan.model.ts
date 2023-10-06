import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";

import type BackendError from "@/errors/backend";
import type { LibraryCreateParameters, LibraryGenericActionPayload } from "@/types/backend";
import { invoke, listen } from "@/utils/tauri";

export default class SetupScanViewModel {
	private navigate = useNavigate();

	payload = createSignal<LibraryGenericActionPayload | null>(null);
	error = createSignal<BackendError | null>(null);

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

		unlisten();

		if (creationResult.isErr()) {
			const [, setError] = this.error;
			return setError(creationResult.unwrapErr());
		}

		this.navigate("/home", { replace: true, state: { name } });
	}
}
