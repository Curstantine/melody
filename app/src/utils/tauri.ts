import { invoke as tauriInvoke } from "@tauri-apps/api";
import type { InvokeArgs } from "@tauri-apps/api/tauri";

import BackendError from "@/errors/backend";
import Result from "@/utils/result";

export function invoke<T>(method: string, args?: InvokeArgs): Promise<Result<T, BackendError>> {
	return Result.runAsync(
		async () => {
			const response = await tauriInvoke<T>(method, args);
			return response;
		},
		(error) => BackendError.fromStupidError(error),
	);
}
