import { type Component, For, type JSX, Match, Show, Switch } from "solid-js";

import type { AppError } from "@/types/errors";

import styles from "./styles/card.module.css";

type Props = {
	data: AppError;
	onDismiss?: JSX.EventHandler<HTMLButtonElement, MouseEvent>;
};

export default function AppErrorDisplayCard(props: Props) {
	return (
		<div class={styles.card}>
			<span class="text-sm leading-tight font-orbiter-deck-medium text-modal-error-text-primary">Error</span>
			<span class="text-2xl font-orbiter-display-medium text-text-1">
				{props.data.error.message}
			</span>
			<Show when={props.data.error.context}>
				{(context) => <ContextContainer context={context()} />}
			</Show>
			<Show when={props.data.dismissible}>
				<div class="flex-1" />
				<div class="mt-6 flex items-end justify-end">
					<button class="button-layout button-template-error" onClick={(e) => props.onDismiss?.call(null, e)}>
						Dismiss
					</button>
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
