import { useLocation, useNavigate } from "@solidjs/router";
import { createSignal, Match, onMount, Switch } from "solid-js";

import BackendError from "@/errors/backend";
import DataError from "@/errors/data";
import { useAppModel } from "@/models/App";
import type {
	LibraryActionData,
	LibraryActionError,
	LibraryActionPayload,
	LibraryCreateParameters,
} from "@/types/backend/library";
import { invoke, listen } from "@/utils/tauri";

import ErrorCard from "@/components/Card/Error";
import CircularLoader from "@/components/Loader/Circular";
import { createStore } from "solid-js/store";

export type LocationState = {
	name: string;
	scanLocations: string[];
};

export default function SetupScanView() {
	const [payload, setPayload] = createSignal<LibraryActionData | null>(null);
	const [error, setError] = createSignal<BackendError | null>(null);
	const [silentErrors, setSilentErrors] = createStore<{ path: string; error: BackendError }[]>([]);

	const isLoading = () => payload() === null && error() === null;

	const navigate = useNavigate();
	const appModel = useAppModel();
	const location = useLocation<LocationState>();

	const startScan = async (name: string, scanLocations: string[]) => {
		const unlisten = await listen<LibraryActionPayload>(
			"library_scan",
			(event) => {
				switch (event.payload.type) {
					case "ok":
						setPayload(event.payload.data as LibraryActionData);
						break;
					case "error": {
						const { path, error } = event.payload.data as LibraryActionError;
						setSilentErrors((others) => [...others, { path, error: BackendError.fromStupidError(error) }]);
						break;
					}
				}
			},
		);

		const result = await invoke<null, LibraryCreateParameters>("create_library", { name, scanLocations });
		unlisten();

		if (result.isErr()) return setError(result.unwrapErr());
		navigate("/home", { replace: true, state: { name } });
	};

	onMount(() => {
		const name = location.state?.name;
		const scanLocations = location.state?.scanLocations;

		if (!name || !scanLocations) {
			const error = DataError.missingLocationState("/setup/scan", { name, scanLocations });
			return appModel.setAppError(error, false);
		}

		setTimeout(() => startScan(name, scanLocations!), 1000);
	});

	return (
		<div
			class="max-h-md max-w-lg w-full flex flex-col rounded"
			classList={{
				"items-center": isLoading(),
				"b-(1 border-secondary) bg-background-secondary p-4 overflow-y-auto": !isLoading(),
			}}
		>
			<Switch fallback={<CircularLoader />}>
				<Match when={error()}>
					{(error) => <ErrorCard data={{ error: error() }} />}
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
