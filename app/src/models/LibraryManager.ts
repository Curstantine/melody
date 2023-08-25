import { ulid } from "ulid";

import { LibraryManagerError } from "@/errors/models";
import LibraryModel from "@/models/Library";
import Result from "@/utils/result";
import { createSignalObject } from "@/utils/solid";

export default class LibraryManagerModel {
	private _libraries = createSignalObject<string[]>([]);
	private _current = createSignalObject<LibraryModel | null>(null);

	get libraries() {
		return this._libraries.get;
	}

	get current() {
		return this._current.get;
	}

	public async setCurrent(id: string): Promise<Result<void, LibraryManagerError>> {
		const library = this._libraries.get().find((library) => library === id);

		if (!library) {
			return Result.err(LibraryManagerError.unknownLibrary(id));
		}

		/// TODO: Read the library from the database
		this._current.set(new LibraryModel({ id, name: "Test", location: "Test" }));

		return Result.ok(void 0);
	}

	public createLibrary(name: string, location: string): LibraryModel {
		const id = ulid();
		/// Create the library in the database
		const library = new LibraryModel({ id, name, location });

		this._libraries.set((libraries) => [...libraries, id]);
		return library;
	}

	public async recoverLibrary(_location: string): Promise<Result<LibraryModel, LibraryManagerError>> {
		throw new Error("Not implemented");
	}
}
