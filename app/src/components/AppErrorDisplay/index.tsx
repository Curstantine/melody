import { Show } from "solid-js";
import { Portal } from "solid-js/web";

import ErrorCard from "@/components/Card/Error";

import { useAppModel } from "@/AppModel";

export default function AppErrorDisplay() {
	const { appError: [appError, setAppError] } = useAppModel();

	const onDismiss = () => {
		if (appError()?.dismissible) return setAppError(null);
	};

	return (
		<Portal mount={document.getElementById("modal-root")!}>
			<Show when={appError()}>
				{(appError) => (
					<div class="absolute inset-0 z-50 flex items-center justify-center bg-black/20 px-4">
						<ErrorCard
							data={appError()}
							onDismiss={onDismiss}
							class="border-b-4 border-modal-error-border border-solid bg-background-secondary p-6"
							contextClass="bg-background-main p-2"
						/>
					</div>
				)}
			</Show>
		</Portal>
	);
}
