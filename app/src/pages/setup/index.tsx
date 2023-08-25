import { SetupViewModelContext, useSetupViewModel } from "@/contexts/models";
import { validateLibraryName } from "@/utils/validators";

import InputError from "@/components/Input/InputError";
import TextInput from "@/components/Input/TextInput";

import SetupViewModel from "./model";

export default function Setup() {
	const viewModel = new SetupViewModel();

	return (
		<SetupViewModelContext.Provider value={viewModel}>
			<div class="h-full flex flex-col items-center justify-center px-6">
				<CreateView />
			</div>
		</SetupViewModelContext.Provider>
	);
}

function CreateView() {
	const {
		mode: [mode],
		name: [libraryName, setLibraryName],
		// @ts-expect-error submit is being used as a directive
		// eslint-disable-next-line @typescript-eslint/no-unused-vars
		form: { validate, submit, errors, setError },
		onConfirm,
	} = useSetupViewModel();

	return (
		<form class="max-w-xl w-full flex flex-col b-1 b-border-main rounded p-4" use:submit={onConfirm}>
			<span class="text-2xl leading-tight font-orbiter-display text-text-1">Setup your library</span>
			<span class="leading-tight font-orbiter-text text-text-2">
				Start by creating a library or recovering an existing one.
			</span>

			<div class="my-8 flex flex-col">
				<span class="pb-1 text-sm font-orbiter-deck">Name</span>
				<TextInput
					name="libraryName"
					value={libraryName() ?? ""}
					onInput={(e) => setLibraryName(e)}
					placeholder="The name of your library"
					icon="i-symbols-badge-outline-rounded"
					validate={validate}
					validators={[validateLibraryName]}
				/>
				<InputError message={errors["libraryName"]} />
			</div>

			<div class="h-10 inline-flex justify-end">
				<button type="submit" class="button-primary">
					{mode() === "recover" ? "Recover" : "Create"}
				</button>
			</div>
		</form>
	);
}
