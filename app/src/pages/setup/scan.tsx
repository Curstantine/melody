import { Match, onMount, Switch } from "solid-js";

import CircularLoader from "@/components/Loader/Circular";

import DataError from "@/errors/data";
import { useAppModel } from "@/models/App";
import SetupScanViewModel from "@/pages/setup/models/scan.model";
import { useLocation } from "@solidjs/router";

export type LocationState = {
	name: string;
	scanLocations: string[];
};

export default function SetupScanView() {
	const viewModel = new SetupScanViewModel();
	const { payload: [payload], error: [error] } = viewModel;

	const isInLoadingState = () => payload() === null && error() === null;

	const appModel = useAppModel();
	const location = useLocation<LocationState>();

	onMount(() => {
		const name = location.state?.name;
		const scanLocations = location.state?.scanLocations;

		if (!name || !scanLocations) {
			const error = DataError.missingLocationState("/setup/scan", { name, scanLocations });
			return appModel.setAppError(error, false);
		}

		setTimeout(() => viewModel.startScan(name, scanLocations!), 1000);
	});

	return (
		<div
			class="max-h-md max-w-lg w-full flex flex-col rounded"
			classList={{
				"items-center": isInLoadingState(),
				"b-(1 border-secondary) bg-background-secondary p-4 overflow-y-auto": !isInLoadingState(),
			}}
		>
			<Switch fallback={<CircularLoader />}>
				<Match when={error()}>
					{(error) => (
						<>
							<span class="text-2xl leading-tight font-orbiter-display text-text-1">
								An error occurred
							</span>
							<span class="leading-tight font-orbiter-text text-text-2">
								{error().message}
								{error().context}
							</span>
						</>
					)}
				</Match>
				<Match when={payload()}>
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
				</Match>
			</Switch>
		</div>
	);
}
