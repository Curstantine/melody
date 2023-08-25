import { createSignal } from "solid-js";

import { useForm } from "@/hooks/validator";

export default class SetupViewModel {
	mode = createSignal<"create" | "recover">();
	name = createSignal<string | null>(null);
	valid = createSignal<boolean>(false);
	form = useForm();

	public async onConfirm() {}
}
