import { Route, Routes } from "@solidjs/router";
import { onMount } from "solid-js";

import AppModel, { AppModelContext } from "@/models/App";

import AppErrorDisplay from "@/components/AppErrorDisplay";
import TitleBar from "@/components/TitleBar";

import SharedRoute from "@/pages/(shared)";
import DevShowcase from "@/pages/dev";
import SetupRoute from "@/pages/setup";

export default function App() {
	const appModel = new AppModel();

	onMount(async () => await appModel.initialize());

	return (
		<AppModelContext.Provider value={appModel}>
			<TitleBar />
			<AppErrorDisplay />
			<Routes>
				<SharedRoute />
				<SetupRoute />
				<Route path="/dev/showcase" component={DevShowcase} />
			</Routes>
		</AppModelContext.Provider>
	);
}
