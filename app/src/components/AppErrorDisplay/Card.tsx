import { JSX, Show } from "solid-js";

import { AppError } from "@/types/errors";

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
				{(contextString) => <code class={styles.context}>{contextString()}</code>}
			</Show>
			<Show when={props.data.dismissible}>
				<div class="flex-1" />
				<div class="mt-6 flex items-end justify-end">
					<button class="button-layout button-template-error" onClick={props.onDismiss}>Dismiss</button>
				</div>
			</Show>
		</div>
	);
}
