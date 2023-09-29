import { Outlet } from "@solidjs/router";

export { default as SetupCreate } from "@/pages/setup/create";
export { default as SetupScan } from "@/pages/setup/scan";

export default function Setup() {
	return (
		<div class="h-full flex flex-col items-center justify-center px-6">
			<Outlet />
		</div>
	);
}
