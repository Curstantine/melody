import { Outlet } from "@solidjs/router";

import ReleaseSideView, { ReleaseSideViewProvider } from "@/components/ReleaseSideView";
import TabBar from "@/components/TabBar";

export default function SharedLayout() {
	return (
		<>
			<TabBar />
			<div class="max-h-[calc(100%-2rem)] max-w-full flex space-x-1">
				<ReleaseSideViewProvider>
					<Outlet />
					<ReleaseSideView />
				</ReleaseSideViewProvider>
			</div>
		</>
	);
}
