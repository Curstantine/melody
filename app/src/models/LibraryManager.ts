import { Accessor, createSignal } from "solid-js";
import { ulid } from "ulid";

import LibraryManagerError from "@/errors/models/LibraryManager";
import Result from "@/utils/result";

import LibraryModel from "@/models/Library";

export default class LibraryManagerModel {
	private _libraries = createSignal<string[]>([]);
	private _current = createSignal<LibraryModel | null>(null);

	get libraries(): Accessor<string[]> {
		const [libraries] = this._libraries;
		return libraries;
	}

	get current(): Accessor<LibraryModel | null> {
		const [current] = this._current;
		return current;
	}

	public async setCurrent(id: string): Promise<Result<void, LibraryManagerError>> {
		const [, setCurrentSignal] = this._current;

		const library = this.libraries().find((library) => library === id);
		if (!library) {
			return Result.err(LibraryManagerError.unknownLibrary(id));
		}

		/// TODO: Read the library from the database
		const model = new LibraryModel({ id, name: "Test", location: "Test" });
		setCurrentSignal(model);

		return Result.ok(void 0);
	}

	public createLibrary(name: string, location: string): LibraryModel {
		const [, setLibrariesSignal] = this._libraries;

		const id = ulid();
		const library = new LibraryModel({ id, name, location });
		setLibrariesSignal((libraries) => [...libraries, id]);

		return library;
	}

	public async recoverLibrary(_location: string): Promise<Result<LibraryModel, LibraryManagerError>> {
		throw new Error("Not implemented");
	}
}
