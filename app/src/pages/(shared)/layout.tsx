import { Outlet } from "@solidjs/router";

import TabBar from "@/components/TabBar";

export default function SharedLayout() {
	return (
		<>
			<TabBar />
			<Outlet />
		</>
	);
}
