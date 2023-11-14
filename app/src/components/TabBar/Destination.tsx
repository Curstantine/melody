import { Link } from "@solidjs/router";
import { createSignal, type JSX } from "solid-js";

type DragEventParam = Parameters<JSX.EventHandler<HTMLDivElement, DragEvent>>[0];
export type DragEventFunc = (id: string, e: DragEventParam) => void;
export type Props = {
	id: string;
	href: string;
	label: string;
	onDragStart: DragEventFunc;
	onDragEnd: DragEventFunc;
};

export default function TabBarDestination(props: Props) {
	const [isFloating, setFloatStatus] = createSignal(false);

	const onDragStart: JSX.EventHandler<HTMLDivElement, DragEvent> = (e) => {
		setFloatStatus(true);
		props.onDragStart.call(null, props.id, e);
	};

	const onDragEnd: JSX.EventHandler<HTMLDivElement, DragEvent> = (e) => {
		setFloatStatus(false);
		props.onDragEnd.call(null, props.id, e);
	};

	return (
		<div
			id={props.id}
			draggable={true}
			onDragStart={onDragStart}
			onDragEnd={onDragEnd}
			class="transition-transform use-transition-standard"
			classList={{ "cursor-move": isFloating() }}
		>
			<Link
				href={props.href}
				class="text-text-2 use-transition-standard hover:text-text-1"
				activeClass="text-text-1"
			>
				{props.label}
			</Link>
		</div>
	);
}
