import { InlinedArtist } from "@/types/backend/generic";
import { Person } from "@/types/backend/person";
import type { Release } from "@/types/backend/release";
import type { Resource } from "@/types/backend/resource";
import { convertFileSrc } from "@tauri-apps/api/tauri";
import { Show } from "solid-js";

type Props = {
	id: number;
	release: Release;
	artists: Record<number, Person>;
	cover?: Resource;
};

export default function ReleaseListItem(props: Props) {
	const getArtist = (ref: InlinedArtist): string => {
		const artist = props.artists[ref.id];
		return `${ref.credited_as ?? artist.name}${ref.join ?? ""}`;
	};

	return (
		<div class="h-52 w-42 flex flex-col gap-2">
			<div class="h-42 w-42 inline-flex items-center justify-center rounded">
				<Show when={props.cover}>
					{(cover) => <img class="h-42 w-42" src={convertFileSrc(cover().path)} />}
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
