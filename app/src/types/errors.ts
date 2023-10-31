export interface LocalError {
	code: number;
	message: string;
	context?: string | string[];
}

export interface ErrorAction {
	type: "error" | "primary" | "text";
	label: string;
	onClick: () => void;
}

export interface ActionableError {
	error: LocalError;
	dismissible?: boolean;
	actions?: ErrorAction[];
}
