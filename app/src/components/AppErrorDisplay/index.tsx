import AppErrorDisplayCard from "@/components/AppErrorDisplay/Card";
import { useAppErrorContext } from "@/contexts/errors";
import { Show } from "solid-js";
import { Portal } from "solid-js/web";

export default function AppErrorDisplay() {
	const [appErrorData, setAppError] = useAppErrorContext();

	const onDismiss = () => {
		if (appErrorData()?.dismissible) return setAppError(null);
	};

	return (
		<Portal mount={document.getElementById("root")!}>
			<Show when={appErrorData()}>
				{(appError) => (
					<div class="absolute inset-0 flex items-center justify-center bg-black/20 px-4">
						<AppErrorDisplayCard data={appError()} onDismiss={onDismiss} />
					</div>
				)}
			</Show>
		</Portal>
	);
}
