import { Route } from "@solidjs/router";

import Layout from "@/pages/(shared)/layout";
import Music from "@/pages/(shared)/music";

export const SHARED_PATHS = {
	MUSIC: "/(shared)/music",
};

export default function SharedRoute() {
	return (
		<Route path="/(shared)" component={Layout}>
			<Route path="/music" component={Music} />
		</Route>
	);
}
