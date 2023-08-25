import { JSX, Match, Show, Switch } from "solid-js";

type Props = {
	onClick: JSX.EventHandler<HTMLDivElement, MouseEvent>;
	value: string | null;
	icon?: string;
	placeholder?: string;
	class?: string;
};

export default function ClickableInput(props: Props) {
	return (
		<div
			onClick={props.onClick}
			class="h-10 flex cursor-pointer items-center b-1 b-input-border-idle rounded px-2 tracking-tight font-orbiter-text text-text-2"
			classList={{ [props.class ?? ""]: !!props.class }}
		>
			<Show when={props.icon}>{(icon) => <div class="h-5 w-5 pr-2" classList={{ [icon()]: true }} />}</Show>
			<Switch>
				<Match when={props.value}>
					{(location) => <span class="select-none">{location()}</span>}
				</Match>
				<Match when={props.placeholder}>
					{(placeholder) => <span class="select-none text-text-3">{placeholder()}</span>}
				</Match>
			</Switch>
		</div>
	);
}
