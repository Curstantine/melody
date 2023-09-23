import { createSignal } from "solid-js";

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
	setupViewModel = useSetupView();

	constructor() {
		this.startScan = this.startScan.bind(this);
	}

	public async startScan(name: string, scanLocations: string[]) {
		try {
			const [, setPayload] = this.payload;
			const creationWorkflow = invoke<LibraryCommand, LibraryCreateParameters>("create_library", {
				name,
				scanLocations,
			});
			const unlisten = await listen<LibraryEvent, LibraryGenericActionPayload>("library_scan", (event) => {
				setPayload(event.payload);
				console.log(event);
			});

			await creationWorkflow;
			unlisten();
		} catch (error) {
			console.error(error);
		}
	}
}
