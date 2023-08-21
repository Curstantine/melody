import { open } from "@tauri-apps/api/dialog";
import { homeDir } from "@tauri-apps/api/path";
import { createEffect, createSignal, Match, Switch } from "solid-js";

import ClickableInput from "@/components/Input/ClickableInput";
import TextInput from "@/components/Input/TextInput";
import CreateLibraryView from "@/pages/setup/create";
import { validateLibraryName } from "@/utils/validators";

export default function Setup() {
	const [page, setPage] = createSignal(0);

	return (
		<div class="h-full flex flex-col items-center justify-center px-6">
			<Switch>
				<Match when={page() === 0}>
					<SetupLibraryView setPage={setPage} />
				</Match>
				<Match when={page() === 1}>
					<CreateLibraryView goBack={() => setPage(0)} />
				</Match>
			</Switch>
		</div>
	);
}

type SetupLibraryViewProps = {
	setPage: (page: number) => void;
};

function SetupLibraryView(props: SetupLibraryViewProps) {
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

		if (!result) return;
		setMode("create");
		setLibraryLocation(Array.isArray(result) ? result[0] : result);
	};

	const onConfirm = () => {
	};

	return (
		<form class="max-w-xl w-full flex flex-col b-1 b-border-main rounded p-4">
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
					value={libraryName() ?? ""}
					onInput={(e) => setLibraryName(e)}
					placeholder="The name of your library"
					icon="i-symbols-badge-outline-rounded"
					validation={validateLibraryName}
				/>
			</div>

			<div class="h-10 inline-flex justify-end">
				<button
					class="button-primary"
					classList={{ "opacity-50 select-none": !valid() }}
					onClick={onConfirm}
				>
					{mode() === "recover" ? "Recover" : "Create"}
				</button>
			</div>
		</form>
	);
}
