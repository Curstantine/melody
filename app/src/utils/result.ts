const empty = Symbol("empty");

export default class Result<T, E> {
	error: E | typeof empty;
	value: T | typeof empty;

	constructor(value: T | typeof empty, error: E | typeof empty = empty) {
		this.error = error;
		this.value = value;
	}

	public isOk(): this is Result<T, never> {
		return this.value !== empty;
	}

	public isErr(): this is Result<never, E> {
		return this.error !== empty;
	}

	public unwrap(): T {
		if (this.value === empty) throw this.error;
		return this.value;
	}

	public unwrapErr(): E {
		if (this.error === empty) {
			throw new Error("called `Result.unwrapErr()` on an `Ok` value");
		}

		return this.error;
	}

	public static ok<T, E>(value: T): Result<T, E> {
		return new Result<T, E>(value, empty);
	}

	public static err<T, E>(error: E): Result<T, E> {
		return new Result<T, E>(empty, error);
	}

	public static run<T, E>(fn: () => T, err: (e: unknown) => E): Result<T, E> {
		try {
			return Result.ok(fn());
		} catch (error) {
			return Result.err(err(error));
		}
	}

	public static async runAsync<T, E>(
		fn: () => Promise<T>,
		err: null | ((e: unknown) => E) = null,
	): Promise<Result<T, E>> {
		try {
			return Result.ok(await fn.call(null));
		} catch (error) {
			return Result.err(err === null ? (error as E) : err.call(null, error));
		}
	}
}
