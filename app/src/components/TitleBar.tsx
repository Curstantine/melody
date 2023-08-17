import { appWindow } from "@tauri-apps/api/window";
import { createSignal, mergeProps, onMount } from "solid-js";
import type { JSX } from "solid-js/jsx-runtime";
import { Portal } from "solid-js/web";

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
			<div class="flex items-center justify-between h-9 inset-x-0 border-b-1 border-b-solid border-b-border ">
				<div data-tauri-drag-region ref={draggableRef!} class="flex-1 inline-flex items-center h-full">
					<span class="pl-4 select-none text-sm font-medium">Melody</span>
				</div>
				<div class="inline-flex h-full">
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

type TitleBarButtonProps = {
	icon: string;
	type?: "common" | "close";
	onClick?: JSX.EventHandler<HTMLButtonElement, MouseEvent>;
};
const TitleBarButton = (_props: TitleBarButtonProps) => {
	const props = mergeProps({ type: "common" }, _props);

	return (
		<button
			class="inline-flex items-center justify-center w-12 h-full duration-200 transition-colors"
			classList={{
				"hover:bg-titlebar-button-common-hover active:bg-titlebar-button-common-active disabled:bg-titlebar-button-common-disabled":
					props.type === "common",
				"hover:bg-titlebar-button-close-hover active:bg-titlebar-button-close-active disabled:bg-titlebar-button-close-disabled":
					props.type === "close",
			}}
			onClick={props.onClick}
		>
			<span
				class="h-4 w-4"
				classList={{ [props.icon]: true }}
			/>
		</button>
	);
};
