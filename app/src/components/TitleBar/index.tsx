import { appWindow } from "@tauri-apps/api/window";
import { createSignal, onCleanup, onMount } from "solid-js";
import { Portal } from "solid-js/web";

import WindowError from "@/errors/window";
import { useAppModel } from "@/models/App";
import Result from "@/utils/result";

import TitleBarButton from "@/components/TitleBar/Button";

export default function TitleBar() {
	const appModel = useAppModel();
	const [isMaximized, setMaximizeStatus] = createSignal(false);
	const [isFocused, setFocusStatus] = createSignal(false);

	let draggableRef: HTMLDivElement | undefined;
	const listeners: Array<() => void> = [];

	const checkMaximizeStatus = async () => {
		const isMax = await Result.runAsync(() => appWindow.isMaximized(), WindowError.maximizeState);

		if (isMax.isErr()) {
			const e = isMax.unwrapErr();
			return appModel.setAppError(e, true);
		}

		setMaximizeStatus(isMax.unwrap());
	};

	const toggleMaximize = async () => await appWindow.toggleMaximize();
	const minimize = async () => await appWindow.minimize();
	const close = async () => await appWindow.close();

	onMount(async () => {
		await checkMaximizeStatus();

		const isFocus = await Result.runAsync(() => appWindow.isFocused(), WindowError.focusState);
		if (isFocus.isErr()) {
			const e = isFocus.unwrapErr();
			return appModel.setAppError(e, true);
		}

		setFocusStatus(isFocus.unwrap());

		const maximizeListener = await appWindow.onResized(async () => await checkMaximizeStatus());
		const focusListener = await appWindow.onFocusChanged(({ payload }) => setFocusStatus(payload));

		listeners.push(maximizeListener, focusListener);
	});

	onCleanup(() => listeners.forEach((unlisten) => unlisten()));

	return (
		<Portal mount={document.getElementById("titlebar")!}>
			<div
				class="inset-x-0 h-8 flex items-center justify-between border-b-1 border-b-border-main border-b-solid"
				classList={{
					"bg-background-secondary text-text-3": !isFocused(),
					"bg-background-main text-text-2": isFocused(),
				}}
			>
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
