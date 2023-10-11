import { LocalError } from "@/types/errors";

export default class DataError implements LocalError {
	type: string = "DataError";
	code: number;
	message: string;
	context?: string | string[];

	constructor(code: number, message: string, context?: string | string[]) {
		this.code = code;
		this.message = message;
		this.context = context;
	}

	static missingLocationState(route: string, props: { [key: string]: unknown }): DataError {
		const unassignedProps = Object.entries(props)
			.filter(([, value]) => value === undefined)
			.map(([key]) => key);

		return new DataError(
			DataError.codes.MISSING_LOCATION_STATE,
			"Route change was missing state data.",
			[
				`Expected to find state data for route: ${route}.`,
				`Values for the following props were missing: ${unassignedProps.join(", ")}`,
			],
		);
	}

	static placeholder(): DataError {
		return new DataError(
			0,
			"A long enough placeholder",
			[
				"1st line of the multi-lined placeholder context message",
				"and then the second line of this multi-lined placeholder context message",
			],
		);
	}

	static codes = {
		MISSING_LOCATION_STATE: 1,
	};
}
