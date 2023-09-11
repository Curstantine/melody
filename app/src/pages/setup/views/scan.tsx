import { Show } from "solid-js";

import SetupScanViewModel from "@/pages/setup/views/scan.model";

export type Props = {
	libraryName: string;
	scanLocations: string[];
};

export default function SetupScanView(props: Props) {
	const { payload: [payload] } = new SetupScanViewModel(props);

	return (
		<Show when={payload()}>
			{(payload) => (
				<div class="flex flex-col items-center justify-center">
					<span>{payload().action_type === "reading" ? "Reading" : "Indexing"}</span>
					<span>{payload().path}</span>
					<span>({payload().current}/{payload().total})</span>
				</div>
			)}
		</Show>
	);
}
