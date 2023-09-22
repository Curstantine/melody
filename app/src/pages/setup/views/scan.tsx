import { onMount, Show } from "solid-js";

import CircularLoader from "@/components/Loader/Circular";
import { useSetupView } from "@/pages/setup/model";
import SetupScanViewModel from "@/pages/setup/views/scan.model";

export default function SetupScanView() {
	const { pageData } = useSetupView();

	const viewModel = new SetupScanViewModel();
	const { payload: [payload] } = viewModel;

	onMount(() => {
		if (!pageData) throw new Error("No page data");
		viewModel.startScan(pageData.name, pageData.scanLocations);
	});

	return (
		<div class="flex flex-col items-center justify-center">
			<Show when={payload()} fallback={<CircularLoader />}>
				{(payload) => (
					<>
						<span>{payload().action_type === "reading" ? "Reading" : "Indexing"}</span>
						<span>{payload().path}</span>
						<span>({payload().current}/{payload().total})</span>
					</>
				)}
			</Show>
		</div>
	);
}
