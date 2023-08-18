export default function Home() {
	return (
		<div class="flex flex-col items-center justify-center h-full">
			<span class="font-orbiter-display text-2xl text-text-1">Setup your library</span>
			<span class="font-orbiter-text text-text-2">Start a new one or recover from an old backup!</span>

			<div class="flex flex-row mt-8">
				<button class="button-layout">Create</button>
				<div class="mx-2 w-[1px] h-8 bg-border" />
				<button class="button-layout">Recover</button>
			</div>
		</div>
	);
}
