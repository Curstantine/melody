import type { LibraryModelOptions } from "@/types/models";

export default class LibraryModel {
	options: LibraryModelOptions;

	constructor(options: LibraryModelOptions) {
		this.options = options;
	}
}
