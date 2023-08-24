import { Show } from "solid-js";

import type { Directive, Validator } from "@/types/validators";

type Props = {
	value: string;
	onInput: (value: string) => void;
	name?: string;
	icon?: string;
	prefix?: string;
	placeholder?: string;
	validate: Directive<HTMLInputElement, Validator[]>;
	validators?: Validator[];
};

export default function TextInput(props: Props) {
	// @ts-expect-error submit is being used as a directive
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const validate: Directive<HTMLInputElement, Validator[]> = (el, validators) => {
		return props.validate(el, validators);
	};

	return (
		<div class="relative h-10 inline-flex items-center b-1 b-input-border-idle rounded px-2 text-text-2">
			<Show when={props.icon}>{(icon) => <div class="h-5 w-5" classList={{ [icon()]: true }} />}</Show>
			<input
				type="text"
				name={props.name}
				class="absolute inset-0 bg-transparent pl-9 outline-none placeholder:(tracking-tight font-orbiter-text text-text-3) focus:placeholder:text-transparent"
				value={props.value}
				placeholder={props.placeholder}
				onInput={(e) => props.onInput(e.target.value)}
				use:validate={props.validators}
			/>
		</div>
	);
}
