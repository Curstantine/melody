import { createContext, createSignal, useContext } from "solid-js";

import type { Props as ScanViewProps } from "@/pages/setup/views/scan";

export default class SetupViewModel {
	page = createSignal<"create" | "recover" | "scan">("create");
	pageData: ScanViewProps | null = null;

	constructor() {
		this.goToScan.bind(this);
	}

	public goToScan(libraryName: string, scanLocations: string[]) {
		const [, setPage] = this.page;
		this.pageData = { libraryName, scanLocations };
		setPage("scan");
	}
}

export const SetupViewContext = createContext<SetupViewModel>(undefined, { name: "SetupViewContext" });
export const useSetupView = () => useContext(SetupViewContext)!;
