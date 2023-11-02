import { Outlet } from "@solidjs/router";

export default function SetupLayout() {
	return (
		<div class="h-full flex flex-col items-center justify-center px-6">
			<Outlet />
		</div>
	);
}
