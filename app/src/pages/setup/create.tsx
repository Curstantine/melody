type Props = {
	goBack: () => void;
};

export default function CreateLibraryView(props: Props) {
	return (
		<div class="h-full flex flex-col items-center justify-center">
			<span class="text-2xl font-orbiter-display text-text-1">Starting from the scratch</span>
			<span class="font-orbiter-text text-text-2">
				Lets create a ne
			</span>

			<div class="mt-8 flex flex-row">
				<button class="button-layout" onClick={props.goBack}>Back</button>
			</div>
		</div>
	);
}
