import { createResource, For, Match, type ResourceFetcher, Show, Switch } from "solid-js";

import type { DisplayTrackList, GetTrackListParameters } from "@/types/backend/track";

import { joinInlinedArtists } from "@/utils/strings";
import { invoke } from "@/utils/tauri";

import { SideViewTrackListItem, SideViewTrackListSkeleton } from "@/components/ListItems/SideViewTrack";
import {
	type ContextDataType,
	ReleaseSideViewProvider,
	useReleaseSideViewData,
} from "@/components/ReleaseSideView/context";

const getData: ResourceFetcher<number, DisplayTrackList> = async (releaseId: number) => {
	const x = await invoke<DisplayTrackList, GetTrackListParameters>("get_track_list_for_release", { releaseId });
	return x.unwrap();
};

export default function ReleaseSideView() {
	const {
		visible: [isVisible],
		data: [viewData],
		sizer: [xSize],
		close,
	} = useReleaseSideViewData();
	const [data] = createResource(() => viewData()?.releaseId, getData);

	// Convenience un-wrappers for non-null data.
	// Guard these by Show or something.
	const release = () => viewData()!.release;
	const artists = () => viewData()!.artists;

	const width = () => isVisible() ? `${xSize()}rem` : "0rem";

	return (
		<div
			class="flex flex-col transform-gpu border-l-(1 border-main solid) use-transition-standard"
			style={{ width: width(), "min-width": width(), "max-width": width() }}
			classList={{
				"translate-x-0": isVisible(),
				"translate-x-full": !isVisible(),
			}}
		>
			<Show when={viewData()}>
				<div class="min-h-8 inline-flex items-center justify-between border-b-(1 border-main solid) pl-4 pr-2">
					<span class="text-sm text-text-3">Release Details</span>
					<button class="h-6 w-6 icon-button-layout button-template-text" onClick={close}>
						<div class="i-symbols-close" />
					</button>
				</div>

				<div class="flex flex-col gap-2 overflow-y-auto py-2">
					<Switch>
						<Match when={data.loading}>
							<For each={Array.from({ length: 4 })}>
								{(_) => <SideViewTrackListSkeleton />}
							</For>
						</Match>
						<Match when={data()}>
							{(trackList) => (
								<For each={trackList().tracks}>
									{(track) => <SideViewTrackListItem track={track} artists={trackList().artists} />}
								</For>
							)}
						</Match>
					</Switch>
				</div>

				<div class="flex flex-col border-t-(1 border-main solid) px-2 py-4">
					<h1 class="text-xl font-medium leading-tight text-text-1">{release().name}</h1>
					<span class="text-sm leading-tight text-text-2">
						{joinInlinedArtists(release().artists, artists())}
					</span>
				</div>
			</Show>
		</div>
	);
}

export { ContextDataType, ReleaseSideViewProvider };
