import { invoke as tauriInvoke } from "@tauri-apps/api";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type { InvokeArgs } from "@tauri-apps/api/tauri";

import BackendError from "@/errors/backend";
import type { BackendCommands, BackendEvents } from "@/types/backend";
import Result from "@/utils/result";

export function invoke<T, A extends InvokeArgs | undefined = undefined>(
	method: BackendCommands,
	args?: A,
): Promise<Result<T, BackendError>> {
	return Result.runAsync(
		async () => {
			const response = await tauriInvoke<T>(method, args);
			return response;
		},
		(error) => BackendError.fromStupidError(error),
	);
}

export async function listen<T>(name: BackendEvents, handler: Parameters<typeof tauriListen<T>>[1]) {
	return await tauriListen<T>(name, handler);
}
