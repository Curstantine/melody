import { For } from "solid-js";

import { InlinedArtist } from "@/types/backend/generic";
import type { Release } from "@/types/backend/release";
import type { Track } from "@/types/backend/track";

export default function AlbumSideView() {
	const bogus: Release = {
		type: "single",
		name: "She's Taken By The Devil",
		artists: [
			{
				id: 212,
				join: " feat. ",
				credited_as: "Viceroy",
			},
			{
				id: 123,
				join: "",
				credited_as: "fred",
			},
		],
	};

	const boogus: Track = {
		title: "She's Taken By The Devil",
		track_number: 1,
		artists: [
			{
				id: 212,
				join: " feat. ",
				credited_as: "Viceroy",
			},
			{
				id: 123,
				join: "",
				credited_as: "fred",
			},
		],
	};

	const artistJoin = (artists: InlinedArtist[] | null) =>
		artists?.map(({ join, credited_as }) => `${credited_as ?? "unknown"}${join}`);

	return (
		<div class="min-w-sm flex flex-col border-l-(1 border-main solid)">
			<div class="h-6 inline-flex items-center border-b-(1 border-main solid) px-2">
				<span class="text-sm text-text-3">Release Details</span>
			</div>

			<div class="flex flex-col gap-2 overflow-y-auto py-2">
				<For each={new Array(12).fill(boogus).map((o, i) => ({ ...o, track_number: i + 1 })) as Track[]}>
					{(item) => (
						<div class="grid grid-flow-col grid-rows-2 items-center px-2 py-1">
							<span class="row-span-2 w-4 self-start text-sm text-text-3">{item.track_number}.</span>

							<span class="col-span-7 text-sm leading-snug text-text-1">
								{item.title}
							</span>
							<span class="text-xs leading-snug text-text-2">
								{artistJoin(item.artists)}
							</span>

							<button class="row-span-2 h-6 w-6 p-0 button-layout button-template-text">
								<div class="i-symbols-more-vert" />
							</button>
						</div>
					)}
				</For>
			</div>

			<div class="flex flex-col border-t-(1 border-main solid) px-2 py-4">
				<h1 class="text-xl font-medium leading-tight text-text-1">{bogus.name}</h1>
				<span class="text-sm leading-tight text-text-2">{artistJoin(bogus.artists)}</span>
			</div>
		</div>
	);
}
