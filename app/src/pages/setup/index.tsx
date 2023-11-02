import { Route } from "@solidjs/router";

import Create from "@/pages/setup/create";
import SetupLayout from "@/pages/setup/layout";
import Scan from "@/pages/setup/scan";

export const SETUP_PATHS = {
	CREATE: "/setup/create",
	SCAN: "/setup/scan",
};

export default function SetupRoute() {
	return (
		<Route path="/setup" component={SetupLayout}>
			<Route path="/create" component={Create} />
			<Route path="/scan" component={Scan} />
		</Route>
	);
}
