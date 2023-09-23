import { listen } from "@tauri-apps/api/event";
import { createSignal } from "solid-js";

import type { LibraryCreateParameters, LibraryEvent, LibraryGenericActionPayload } from "@/types/backend";
import { invoke } from "@/utils/tauri";

import { useSetupView } from "@/pages/setup/index.model";

export default class SetupScanViewModel {
	payload = createSignal<LibraryGenericActionPayload | null>(null);
	setupViewModel = useSetupView();

	constructor() {
		this.startScan = this.startScan.bind(this);
	}

	public async startScan(name: string, scanLocations: string[]) {
		try {
			const [, setPayload] = this.payload;
			const creationWorkflow = invoke<LibraryEvent, LibraryCreateParameters>("library_scan", {
				name,
				scanLocations,
			});
			const unlisten = await listen<LibraryGenericActionPayload>("library-scan", (event) => {
				setPayload(event.payload);
			});

			await creationWorkflow;
			unlisten();
		} catch (error) {
			console.error(error);
		}
	}
}
