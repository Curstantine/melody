import { invoke } from "@tauri-apps/api";
import { open } from "@tauri-apps/api/dialog";
import { listen } from "@tauri-apps/api/event";
import { homeDir } from "@tauri-apps/api/path";
import { createEffect, createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import { ulid } from "ulid";

import { useForm } from "@/hooks/form";

export default class SetupViewModel {
	mode = createSignal<"create" | "recover">();
	name = createSignal<string | null>(null);
	continuable = createSignal<boolean>(false);
	scanLocations = createStore<Array<{ id: string; location: string | null }>>([]);
	form = useForm();

	constructor() {
		this.addScanLocation();

		const [, setContinuability] = this.continuable;
		const [locations] = this.scanLocations;

		createEffect(() => {
			const noErrors = Object.values(this.form.errors).filter((x) => !!x).length === 0;
			const noEmptyLocations = locations.filter((x) => !x.location).length === 0;
			const result = noErrors && noEmptyLocations;

			setContinuability(result);
		});
	}

	public onConfirm = async () => {
		// const [mode] = this.mode;
		const [name] = this.name;
		const [locations] = this.scanLocations;

		try {
			const creationWorkflow = invoke("create_library", {
				name: name(),
				scanLocations: locations.map((i) => i.location),
			});
			const unlisten = await listen("library-scan", (payload) => {
				console.log("library-scan", payload);
			});

			console.log(await creationWorkflow);
			unlisten();
		} catch (error) {
			console.error(error);
		}
	};

	public onScanLocationFieldPress = async (id: string, e?: MouseEvent) => {
		e?.preventDefault();

		const result = await open({
			directory: true,
			multiple: false,
			defaultPath: await homeDir(),
			title: "Select a location to add to scan paths.",
		});

		if (result === null || typeof result !== "string") return;
		this.setScanLocation(id, result);
	};

	public addScanLocation = (e?: MouseEvent) => {
		e?.preventDefault();

		const [locations, setLocations] = this.scanLocations;
		setLocations([...locations, { id: ulid(), location: null }]);
	};

	public removeScanLocation = (id: string, e?: MouseEvent) => {
		e?.preventDefault();

		const [locations, setLocations] = this.scanLocations;
		setLocations(locations.filter((x) => x.id !== id));
	};

	public setScanLocation = (id: string, location: string) => {
		const [, setLocation] = this.scanLocations;
		setLocation((x) => x.id === id, "location", location);
	};
}
