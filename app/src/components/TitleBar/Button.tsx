import { mergeProps } from "solid-js";
import type { JSX } from "solid-js/jsx-runtime";

type Props = {
	icon: string;
	type?: "common" | "close";
	onClick?: JSX.EventHandler<HTMLButtonElement, MouseEvent>;
};

const styles = {
	common: "hover:bg-titlebar-button-common-background-hover active:bg-titlebar-button-common-background-active",
	close: "hover:bg-titlebar-button-close-background-hover active:bg-titlebar-button-close-background-active",
};

export default function TitleBarButton(_props: Props) {
	const props = mergeProps({ type: "common" }, _props);

	return (
		<button
			onClick={(e) => props.onClick?.call(null, e)}
			class="h-full w-12 inline-flex items-center justify-center bg-transparent transition-colors use-transition-standard"
			classList={{
				[styles.common]: props.type === "common",
				[styles.close]: props.type === "close",
			}}
		>
			<span class="h-[16px] w-[16px]" classList={{ [props.icon]: true }} />
		</button>
	);
}
