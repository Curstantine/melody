import { createEffect, createResource, createSignal, For, Match, Show, Switch } from "solid-js";

import type { DisplayTrackList, GetTrackListParameters } from "@/types/backend/track";

import { joinInlinedArtists } from "@/utils/strings";
import { invoke } from "@/utils/tauri";

import {
	type ContextType,
	ReleaseSideViewProvider,
	useReleaseSideViewData,
} from "@/components/ReleaseSideView/context";

const getData = async (releaseId: number): Promise<DisplayTrackList> => {
	const x = await invoke<DisplayTrackList, GetTrackListParameters>("get_track_list_for_release", { releaseId });
	console.log(x);
	return x.unwrap();
};

export default function ReleaseSideView() {
	const [viewData, setViewData] = useReleaseSideViewData();
	const [data] = createResource(() => viewData()?.releaseId, getData);

	// This boolean controls the overall visibility of the sheet.
	// Used to guard data when a close is initiated to let transitions finish before clearing the signal.
	//
	// Whenever viewData is set to a non-nullish value, opened should be true.
	// And when the setVisibility is set to false, viewData should be set to null.
	const [opened, setVisibility] = createSignal(false);

	// Convenience un-wrappers for non-null data.
	// Guard these by Show or something.
	const release = () => viewData()!.release;
	const artists = () => viewData()!.artists;

	createEffect(() => {
		if (viewData()) return setVisibility(true);
		if (!opened()) return setTimeout(() => setViewData(null), 1500);
	});

	return (
		<div
			class="flex flex-col transform-gpu border-l-(1 border-main solid) use-transition-standard"
			classList={{
				"translate-x-0 w-xl": opened(),
				"translate-x-full w-0": !opened(),
			}}
		>
			<Show when={viewData()}>
				<div class="min-h-8 inline-flex items-center justify-between border-b-(1 border-main solid) px-4">
					<span class="text-sm text-text-3">Release Details</span>
					<button class="button-layout" onClick={() => setVisibility(false)}>
						<div class="i-symbols-close" />
					</button>
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
										<div class="flex items-center px-4 py-1">
											<span class="w-8 self-start text-sm text-text-3">
												{track.track_number}.
											</span>

											<div class="flex flex-1 flex-col pr-4">
												<span class="text-sm leading-snug text-text-1">
													{track.title}
												</span>
												<span class="text-xs leading-snug text-text-2">
													{joinInlinedArtists(track.artists, trackList().artists)}
												</span>
											</div>

											<button class="2 h-6 w-6 p-0 button-layout button-template-text">
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
