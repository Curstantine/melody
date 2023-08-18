import { appWindow } from "@tauri-apps/api/window";
import { createSignal, onMount } from "solid-js";
import { Portal } from "solid-js/web";

import TitleBarButton from "@/components/TitleBar/Button";

export default function TitleBar() {
	const [isMaximized, setMaximizeStatus] = createSignal(false);

	let draggableRef: HTMLDivElement | undefined;

	onMount(() => {
		appWindow.isMaximized()
			.then((maximized) => setMaximizeStatus(maximized))
			.catch((e) => console.error(e));
	});

	const toggleMaximize = async () => {
		await appWindow.toggleMaximize();
	};

	const minimize = async () => {
		await appWindow.minimize();
	};

	const close = async () => {
		await appWindow.close();
	};

	return (
		<Portal mount={document.getElementById("titlebar")!}>
			<div class="inset-x-0 h-9 flex items-center justify-between border-b-1 border-b-border border-b-solid">
				<div data-tauri-drag-region ref={draggableRef!} class="h-full inline-flex flex-1 items-center">
					<span class="select-none pl-4 text-sm font-orbiter-deck-medium">Melody</span>
				</div>
				<div class="h-full inline-flex">
					<TitleBarButton icon="i-mdi-minimize" onClick={minimize} />
					<TitleBarButton
						icon={isMaximized() ? "i-mdi-window-restore" : "i-mdi-window-maximize"}
						onClick={toggleMaximize}
					/>
					<TitleBarButton type="close" icon="i-symbols-close" onClick={close} />
				</div>
			</div>
		</Portal>
	);
}
