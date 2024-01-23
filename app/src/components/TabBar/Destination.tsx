import { Link } from "@solidjs/router";
import { createSignal, type JSX } from "solid-js";

type DragEventParam = Parameters<JSX.EventHandler<HTMLAnchorElement, DragEvent>>[0];
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

	const onDragStart: JSX.EventHandler<HTMLAnchorElement, DragEvent> = (e) => {
		setFloatStatus(true);
		props.onDragStart.call(null, props.id, e);
	};

	const onDragEnd: JSX.EventHandler<HTMLAnchorElement, DragEvent> = (e) => {
		setFloatStatus(false);
		props.onDragEnd.call(null, props.id, e);
	};

	return (
		<Link
			id={props.id}
			href={props.href}
			draggable={true}
			onDragStart={onDragStart}
			onDragEnd={onDragEnd}
			class="text-text-3 transition-transform,opacity use-transition-standard hover:text-text-1"
			activeClass="text-text-1!"
			classList={{ "opacity-25": isFloating() }}
		>
			{props.label}
		</Link>
	);
}
