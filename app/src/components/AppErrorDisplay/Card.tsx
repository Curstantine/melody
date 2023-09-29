import { type Component, type JSX, Match, Show, Switch } from "solid-js";

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
					{Object.entries(props.context).map(([key, value]) => (
						<div class="flex flex-row">
							<span class="text-text-2">{key}:</span>
							<span class="ml-2 text-text-2">{value}</span>
						</div>
					))}
				</Match>
				<Match when={typeof props.context === "string"}>
					<span class="text-text-2">{props.context}</span>
				</Match>
			</Switch>
		</span>
	);
};
