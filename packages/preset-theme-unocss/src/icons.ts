import { IconifyJSON } from "@iconify/types";
import { Awaitable } from "unocss";

export const iconCollection: Record<string, () => Awaitable<IconifyJSON>> = {
	symbols: importCollection("material-symbols"),
	mdi: importCollection("mdi"),
};

function importCollection(name: string): () => Awaitable<IconifyJSON> {
	return async () => {
		const { default: icons } = await import(`@iconify-json/${name}/icons.json`);
		return icons;
	};
}
