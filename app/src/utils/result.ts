export default class Result<T, E> {
	error: E | null;
	value: T | null;

	constructor(value: T | null, error: E | null) {
		this.error = error;
		this.value = value;
	}

	get isOk(): boolean {
		return this.error === null;
	}

	get isErr(): boolean {
		return this.error !== null;
	}

	unwrapErr(): E {
		if (this.error === null) {
			throw new Error("called `Result.unwrapErr()` on an `Ok` value");
		}

		return this.error;
	}

	static ok<T, E>(value: T): Result<T, E> {
		return new Result<T, E>(value, null);
	}

	static err<T, E>(error: E): Result<T, E> {
		return new Result<T, E>(null, error);
	}

	static run<T, E>(fn: () => T, err: (e: unknown) => E): Result<T, E> {
		try {
			return Result.ok(fn());
		} catch (error) {
			return Result.err(err(error));
		}
	}

	static async runAsync<T, E>(fn: () => Promise<T>, err: (e: unknown) => E): Promise<Result<T, E>> {
		try {
			return Result.ok(await fn());
		} catch (error) {
			return Result.err(err(error));
		}
	}
}
