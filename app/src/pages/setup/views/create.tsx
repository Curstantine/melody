import { createEffect, For } from "solid-js";

import { validateLibraryName } from "@/utils/validators";

import InputError from "@/components/Input/InputError";
import LeadingClickableInput from "@/components/Input/LeadingClickableInput";
import TextInput from "@/components/Input/TextInput";

import SetupCreateViewModel from "./create.model";

export default function SetupCreateView() {
	const {
		mode: [mode],
		name: [libraryName, setLibraryName],
		scanLocations: [scanLocations],
		continuable: [continuable, setContinuability],
		// @ts-expect-error submit is being used as a directive
		// eslint-disable-next-line @typescript-eslint/no-unused-vars
		form: { validate, submit, errors, setError },
		onConfirm,
		onScanLocationFieldPress,
		addScanLocation,
		removeScanLocation,
	} = new SetupCreateViewModel();

	createEffect(() => {
		const noErrors = Object.values(errors).filter((x) => !!x).length === 0;
		const noEmptyLocations = scanLocations.filter((x) => !x.location).length === 0;
		const result = noErrors && noEmptyLocations;

		setContinuability(result);
	});

	return (
		<form
			use:submit={onConfirm}
			class="max-h-md max-w-lg w-full flex flex-col overflow-auto b-(1 border-secondary) rounded bg-background-secondary p-4"
		>
			<span class="text-2xl leading-tight font-orbiter-display text-text-1">Setting up</span>
			<span class="leading-tight font-orbiter-text text-text-2">
				Continue by creating a new library or recovering an existing one.
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

				<span class="mt-4 pb-1 text-sm font-orbiter-deck">Scan Locations</span>
				<div class="flex flex-col gap-2">
					<For each={scanLocations}>
						{(obj) => (
							<div class="flex flex-col">
								<LeadingClickableInput
									value={obj.location}
									leadingButtonType="error"
									placeholder="The path to a folder to scan"
									leadingIcon="i-symbols-delete-outline-rounded"
									showLeadingButton={scanLocations.length > 1}
									onClick={(e) => onScanLocationFieldPress(obj.id, e)}
									onLeadingButtonClick={(e) => removeScanLocation(obj.id, e)}
								/>
								<InputError message={errors[obj.id]} />
							</div>
						)}
					</For>
					<button
						onClick={addScanLocation}
						class="h-9 inline-flex items-center justify-center b-(2 border-secondary dotted) rounded bg-transparent text-text-2"
					>
						<div class="i-symbols-add-rounded mr-2 h-5 w-5" />
						<span class="text-sm font-orbiter-deck">Add scan location</span>
					</button>
				</div>
			</div>

			<div class="h-10 inline-flex justify-end">
				<button type="submit" class="button-layout button-template-primary" disabled={!continuable()}>
					{mode() === "recover" ? "Recover" : "Create"}
				</button>
			</div>
		</form>
	);
}
