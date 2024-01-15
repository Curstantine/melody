import { convertFileSrc } from "@tauri-apps/api/tauri";
import { Show } from "solid-js";

import type { DisplayCover } from "@/types/backend/cover";
import type { InlinedArtist } from "@/types/backend/generic";
import type { Person } from "@/types/backend/person";
import type { Release } from "@/types/backend/release";

type Props = {
	id: number;
	release: Release;
	artists: Record<number, Person>;
	cover?: DisplayCover;
};

export default function ReleaseListItem(props: Props) {
	const getArtist = (ref: InlinedArtist): string => {
		const artist = props.artists[ref.id];
		return `${ref.credited_as ?? artist.name}${ref.join ?? ""}`;
	};

	const getCoverPath = (cover: DisplayCover) => {
		return convertFileSrc(cover.path);
	};

	return (
		<div class="h-52 w-42 flex flex-col gap-2">
			<div
				class="h-42 w-42 inline-flex items-center justify-center"
				classList={{ "bg-background-secondary rounded-md": props.cover === undefined }}
			>
				<Show when={props.cover} fallback={<div class="i-symbols-image h-8 w-8 text-text-3" />}>
					{(cover) => <img class="h-42 w-42 rounded-md" src={getCoverPath(cover())} />}
				</Show>
			</div>
			<div class="h-10 inline-flex flex-col">
				<span class="line-clamp-1 text-ellipsis text-sm font-orbiter-display-medium text-text-2">
					{props.release.name}
				</span>
				<span class="line-clamp-1 text-ellipsis text-xs text-text-3">
					{props.release.artists.map(getArtist).join("")}
				</span>
			</div>
		</div>
	);
}
