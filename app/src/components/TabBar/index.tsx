import { createSignal, For } from "solid-js";

import TabBarDestination, { type Props as Destination } from "@/components/TabBar/Destination";

export default function TabBar() {
	const [destinations] = createSignal<Destination[]>([
		{ label: "Albums", href: "/(shared)/albums" },
		{ label: "Artists", href: "/(shared)/albums" },
		{ label: "Playlists", href: "/(shared)/playlists" },
		{ label: "Now Playing", href: "/(shared)/np" },
	]);

	return (
		<div class="h-8 flex items-center border-b-1 border-b-border-main border-b-solid px-4">
			<div class="w-16" />
			<div class="inline-flex items-center gap-4 text-sm text-text-2">
				<For each={destinations()}>{(x) => <TabBarDestination {...x} />}</For>
			</div>
		</div>
	);
}
