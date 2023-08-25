import { createStore, type SetStoreFunction } from "solid-js/store";

import type { Directive, FormConfig, Validator } from "@/types/validators";

export function useForm() {
	const [errors, setErrors] = createStore<Record<string, string>>({});
	const fields: Record<string, FormConfig> = {};

	const validate: Directive<HTMLInputElement, Validator[]> = (ref, accessor) => {
		const validators = accessor() || [];
		const config: FormConfig = (fields[ref.name] = { element: ref, validators });

		ref.onblur = checkValid(config, setErrors);

		ref.oninput = () => {
			if (!errors[ref.name]) return;
			setErrors({ [ref.name]: undefined });
		};
	};

	const submit: Directive<HTMLFormElement, (form: HTMLFormElement) => void> = (ref, accessor) => {
		const callback = accessor() || (() => {});

		ref.setAttribute("novalidate", "");
		ref.onsubmit = (e) => {
			e.preventDefault();
			let errored = false;

			for (const key in fields) {
				const field = fields[key];
				checkValid(field, setErrors)();

				if (!errored && field.element.validationMessage) {
					field.element.focus();
					errored = true;
				}
			}

			if (!errored) callback.call(null, ref);
		};
	};

	const setError = (name: string, message: string | undefined) => {
		setErrors({ [name]: message });
	};

	return { validate, submit, errors, setError };
}

function checkValid({ element, validators }: FormConfig, setErrors: SetStoreFunction<Record<string, string>>) {
	return () => {
		element.setCustomValidity("");
		element.checkValidity();
		let message = element.validationMessage;

		if (!message) {
			for (const validator of validators) {
				const text = validator(element.value);
				if (text !== null) {
					element.setCustomValidity(text);
					break;
				}
			}
			message = element.validationMessage;
		}

		if (message) {
			setErrors({ [element.name]: message });
		}
	};
}

declare module "solid-js" {
	// eslint-disable-next-line @typescript-eslint/no-namespace
	namespace JSX {
		interface Directives {
			validate: import("@/types/validators").Validator[];
			submit: (form: HTMLFormElement) => void;
		}
	}
}
