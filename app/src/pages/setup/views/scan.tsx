import { onMount, Show } from "solid-js";

import CircularLoader from "@/components/Loader/Circular";

import { useSetupView } from "@/pages/setup/index.model";
import SetupScanViewModel from "@/pages/setup/views/scan.model";

export default function SetupScanView() {
	const { pageData } = useSetupView();

	const viewModel = new SetupScanViewModel();
	const { payload: [payload] } = viewModel;

	onMount(() => {
		if (!pageData) throw new Error("No page data");
		setTimeout(() => viewModel.startScan(pageData.name, pageData.scanLocations), 1000);
	});

	return (
		<div
			class="max-h-md max-w-lg w-full flex flex-col overflow-auto rounded"
			classList={{
				"items-center": payload() === null,
				"b-(1 border-secondary) bg-background-secondary p-4": payload() !== null,
			}}
		>
			<Show when={payload()} fallback={<CircularLoader />}>
				{(payload) => (
					<>
						<span class="text-2xl leading-tight font-orbiter-display text-text-1">
							Scanning your library
						</span>
						<span class="leading-tight font-orbiter-text text-text-2">
							This action might take some time...
						</span>

						<div class="mt-8 flex gap-1 text-text-2">
							<span>{payload().action_type === "reading" ? "Reading" : "Indexing"}</span>
							<span>({payload().current}/{payload().total})</span>
						</div>
						<span class="text-sm text-text-3">{payload().path}</span>
					</>
				)}
			</Show>
		</div>
	);
}
