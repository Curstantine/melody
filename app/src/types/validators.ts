export type Validator = (value: string | number) => string | null;

export type Directive<E = HTMLElement, T = unknown> = (ref: E, accessor: () => T) => void;
export type FormConfig = { element: HTMLInputElement; validators: Validator[] };
