import { createSignal, For } from "solid-js";

import TabBarDestination, { DragEventFunc, type Props as Destination } from "@/components/TabBar/Destination";

export default function TabBar() {
	// eslint-disable-next-line prefer-const
	let destinationListRef: HTMLDivElement | undefined = undefined;

	const [destinations, setDestinations] = createSignal<Pick<Destination, "id" | "label" | "href">[]>([
		{ id: "music", label: "Music", href: "/(shared)/music" },
		{ id: "artists", label: "Artists", href: "/(shared)/artists" },
		{ id: "playlists", label: "Playlists", href: "/(shared)/playlists" },
		{ id: "np", label: "Now Playing", href: "/(shared)/np" },
	]);

	// TODO: Add support for following the cursor
	const onDestinationDragStart: DragEventFunc = (_, __) => {};

	/**
	 * Thinking that the destinations are an array with a predefined index to each position.
	 * If we were to listen to the drag event and deduce the relative X width the dragging element
	 * and each element on the list has, we can
	 */
	const onDestinationDrop: DragEventFunc = (id, e) => {
		if (!destinationListRef) return;

		const items = destinations();
		const draggedIndex = items.findIndex(item => item.id === id);
		let droppedIndex = draggedIndex;

		const rectCache: DOMRect[] = [];
		for (let i = 0; i < destinationListRef.children.length; i++) {
			const isLast = i === destinationListRef.children.length - 1;

			const child = destinationListRef.children[i];
			const childRect = rectCache[i] ?? child.getBoundingClientRect();

			const nextChild = isLast ? child : destinationListRef.children[i + 1]!;
			const nextChildRect = nextChild.getBoundingClientRect();
			if (!isLast) rectCache[i + 1] = nextChildRect;

			console.log("element at", child.id, {
				childId: child.id,
				childLeft: childRect.left,
				childRight: childRect.right,
				nextChildId: nextChild.id,
				nextChildLeft: nextChildRect.left,
				nextChildRight: nextChildRect.right,
			});

			// Cases where drag to right is bigger than the childRect end.
			if (i === destinationListRef.children.length - 1 && e.clientX >= childRect.right) {
				droppedIndex = i;
				break;
			}

			if (e.clientX > childRect.left && e.clientX < nextChildRect.left) {
				droppedIndex = i;
				break;
			}
		}

		const newItems = [...items];
		const itemAtDraggedIndex = newItems[draggedIndex];
		newItems[draggedIndex] = newItems[droppedIndex];
		newItems[droppedIndex] = itemAtDraggedIndex;

		setDestinations(newItems);
	};

	return (
		<div class="h-8 flex items-center border-b-1 border-b-border-main border-b-solid px-4">
			<div
				ref={destinationListRef}
				class="h-full flex flex-1 items-center gap-4 text-sm text-text-2"
			>
				<For each={destinations()}>
					{(x) => (
						<TabBarDestination
							{...x}
							onDragStart={onDestinationDragStart}
							onDragEnd={onDestinationDrop}
						/>
					)}
				</For>
				<button class="h-5 w-4 inline-flex items-center justify-center text-text-3">
					<div class="i-symbols-add-rounded h-4 w-4" />
				</button>
			</div>
		</div>
	);
}
