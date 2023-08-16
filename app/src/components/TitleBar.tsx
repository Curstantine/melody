import { app, events, window } from "@neutralinojs/lib";
import { createSignal, onMount } from "solid-js";
import type { JSX } from "solid-js/jsx-runtime";
import { Portal } from "solid-js/web";

export default function TitleBar() {
	const [isMaximized, setMaximizeStatus] = createSignal(false);

	const titlebarRef = document.getElementById("titlebar")!;
	let draggableRef: HTMLDivElement | undefined;

	onMount(() => {
		window.isMaximized()
			.then((status) => setMaximizeStatus(status))
			.catch((e) => console.error(e));

		window.setDraggableRegion(draggableRef!);

		draggableRef!.addEventListener("mousemove", async (e) => {
			if (e.buttons === 1 && await window.isMaximized()) {
				await contextuallyMaximize();
			}
		});

		draggableRef!.addEventListener("dblclick", async (e) => {
			if (e.buttons !== 1) return;
			contextuallyMaximize();
		});

		events.on("windowClose", close);
	});

	const contextuallyMaximize = async () => {
		if (isMaximized()) {
			await window.unmaximize();
			return setMaximizeStatus(false);
		}

		await window.maximize();
		setMaximizeStatus(true);
	};

	const minimize = async () => {
		try {
			await window.minimize();
		} catch (e) {
			console.error(e);
		}
	};

	const close = () => {
		app.exit(0);
	};

	return (
		<Portal mount={titlebarRef}>
			<div class="flex items-center justify-between h-9 inset-x-0 border-b-1 border-b-solid border-b-border ">
				<div ref={draggableRef!} class="flex-1 ">
					<span class="pl-4 select-none text-sm font-medium">Melody</span>
				</div>
				<div class="inline-flex">
					<TitleBarButton icon="i-material-minimize" type="common" onClick={minimize} />
					<TitleBarButton icon="i-material-maximize" type="common" onClick={contextuallyMaximize} />
					<TitleBarButton icon="i-material-close" type="close" onClick={close} />
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
const TitleBarButton = ({ icon, type = "common", onClick }: TitleBarButtonProps) => {
	return (
		<button
			class="inline-flex items-center justify-center w-12 h-full duration-200 transition-colors"
			classList={{
				"hover:bg-titlebar-button-common-hover active:bg-titlebar-button-common-active disabled:bg-titlebar-button-common-disabled":
					type === "common",
				"hover:bg-titlebar-button-close-hover active:bg-titlebar-button-close-active disabled:bg-titlebar-button-close-disabled":
					type === "close",
			}}
			onClick={onClick}
		>
			<span
				class="h-5 w-5"
				classList={{
					[icon]: true,
				}}
			/>
		</button>
	);
};
