import { appWindow } from "@tauri-apps/api/window";
import { createResource, createSignal, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";

import type BackendError from "@/errors/backend";
import type { ReleaseEntity, ReleasesGetParameters } from "@/types/backend/release";
import type Result from "@/utils/result";

import { useAppModel } from "@/AppModel";
import { invoke } from "@/utils/tauri";

import ReleaseListItem from "@/components/ListItems/Release";

const getData = async (libraryId: number | undefined): Promise<Result<ReleaseEntity[], BackendError>> => {
	const p = await invoke<ReleaseEntity[], ReleasesGetParameters>("get_releases", { libraryId: libraryId! });
	console.log(p);

	return p;
};

export default function Home() {
	const { currentLibraryId: [currentLibraryId] } = useAppModel();
	const [gridXSize, setGridXSize] = createSignal(4);
	const [data] = createResource(currentLibraryId, getData);

	// eslint-disable-next-line prefer-const
	let ref: HTMLDivElement | undefined = undefined;
	const listeners: Array<() => void> = [];

	onMount(async () => {
		const remConst = parseFloat(getComputedStyle(document.documentElement).fontSize);
		const resizeListener = await appWindow.onResized(() => {
			const { width } = ref!.getBoundingClientRect();
			const widthRem = width / remConst;

			// 10.5 item width + 1 gap
			const itemLength = widthRem / 11.5;
			setGridXSize(Math.floor(itemLength));
		});

		listeners.push(resizeListener);
	});

	onCleanup(() => listeners.forEach((fn) => fn()));

	return (
		<div
			ref={ref}
			style={`grid-template-columns: repeat(${gridXSize()}, minmax(0, 1fr));`}
			class="grid max-h-[calc(100%-2rem)] items-center justify-center gap-4 overflow-y-auto p-4"
		>
			<Show
				when={data()}
				fallback={
					<For each={Array.from({ length: 10 })}>
						{(_) => <div class="h-42 w-42 rounded bg-background-secondary"></div>}
					</For>
				}
			>
				{(data) => (
					<Switch>
						<Match when={data().isOk()}>
							<For each={data().unwrap()}>
								{(release) => <ReleaseListItem {...release} />}
							</For>
						</Match>
						<Match when={data().isErr()}>
							<span>{data().unwrapErr().message}</span>
						</Match>
					</Switch>
				)}
			</Show>
		</div>
	);
}
