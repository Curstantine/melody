import { createResource, For, Match, Show, Switch } from "solid-js";

import type { DisplayTrackList, GetTrackListParameters } from "@/types/backend/track";

import { joinInlinedArtists } from "@/utils/strings";
import { invoke } from "@/utils/tauri";

import {
	type ContextType,
	ReleaseSideViewProvider,
	useReleaseSideViewData,
} from "@/components/ReleaseSideView/context";

const getData = async (): Promise<DisplayTrackList> => {
	const x = await invoke<DisplayTrackList, GetTrackListParameters>("get_track_list_for_release");
	console.log(x);
	return x.unwrap();
};

export default function ReleaseSideView() {
	const [viewData] = useReleaseSideViewData();
	const [data] = createResource(getData, {});

	// Note(Curstantine): Convenience un-wrappers for non-null data.
	// Guard these by Show or something.
	const release = () => viewData()!.release;
	const artists = () => viewData()!.artists;

	return (
		<div class="min-w-sm flex flex-col border-l-(1 border-main solid)">
			<Show when={viewData()}>
				<div class="h-6 inline-flex items-center border-b-(1 border-main solid) px-2">
					<span class="text-sm text-text-3">Release Details</span>
				</div>

				<div class="flex flex-col gap-2 overflow-y-auto py-2">
					<Switch>
						<Match when={data.loading}>
							<For each={Array.from({ length: 10 })}>
								{(_) => <div class="h-18"></div>}
							</For>
						</Match>
						<Match when={data()}>
							{(trackList) => (
								<For each={trackList().tracks}>
									{(track) => (
										<div class="grid grid-flow-col grid-rows-2 items-center px-2 py-1">
											<span class="row-span-2 w-4 self-start text-sm text-text-3">
												{track.track_number}.
											</span>

											<span class="col-span-7 text-sm leading-snug text-text-1">
												{track.title}
											</span>
											<span class="text-xs leading-snug text-text-2">
												{joinInlinedArtists(track.artists, trackList().artists)}
											</span>

											<button class="row-span-2 h-6 w-6 p-0 button-layout button-template-text">
												<div class="i-symbols-more-vert" />
											</button>
										</div>
									)}
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

export { ContextType, ReleaseSideViewProvider };
