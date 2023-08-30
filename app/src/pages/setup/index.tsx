import { SetupViewModelContext } from "@/contexts/models";

import SetupViewModel from "@/pages/setup/model";
import CreateView from "@/pages/setup/views/create";

export default function Setup() {
	const viewModel = new SetupViewModel();

	return (
		<SetupViewModelContext.Provider value={viewModel}>
			<div class="h-full flex flex-col items-center justify-center px-6">
				<CreateView />
			</div>
		</SetupViewModelContext.Provider>
	);
}
