import { Show } from "solid-js";

import { AppError } from "@/types/errors";

import styles from "./styles/card.module.css";

export default function AppErrorDisplayCard(props: AppError) {
	return (
		<div class={styles.card}>
			<span class="text-2xl font-orbiter-display-medium text-modal-error-text-primary leading-tight">
				{props.error.message}
			</span>
			<Show when={props.error.context}>
				{(contextString) => <span class="text-text-2 leading-tight">{contextString()}</span>}
			</Show>
			<Show when={props.dismissible}>
				<div class="flex-1 min-h-8" />
				<div class="flex items-end justify-end">
					<button class="button-error">Dismiss</button>
				</div>
			</Show>
		</div>
	);
}
