export default class Result<T, E> {
	error: E | null;
	value: T | null;

	constructor(value: T | null, error: E | null) {
		this.error = error;
		this.value = value;
	}

	public isOk(): this is Result<T, never> {
		return this.value !== null;
	}

	public isErr(): this is Result<never, E> {
		return this.error !== null;
	}

	public unwrap(): T {
		if (this.value === null) {
			throw new Error("called `Result.unwrap()` on an `Err` value");
		}

		return this.value;
	}

	public unwrapErr(): E {
		if (this.error === null) {
			throw new Error("called `Result.unwrapErr()` on an `Ok` value");
		}

		return this.error;
	}

	public static ok<T, E>(value: T): Result<T, E> {
		return new Result<T, E>(value, null);
	}

	public static err<T, E>(error: E): Result<T, E> {
		return new Result<T, E>(null, error);
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
			return Result.ok(await fn());
		} catch (error) {
			return Result.err(err === null ? (error as E) : err(error));
		}
	}
}
