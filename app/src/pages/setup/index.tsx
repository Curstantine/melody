import { open } from "@tauri-apps/api/dialog";
import { readDir } from "@tauri-apps/api/fs";
import { homeDir } from "@tauri-apps/api/path";
import { createEffect, createSignal } from "solid-js";

import { useForm } from "@/hooks/validator";
import { validateLibraryName } from "@/utils/validators";

import ClickableInput from "@/components/Input/ClickableInput";
import InputError from "@/components/Input/InputError";
import TextInput from "@/components/Input/TextInput";
import Result from "@/utils/result";

export default function Setup() {
	// const [page, setPage] = createSignal(0);

	return (
		<div class="h-full flex flex-col items-center justify-center px-6">
			<SetupLibraryView />
		</div>
	);
}

function SetupLibraryView() {
	// @ts-expect-error submit is being used as a directive
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { validate, submit, errors } = useForm();

	const [mode, setMode] = createSignal<"create" | "recover">();
	const [valid, setValidation] = createSignal(false);

	const [libraryLocation, setLibraryLocation] = createSignal<string | null>(null);
	const [libraryName, setLibraryName] = createSignal<string | null>(null);

	createEffect(() => {
		setValidation(libraryLocation() !== null && libraryName() !== null);
	});

	/*
	 * NOTE(Curstantine):
	 * The plan is to add the recovery based on the selected folder.
	 *
	 * So in cases where the selected folder matches the library structure, it would try to recover it.
	 * And in cases where it doesn't, it would create a new library.
	 */
	const openLibraryLocation = async () => {
		const result = await open({
			title: "Select library location",
			directory: true,
			multiple: false,
			defaultPath: libraryLocation() ?? await homeDir(),
		});

		if (!result || typeof result !== "string") return;
		setMode("create");

		if (result.match(/(M|m)elody(\/|$)/)) {
			setLibraryLocation(result);
		} else if (result.endsWith("/")) {
			setLibraryLocation(result + "Melody/");
		} else {
			setLibraryLocation(result + "/Melody/");
		}

		const files = await Result.runAsync(() => readDir(result), (e) => console.error(e));
		console.log(files);
	};

	const onConfirm = () => {
		console.log(errors);
	};

	return (
		<form class="max-w-xl w-full flex flex-col b-1 b-border-main rounded p-4" use:submit={onConfirm}>
			<span class="text-2xl leading-tight font-orbiter-display text-text-1">Setup your library</span>
			<span class="leading-tight font-orbiter-text text-text-2">
				Start by creating a library or recovering an existing one.
			</span>

			<div class="my-8 flex flex-col">
				<span class="pb-1 text-sm font-orbiter-deck">Location</span>
				<ClickableInput
					value={libraryLocation()}
					onClick={openLibraryLocation}
					placeholder="The magical place where your library will be stored"
					icon="i-symbols-folder-outline-rounded"
				/>

				<span class="mt-4 pb-1 text-sm font-orbiter-deck">Name</span>
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
				<button type="submit" class="button-primary" disabled={!valid()}>
					{mode() === "recover" ? "Recover" : "Create"}
				</button>
			</div>
		</form>
	);
}
