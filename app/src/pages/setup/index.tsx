import { Match, Switch } from "solid-js";

import SetupViewModel, { SetupViewContext } from "@/pages/setup/index.model";
import SetupCreateView from "@/pages/setup/views/create";
import SetupScanView from "@/pages/setup/views/scan";

export default function Setup() {
	const model = new SetupViewModel();
	const { page: [page] } = model;

	return (
		<SetupViewContext.Provider value={model}>
			<div class="h-full flex flex-col items-center justify-center px-6">
				<Switch>
					<Match when={page() === "create"}>
						<SetupCreateView />
					</Match>
					<Match when={page() === "scan"}>
						<SetupScanView />
					</Match>
				</Switch>
			</div>
		</SetupViewContext.Provider>
	);
}
