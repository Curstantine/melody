import { appWindow } from "@tauri-apps/api/window";
import { createResource, createSignal, For, Match, onCleanup, onMount, Switch } from "solid-js";

import type { DisplayReleases, ReleasesGetParameters } from "@/types/backend/release";

import { invoke } from "@/utils/tauri";

import ErrorCard from "@/components/Card/Error";
import ReleaseListItem from "@/components/ListItems/Release";

const getData = async (): Promise<DisplayReleases> => {
	const p = await invoke<DisplayReleases, ReleasesGetParameters>("get_display_releases");
	console.log(p);
	return p.unwrap();
};

export default function Home() {
	const [gridXSize, setGridXSize] = createSignal(4);
	const [data] = createResource(getData, {});

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
			class="max-h-[calc(100%-2rem)] items-center justify-center overflow-y-auto"
			classList={{ "grid gap-4 p-4": data() !== undefined, "flex h-full w-full": data() === undefined }}
		>
			<Switch>
				<Match when={data.loading}>
					<For each={Array.from({ length: 10 })}>
						{(_) => <div class="h-42 w-42 rounded bg-background-secondary"></div>}
					</For>
				</Match>
				<Match when={data.error}>
					{(error) => <ErrorCard data={{ error: error(), dismissible: false }} />}
				</Match>
				<Match when={data()}>
					{(data) => (
						<For each={Object.entries(data().releases)}>
							{([id, release]) => (
								<ReleaseListItem
									id={Number.parseInt(id)}
									release={release}
									artists={Object.fromEntries(
										release.artists.map(({ id }) => [id, data().artists[id]]),
									)}
									cover={release.cover_ids === undefined || release.cover_ids === null
										? undefined
										: data().covers[release.cover_ids[0]]}
								/>
							)}
						</For>
					)}
				</Match>
			</Switch>
		</div>
	);
}
