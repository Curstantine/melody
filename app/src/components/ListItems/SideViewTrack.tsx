import { Person } from "@/types/backend/person";
import { Track } from "@/types/backend/track";
import { joinInlinedArtists } from "@/utils/strings";

type Props = {
	artists: Record<number, Person>;
	track: Track;
};

export function SideViewTrackListItem(props: Props) {
	return (
		<div class="flex items-center py-1 pl-4 pr-2">
			<span class="w-8 self-start text-sm text-text-3">
				{props.track.track_number}.
			</span>

			<div class="flex flex-1 flex-col pr-4">
				<span class="text-sm leading-snug text-text-1">
					{props.track.title}
				</span>
				<span class="text-xs leading-snug text-text-2">
					{joinInlinedArtists(props.track.artists, props.artists)}
				</span>
			</div>

			<button class="h-6 w-6 icon-button-layout button-template-text">
				<div class="i-symbols-more-vert" />
			</button>
		</div>
	);
}

export function SideViewTrackListSkeleton() {
	return (
		<div class="flex animate-pulse items-center py-1 pl-4 pr-2 space-x-4">
			<div class="h-4 w-4 self-start bg-background-secondary" />

			<div class="flex flex-1 flex-col gap-2 pr-4">
				<div class="h-4 w-32 bg-background-secondary text-sm leading-snug" />
				<div class="h-4 w-12 bg-background-secondary text-xs leading-snug" />
			</div>

			<div class="h-6 w-6 icon-button-layout bg-background-secondary" />
		</div>
	);
}
