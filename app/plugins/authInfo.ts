import { readFileSync } from "node:fs";
import { resolve as pathResolve } from "node:path";
import type { Plugin } from "vite";

type AuthInfoData = {
	port: number;
	accessToken: string;
};

export default function authInfoPlugin(): Plugin {
	const virtualModuleId = "virtual:auth-info";
	const resolvedVirtualModuleId = "\0" + virtualModuleId;

	let mode: "serve" | "build";

	return {
		name: "authInfoInject",
		configResolved(x) {
			mode = x.command;
		},
		resolveId: (id) => {
			if (id === virtualModuleId) return resolvedVirtualModuleId;
		},
		load: (id) => {
			if (id !== resolvedVirtualModuleId) return;
			if (mode !== "serve") return `export default {}`;

			const authInfoJson = readFileSync(pathResolve("..", ".tmp", "auth_info.json"));
			const conf: AuthInfoData = JSON.parse(authInfoJson.toString());

			return `
				window.NL_PORT = ${conf.port};
				window.NL_TOKEN = "${conf.accessToken}";
			`;
		},
	};
}
