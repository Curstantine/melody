import { createContext, createSignal, useContext } from "solid-js";

export default class SetupViewModel {
	page = createSignal<"create" | "recover" | "scan">("create");
	pageData: { name: string; scanLocations: string[] } | null = null;

	constructor() {
		this.goToScan.bind(this);
		this.goToCreate.bind(this);
	}

	public goToCreate() {
		const [, setPage] = this.page;
		this.pageData = null;
		setPage("create");
	}

	public goToScan(name: string, scanLocations: string[]) {
		const [, setPage] = this.page;
		this.pageData = { name, scanLocations };
		setPage("scan");
	}
}

export const SetupViewContext = createContext<SetupViewModel>(undefined, { name: "SetupViewContext" });
export const useSetupView = () => useContext(SetupViewContext)!;
