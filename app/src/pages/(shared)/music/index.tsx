import { useAppModel } from "@/models/App";
import { invoke } from "@/utils/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { Accessor, createResource, createSignal, For, onCleanup, onMount } from "solid-js";

const getData = async (libraryId: Accessor<number | undefined>) => {
	const cmd = await invoke("get_releases", { library_id: libraryId });
};

export default function Home() {
	const { currentLibraryId: [currentLibraryId] } = useAppModel();
	const [gridXSize, setGridXSize] = createSignal(4);
	const [data] = createResource(currentLibraryId, getData);

	// eslint-disable-next-line prefer-const
	let ref: HTMLDivElement | undefined = undefined;
	const listeners: Array<() => void> = [];

	onMount(async () => {
		const remConst = parseFloat(getComputedStyle(document.documentElement).fontSize);

		const resizeListener = await appWindow.onResized(() => {
			const { width } = ref!.getBoundingClientRect();
			const widthRem = width / remConst;

			// 10.5 item width + 1 gap
			const itemLength = widthRem / 11.5;
			setGridXSize(Math.floor(itemLength));
		});

		listeners.push(resizeListener);
	});

	onCleanup(() => listeners.forEach((fn) => fn()));

	return (
		<div
			ref={ref}
			style={`grid-template-columns: repeat(${gridXSize()}, minmax(0, 1fr));`}
			class="grid h-full items-center justify-center gap-4 overflow-y-auto p-4"
		>
			<For each={Array.from({ length: 50 })}>
				{(_) => <div class="h-42 w-42 rounded bg-background-secondary"></div>}
			</For>
		</div>
	);
}
