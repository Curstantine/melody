export function getErrorFromUnknown(e: unknown) {
	return e instanceof Error ? e as Error : null;
}
