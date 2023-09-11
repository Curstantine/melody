import { listen } from "@tauri-apps/api/event";
import { createSignal } from "solid-js";

import type { Props as ScanViewProps } from "@/pages/setup/views/scan";
import type { LibraryCreateParameters, LibraryGenericActionPayload } from "@/types/backend";
import { invoke } from "@/utils/tauri";

export default class SetupScanViewModel {
	payload = createSignal<LibraryGenericActionPayload | null>(null);

	constructor({ libraryName, scanLocations }: ScanViewProps) {
		this.startScan(libraryName, scanLocations);
	}

	public async startScan(name: string, scanLocations: string[]) {
		try {
			const [, setPayload] = this.payload;
			const creationWorkflow = invoke<LibraryCreateParameters>("create_library", { name, scanLocations });
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
