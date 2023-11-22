import { useLocation, useNavigate } from "@solidjs/router";
import { createSignal, For, Match, onMount, Show, Switch } from "solid-js";
import { createStore } from "solid-js/store";

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

import { SHARED_PATHS } from "@/pages/(shared)";
import { SETUP_PATHS } from "@/pages/setup";

import ErrorCard from "@/components/Card/Error";
import CircularLoader from "@/components/Loader/Circular";

export type LocationState = {
	name: string;
	scanLocations: string[];
};

export default function SetupScanView() {
	const [showSilentErrors, setSilentErrorsVisibility] = createSignal(false);
	const [completed, setCompletion] = createSignal(false);

	const [payload, setPayload] = createSignal<LibraryActionData | null>(null);
	const [error, setError] = createSignal<BackendError | null>(null);
	const [silentErrors, setSilentErrors] = createStore<{ path: string; error: BackendError }[]>([]);

	const isLoading = () => payload() === null && error() === null;
	const completedWithSilentErrors = () => completed() && silentErrors.length > 0;

	const navigate = useNavigate();
	const appModel = useAppModel();
	const location = useLocation<LocationState>();

	const cont = () => navigate(SHARED_PATHS.MUSIC, { replace: true });

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
		if (silentErrors.length > 0) {
			setCompletion(true);
			return setSilentErrorsVisibility(true);
		}

		cont();
	};

	onMount(() => {
		const name = location.state?.name;
		const scanLocations = location.state?.scanLocations;

		if (!name || !scanLocations) {
			const error = DataError.missingLocationState(SETUP_PATHS.SCAN, { name, scanLocations });
			return appModel.setAppError(error, false);
		}

		setTimeout(() => startScan(name, scanLocations!), 1000);

		// let timeout = 0;
		// for (let i = 0; i < 10; i++) {
		// 	const p =
		// 		`c:\\Users\\Curstantine\\Music\\TempLib\\オンゲキシューターズ\\ONGEKI Vocal Party 05\\${i} bitter flavor - give it up to you (Game Size).opus`;
		// 	setTimeout(() => {
		// 		if (i % 2 == 0) {
		// 			setPayload({ action_type: "reading", total: 10, current: i, path: p });
		// 		} else {
		// 			setSilentErrors((x) => [...x, { path: p, error: BackendError.placeholder() }]);
		// 		}
		// 	}, timeout += 2000);
		// }
		// setTimeout(() => {
		// 	setCompletion(true);
		// 	setSilentErrorsVisibility(true);
		// }, timeout);
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
							<Switch>
								<Match when={!completed()}>
									<span class="select-none text-2xl leading-tight font-orbiter-display text-text-1">
										Scanning your library
									</span>
									<span class="select-none leading-tight font-orbiter-text text-text-2">
										This might take some time...
									</span>

									<div class="mt-8 flex select-none gap-1 text-text-2">
										<span>{payload().action_type === "reading" ? "Reading" : "Indexing"}</span>
										<span>({payload().current}/{payload().total})</span>
									</div>
									<span class="min-h-12 text-sm text-text-3">{payload().path}</span>
								</Match>
								<Match when={completedWithSilentErrors()}>
									<span class="select-none text-2xl leading-tight font-orbiter-display text-text-1">
										Finishing up
									</span>
									<span class="select-none py-2 leading-tight font-orbiter-text text-text-2">
										<p>Found errors while indexing the library.</p>
										<p>
											These errors could be ignored, but the resulting tracks would not be
											included in the indexed library.
										</p>
									</span>
								</Match>
							</Switch>

							<Show when={silentErrors.length > 0}>
								<button
									class="flex items-center pt-2"
									onClick={() => setSilentErrorsVisibility((x) => !x)}
								>
									<div class="i-symbols-error-outline mr-2 h-5 w-5 text-text-error" />
									<span class="flex-1 text-left text-sm text-text-2">
										Found {silentErrors.length} errors
									</span>

									<span class="text-xs text-text-3">Click to expand</span>
									<div
										class="i-symbols-expand-more h-5 w-5 text-text-3 transition-transform duration-standard ease-standard"
										classList={{ "rotate-180": showSilentErrors() }}
									/>
								</button>
							</Show>

							<Show when={showSilentErrors()}>
								<div class="mt-2 max-h-56 flex flex-col gap-1 overflow-y-auto text-xs text-text-3">
									<For each={silentErrors}>
										{(item) => (
											<span>
												<p class="text-text-error">{item.error.message}</p>
												<For each={item.error.getMultilineContext()}>
													{(line) => <p>{line}</p>}
												</For>
											</span>
										)}
									</For>
								</div>
							</Show>

							<Show when={completedWithSilentErrors()}>
								<div class="mt-4 flex justify-end gap-2">
									<button class="button-layout button-template-primary" onClick={cont}>
										Continue
									</button>
								</div>
							</Show>
						</>
					)}
				</Match>
			</Switch>
		</div>
	);
}
