export interface LocalError {
	type: string;
	code: number;
	message: string;
	context?: string | string[];
}

export interface AppError {
	error: LocalError;
	dismissible: boolean;
}
