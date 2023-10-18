import { type Component, For, Match, mergeProps, Show, Switch } from "solid-js";

import type { ActionableError } from "@/types/errors";

type Props = {
	data: ActionableError;
	class?: string;
	contextClass?: string;
	onDismiss?: () => void;
};

export default function ErrorCard(x: Props) {
	const props = mergeProps({ class: "", contextClass: "" }, x);
	const showActionRow = () => props.data.dismissible || props.data.actions?.length;

	return (
		<div
			class="max-h-xl max-w-xl min-h-36 w-full flex flex-col"
			classList={{ [props.class]: !!props.class }}
		>
			<span class="text-sm leading-tight font-orbiter-deck-medium text-modal-error-text-primary">Error</span>
			<span class="text-2xl font-orbiter-display-medium text-text-1">
				{props.data.error.message}
			</span>
			<Show when={props.data.error.context}>
				{(context) => <ContextContainer class={props.contextClass} context={context()} />}
			</Show>
			<Show when={showActionRow()}>
				<div class="flex-1" />
				<div class="mt-6 flex items-end justify-end">
					<ActionArea {...props.data} onDismiss={props.onDismiss} />
				</div>
			</Show>
		</div>
	);
}

const ContextContainer: Component<{ context: string | string[]; class: string }> = (props) => {
	const ctx = () => typeof props.context === "string" ? props.context.split("\n") : props.context;

	return (
		<span
			class="mt-2 rounded tracking-wide font-orbiter-text text-text-2"
			classList={{ [props.class]: !!props.class }}
		>
			<For each={ctx()}>
				{(context) => <p>{context}</p>}
			</For>
		</span>
	);
};

const ActionArea: Component<
	Pick<ActionableError, "dismissible" | "actions"> & Pick<Props, "onDismiss">
> = (props) => {
	return (
		<Switch>
			<Match when={props.dismissible}>
				<button class="button-layout button-template-error" onClick={() => props.onDismiss?.call(null)}>
					Dismiss
				</button>
			</Match>
			<Match when={props.actions?.length}>
				<Show when={props.dismissible}>
					<button class="button-layout button-template-text" onClick={() => props.onDismiss?.call(null)}>
						Dismiss
					</button>
				</Show>
				<For each={props.actions!}>
					{(action) => (
						<button
							class="button-layout button-template-error"
							classList={{
								"button-template-error": action.type === "error",
								"button-template-text": action.type === "text",
							}}
							onClick={() => props.onDismiss?.call(null)}
						>
							{action.label}
						</button>
					)}
				</For>
			</Match>
		</Switch>
	);
};
