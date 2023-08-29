import { JSX, Match, Show, Switch } from "solid-js";

type Props = {
	value: string | null;
	onClick: JSX.EventHandler<HTMLDivElement, MouseEvent>;
	onLeadingButtonClick: JSX.EventHandler<HTMLButtonElement, MouseEvent>;
	leadingIcon: string;
	leadingButtonType: "error" | "primary";
	showLeadingButton?: boolean;
	placeholder?: string;
};

export default function LeadingClickableInput(props: Props) {
	return (
		<div class="h-10 flex items-center gap-2">
			<Show when={props.showLeadingButton}>
				<button
					onClick={props.onLeadingButtonClick}
					class="h-8 min-w-8 icon-button-layout bg-transparent"
					classList={{
						"button-template-error": props.leadingButtonType === "error",
						"button-template-primary": props.leadingButtonType === "primary",
					}}
				>
					<div class="h-5 w-5 pr-2" classList={{ [props.leadingIcon]: true }} />
				</button>
			</Show>
			<div
				onClick={props.onClick}
				class="h-10 inline-flex flex-1 cursor-pointer items-center b-(1 input-border-idle) rounded px-2 tracking-tight font-orbiter-text text-text-2"
			>
				<Switch>
					<Match when={props.value}>
						{(location) => <span class="select-none">{location()}</span>}
					</Match>
					<Match when={props.placeholder}>
						{(placeholder) => <span class="select-none text-text-3">{placeholder()}</span>}
					</Match>
				</Switch>
			</div>
		</div>
	);
}
