import { Validator } from "@/types/validators";

export const validateLibraryName: Validator<string> = (value: string) => {
	if (value.length < 3) return "The library name must be at least 3 characters long.";
	if (value.length > 32) return "The library name must be at most 32 characters long.";
	if (!/^[a-zA-Z0-9-]+$/.test(value)) {
		return "The library name must only contain letters, numbers, and dashes.";
	}

	return null;
};
