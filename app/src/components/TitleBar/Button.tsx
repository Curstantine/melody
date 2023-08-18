import { mergeProps } from "solid-js";
import type { JSX } from "solid-js/jsx-runtime";

import styles from "./styles/button.module.css";

type Props = {
	icon: string;
	type?: "common" | "close";
	onClick?: JSX.EventHandler<HTMLButtonElement, MouseEvent>;
};

export default function TitleBarButton(_props: Props) {
	const props = mergeProps({ type: "common" }, _props);

	return (
		<button
			class="h-full w-12 inline-flex items-center justify-center transition-colors duration-200"
			classList={{
				[styles.button_common]: props.type === "common",
				[styles.button_close]: props.type === "close",
			}}
			onClick={props.onClick}
		>
			<span
				class="h-4 w-4"
				classList={{ [props.icon]: true }}
			/>
		</button>
	);
}
