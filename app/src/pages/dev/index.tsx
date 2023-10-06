import { createResource, For } from "solid-js";

export default function DevShowcase() {
	const [colorCSSNames] = createResource(async () => {
		const conf = await import("@/assets/themes/dark.json");
		const keys = Object.keys(conf.default.colors);
		return keys.map((key) => [key, key.replace(/\./g, "-")]);
	});

	return (
		<div class="h-full w-full overflow-y-auto px-6 pt-4">
			<h1 class="text-3xl text-text-1">Components</h1>
			<h2 class="mt-4 text-2xl text-text-1">Buttons</h2>
			<div class="grid grid-cols-8 mt-2 gap-4">
				<button class="button-layout button-template-primary">Primary</button>
				<button class="button-layout button-template-error">Error</button>
				<button class="button-layout button-template-text">Text</button>
			</div>

			<h1 class="mt-12 text-3xl text-text-1">Colors</h1>
			<table class="mt-4 w-full rounded bg-background-secondary">
				<thead>
					<tr class="h-10">
						<th class="px-4 text-left">Key</th>
						<th class="text-left">CSS Variable</th>
						<th class="w-32 pr-4 text-left">Color</th>
					</tr>
				</thead>
				<tbody>
					<For each={colorCSSNames()}>
						{(cssName) => (
							<tr class="h-10 text-text-2">
								<td class="px-4">{cssName[0]}</td>
								<td class="px-4">{`--${cssName[1]}`}</td>
								<td class="pr-4">
									<div
										class="h-8 w-32 border-(1 border-main solid) rounded"
										style={`background-color: var(--${cssName[1]})`}
									/>
								</td>
							</tr>
						)}
					</For>
				</tbody>
			</table>
		</div>
	);
}
