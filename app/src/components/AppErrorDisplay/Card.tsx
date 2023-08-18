import { AppError } from "@/types/errors";
import { Show } from "solid-js";

export default function AppErrorDisplayCard(props: AppError) {
	return (
		<div class="max-h-xl max-w-xl min-h-36 w-full flex flex-col rounded bg-background-secondary p-4">
			<span class="text-2xl font-orbiter-display-medium">
				{props.error.message}
			</span>
			<Show when={props.error.context}>
				{(contextString) => <span>{contextString()}</span>}
			</Show>
			<Show when={props.dismissible}>
				<div class="flex-1" />
				<div class="flex items-end justify-end">
					<button>Dismiss</button>
				</div>
			</Show>
		</div>
	);
}
