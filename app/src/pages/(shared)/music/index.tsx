import { appWindow } from "@tauri-apps/api/window";
import { createEffect, createResource, createSignal, For, Match, onCleanup, onMount, Switch, untrack } from "solid-js";

import type { Person } from "@/types/backend/person";
import type { DisplayReleases, Release, ReleasesGetParameters } from "@/types/backend/release";

import { invoke } from "@/utils/tauri";

import ErrorCard from "@/components/Card/Error";
import ReleaseListItem from "@/components/ListItems/Release";
import {
	type ContextDataType as ReleaseSideViewData,
	useReleaseSideViewData,
} from "@/components/ReleaseSideView/context";

const getData = async (): Promise<DisplayReleases> => {
	const p = await invoke<DisplayReleases, ReleasesGetParameters>("get_display_releases");
	return p.unwrap();
};

export default function Home() {
	const [gridXSize, setGridXSize] = createSignal(4);
	const [data] = createResource(getData, {});

	const {
		visible: [isSideViewVisible],
		sizer: [sideViewXSize],
		open: openSideBar,
	} = useReleaseSideViewData();

	// eslint-disable-next-line prefer-const
	let ref: HTMLDivElement | undefined = undefined;
	const listeners: Array<() => void> = [];

	let remUnit = 16;
	let unrestrictedRemWidth = 0;

	const refreshContainerSizing = () => {
		const { width } = ref!.getBoundingClientRect();
		unrestrictedRemWidth = width / remUnit;
	};

	const adjustGridSize = () => {
		const isSideVisible = untrack(isSideViewVisible);
		const sideViewSize = untrack(sideViewXSize);
		const widthRem = unrestrictedRemWidth - (isSideVisible ? sideViewSize : 0);

		// 10.5 item width + 1 gap
		const itemLength = widthRem / 11.5;
		setGridXSize(Math.floor(itemLength));
	};

	const onReleaseItemClick = (releaseId: number, release: Release, artists: Record<number, Person>) => {
		openSideBar({ release, artists, releaseId } satisfies ReleaseSideViewData);
	};

	onMount(async () => {
		const { fontSize } = getComputedStyle(ref!);
		remUnit = parseFloat(fontSize);

		refreshContainerSizing();
		adjustGridSize();

		const resizeListener = await appWindow.onResized(() => {
			refreshContainerSizing();
			adjustGridSize();
		});

		listeners.push(resizeListener);
	});

	onCleanup(() => listeners.forEach((fn) => fn()));

	createEffect(() => {
		isSideViewVisible();
		adjustGridSize();
	});

	return (
		<div
			ref={ref}
			style={`grid-template-columns: repeat(${gridXSize()}, minmax(0, 1fr));`}
			class="grid w-full transform-gpu items-center justify-center gap-4 overflow-y-auto p-4"
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
									onClick={onReleaseItemClick}
								/>
							)}
						</For>
					)}
				</Match>
			</Switch>
		</div>
	);
}
