export type Validator<T extends string | number> = (value: T) => string | null;
