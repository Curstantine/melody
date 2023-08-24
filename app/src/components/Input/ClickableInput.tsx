import { JSX, Match, Show, Switch } from "solid-js";

type Props = {
	onClick: JSX.EventHandler<HTMLDivElement, MouseEvent>;
	value: string | null;
	icon?: string;
	placeholder?: string;
};

export default function ClickableInput(props: Props) {
	return (
		<div class="relative h-10 inline-flex items-center b-1 b-input-border-idle rounded px-2 text-text-2">
			<Show when={props.icon}>{(icon) => <div class="h-5 w-5" classList={{ [icon()]: true }} />}</Show>
			<div onClick={props.onClick} class="absolute inset-0 bg-transparent outline-none" />
			<Switch>
				<Match when={props.value}>
					{(location) => <span class="pl-2 tracking-tight font-orbiter-text">{location()}</span>}
				</Match>
				<Match when={props.placeholder}>
					{(placeholder) => (
						<span class="pl-2 tracking-tight font-orbiter-text text-text-3">{placeholder()}</span>
					)}
				</Match>
			</Switch>
		</div>
	);
}
