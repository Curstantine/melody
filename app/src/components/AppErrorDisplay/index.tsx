import { Show } from "solid-js";
import { Portal } from "solid-js/web";

import AppErrorDisplayCard from "@/components/AppErrorDisplay/Card";
import { useAppModel } from "@/contexts/models";

export default function AppErrorDisplay() {
	const { appError } = useAppModel();

	const onDismiss = () => {
		if (appError.get()?.dismissible) return appError.set(null);
	};

	return (
		<Portal mount={document.getElementById("modal-root")!}>
			<Show when={appError.get()}>
				{(appError) => (
					<div class="absolute inset-0 z-50 flex items-center justify-center bg-black/20 px-4">
						<AppErrorDisplayCard data={appError()} onDismiss={onDismiss} />
					</div>
				)}
			</Show>
		</Portal>
	);
}
