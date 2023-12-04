import { Person } from "@/types/backend/person";
import type { Release } from "@/types/backend/release";
import type { Resource } from "@/types/backend/resource";

type Props = {
	id: number;
	release: Release;
	artists: Person[];
	cover?: Resource;
};

export default function ReleaseListItem(props: Props) {
	return (
		<div class="h-52 w-42 flex flex-col gap-2">
			<div class="h-42 w-42 rounded bg-background-secondary" />
			<div class="h-10 inline-flex flex-col">
				<span class="line-clamp-1 text-ellipsis text-sm font-orbiter-display-medium text-text-2">
					{props.release.name}
				</span>
				<span class="line-clamp-1 text-ellipsis text-xs text-text-3">
					{props.release.artists
						.map(({ join, credited_as, id }) => `${credited_as ?? id}${join ?? ""}`)
						.join("")}
				</span>
			</div>
		</div>
	);
}
