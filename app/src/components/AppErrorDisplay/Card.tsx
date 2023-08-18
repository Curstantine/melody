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
			<span class="font-orbiter-deck-medium text-sm text-modal-error-text-primary leading-tight">Error</span>
			<span class="text-2xl font-orbiter-display-medium text-text-1">
				{props.data.error.message}
			</span>
			<Show when={props.data.error.context}>
				{(contextString) => <code class={styles.context}>{contextString()}</code>}
			</Show>
			<Show when={props.data.dismissible}>
				<div class="flex-1" />
				<div class="flex items-end justify-end mt-6">
					<button class="button-error" onClick={props.onDismiss}>Dismiss</button>
				</div>
			</Show>
		</div>
	);
}
