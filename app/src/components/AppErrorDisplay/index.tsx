import { Show } from "solid-js";
import { Portal } from "solid-js/web";

import AppErrorDisplayCard from "@/components/AppErrorDisplay/Card";
import { useAppModelContext } from "@/contexts/models";

export default function AppErrorDisplay() {
	const { appError } = useAppModelContext();

	const onDismiss = () => {
		if (appError.get()?.dismissible) return appError.set(null);
	};

	return (
		<Portal mount={document.getElementById("root")!}>
			<Show when={appError.get()}>
				{(appError) => (
					<div class="absolute inset-0 flex items-center justify-center bg-black/20 px-4">
						<AppErrorDisplayCard data={appError()} onDismiss={onDismiss} />
					</div>
				)}
			</Show>
		</Portal>
	);
}
