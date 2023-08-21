import { Validator } from "@/types/validators";
import { createSignal, type JSX, Show } from "solid-js";

type Props = {
	value: string;
	onInput: (value: string) => void;
	icon?: string;
	prefix?: string;
	placeholder?: string;
	validation?: Validator<string>;
};

export default function TextInput(props: Props) {
	const [validationError, setValidationError] = createSignal<string | null>(null);

	const onInput: JSX.EventHandler<HTMLInputElement, Event> = (e) => {
		const val = e.currentTarget.value;
		const supposedErr = props.validation?.call(null, val) || null;

		if (supposedErr) setValidationError(supposedErr);
		else setValidationError(null);

		props.onInput(val);
	};

	return (
		<div class="relative h-10 inline-flex items-center b-1 b-input-border-idle rounded px-2 text-text-2">
			<Show when={props.icon}>{(icon) => <div class="h-5 w-5" classList={{ [icon()]: true }} />}</Show>
			<input
				type="text"
				class="absolute inset-0 bg-transparent pl-9 outline-none placeholder:(tracking-tight font-orbiter-text text-text-3) focus:placeholder:text-transparent"
				placeholder={props.placeholder}
				value={props.value}
				onInput={onInput}
			/>
			<Show when={validationError()}>
				{(error) => (
					<span class="absolute inset-x-0 text-sm font-orbiter-text text-text-error -bottom-6">
						{error()}
					</span>
				)}
			</Show>
		</div>
	);
}
