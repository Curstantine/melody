import { Show } from "solid-js";

type Props = {
	message?: string | null;
};

export default function InputError(props: Props) {
	return (
		<Show when={props.message}>
			{(error) => (
				<span class="mt-1 inline-flex items-center text-start text-sm font-orbiter-deck text-text-error">
					{error()}
				</span>
			)}
		</Show>
	);
}
