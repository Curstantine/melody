import { Route } from "@solidjs/router";

import Home from "@/pages/(shared)/home";
import Layout from "@/pages/(shared)/layout";

export const SHARED_PATHS = {
	HOME: "/(shared)/home",
};

export default function SharedRoute() {
	return (
		<Route path="/(shared)" component={Layout}>
			<Route path="/home" component={Home} />
		</Route>
	);
}
