import { type Component, For, Match, Show, Switch } from "solid-js";

import type { ActionableError } from "@/types/errors";

import styles from "./styles/card.module.css";

type Props = {
	data: ActionableError;
	onDismiss?: () => void;
};

export default function AppErrorDisplayCard(props: Props) {
	const showActionRow = () => props.data.dismissible || props.data.actions?.length;

	return (
		<div class={styles.card}>
			<span class="text-sm leading-tight font-orbiter-deck-medium text-modal-error-text-primary">Error</span>
			<span class="text-2xl font-orbiter-display-medium text-text-1">
				{props.data.error.message}
			</span>
			<Show when={props.data.error.context}>
				{(context) => <ContextContainer context={context()} />}
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

const ContextContainer: Component<{ context: string | string[] }> = (props) => {
	return (
		<span class={styles.context}>
			<Switch fallback={<>{props.context}</>}>
				<Match when={Array.isArray(props.context)}>
					<For each={props.context as string[]}>
						{(context) => <p>{context}</p>}
					</For>
				</Match>
				<Match when={typeof props.context === "string"}>
					{props.context}
				</Match>
			</Switch>
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
