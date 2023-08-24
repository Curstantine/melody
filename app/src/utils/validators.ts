import type { Validator } from "@/types/validators";

export const validateLibraryName: Validator = (value) => {
	if (typeof value !== "string") return "The library name must be a string.";
	if (value.length < 3) return "The library name must be at least 3 characters long.";
	if (value.length > 32) return "The library name must be at most 32 characters long.";

	return null;
};
