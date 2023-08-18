import { expect, suite, test } from "vitest";
import { createNestedPropertyValue, isObject, mergeDeep } from "./general.node";

suite("isObject", () => {
	test("should return true if the item is an object", () => {
		const item = { a: 1 };
		const result = isObject(item);
		expect(result).toBeTruthy();
	});
	test("should return false if the item is null or undefined", () => {
		const item = null;
		const result = isObject(item);
		expect(result).toBeFalsy();

		const item2 = undefined;
		const result2 = isObject(item2);
		expect(result2).toBeFalsy();
	});
	test("should return false for arrays", () => {
		const item = [1, 2, 3];
		const result = isObject(item);
		expect(result).toBeFalsy();
	});
});

suite("mergeDeep", () => {
	test("should merge two objects deeply", () => {
		const object = { a: 1, b: { c: 2, d: { e: 3 } } };
		const object2 = { one: 1, b: { d: { two: 2 } } };

		const result = mergeDeep(object, object2);
		expect(result).toStrictEqual({
			a: 1,
			one: 1,
			b: { c: 2, d: { e: 3, two: 2 } },
		});
	});
	test("should overwrite overlapping properties on source", () => {
		const object = { a: 1, b: { c: 2, d: { e: 3 } } };
		const object2 = { one: 1, b: { d: { e: 2 } } };

		const result = mergeDeep(object, object2);
		expect(result).toStrictEqual({
			a: 1,
			one: 1,
			b: { c: 2, d: { e: 2 } },
		});
	});
});

suite("createNestedPropertyValue", () => {
	test("should create a nested object with the given properties and value", () => {
		const result = createNestedPropertyValue(["a", "b", "c"], 1);
		expect(result).toStrictEqual({ a: { b: { c: 1 } } });
	});
});
