import { Route, Routes } from "@solidjs/router";
import { onMount } from "solid-js";

import AppModel, { AppModelContext } from "@/models/App";

import AppErrorDisplay from "@/components/AppErrorDisplay";
import TitleBar from "@/components/TitleBar";

import UIRoot from "@/pages";
import Setup from "@/pages/setup";

export default function App() {
	const appModel = new AppModel();

	onMount(async () => await appModel.initialize());

	return (
		<AppModelContext.Provider value={appModel}>
			<TitleBar />
			<AppErrorDisplay />
			<Routes>
				<Route path="/" component={UIRoot} />
				<Route path="/setup" component={Setup} />
			</Routes>
		</AppModelContext.Provider>
	);
}
