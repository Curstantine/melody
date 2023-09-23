import { open } from "@tauri-apps/api/dialog";
import { homeDir, join } from "@tauri-apps/api/path";
import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import { ulid } from "ulid";

import { useForm } from "@/hooks/form";
import { useAppModel } from "@/models/App";
import { useSetupView } from "@/pages/setup/index.model";

export default class SetupCreateViewModel {
	form = useForm();
	appModel = useAppModel();
	setupViewModel = useSetupView();

	mode = createSignal<"create" | "recover">();
	name = createSignal<string | null>(null);
	continuable = createSignal<boolean>(false);
	scanLocations = createStore<Array<{ id: string; location: string | null }>>([]);

	homeDirectory: string | null = null;

	constructor() {
		this.onConfirm = this.onConfirm.bind(this);
		this.onScanLocationFieldPress = this.onScanLocationFieldPress.bind(this);
		this.addScanLocation = this.addScanLocation.bind(this);
		this.removeScanLocation = this.removeScanLocation.bind(this);
		this.setScanLocation = this.setScanLocation.bind(this);

		this.initialize();
	}

	private async initialize() {
		this.homeDirectory = await homeDir();
		this.addScanLocation(undefined, await join(this.homeDirectory, "Music"));
	}

	public async onConfirm() {
		const [name] = this.name;
		const [locations] = this.scanLocations;
		this.setupViewModel.goToScan(name()!, locations.map((x) => x.location!));
	}

	public async onScanLocationFieldPress(id: string, e?: MouseEvent) {
		e?.preventDefault();

		const result = await open({
			directory: true,
			multiple: false,
			defaultPath: this.homeDirectory ?? await homeDir(),
			title: "Select a location to add to scan paths.",
		});

		if (result === null || typeof result !== "string") return;
		this.setScanLocation(id, result);
	}

	public addScanLocation(e?: MouseEvent, location?: string) {
		e?.preventDefault();

		const [locations, setLocations] = this.scanLocations;
		setLocations([...locations, { id: ulid(), location: location ?? null }]);
	}

	public removeScanLocation(id: string, e?: MouseEvent) {
		e?.preventDefault();

		const [locations, setLocations] = this.scanLocations;
		setLocations(locations.filter((x) => x.id !== id));
	}

	public setScanLocation(id: string, location: string) {
		const [, setLocation] = this.scanLocations;
		setLocation((x) => x.id === id, "location", location);
	}
}
