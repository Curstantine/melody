import { useAppErrorContext } from "@/contexts/errors";
import { Show } from "solid-js";

export default function AppErrorDisplay() {
	const [appError] = useAppErrorContext();

	return (
		<Show when={appError()}>
			<div class="h-full w-full flex">
				{appError()?.message}
			</div>
		</Show>
	);
}
