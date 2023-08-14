import { filesystem } from "@neutralinojs/lib";
import "./App.css";
import { createSignal, For } from "solid-js";

function App() {
	const [files, setFiles] = createSignal<string[]>([]);

	filesystem.readDirectory("./")
		.then((x) => setFiles(x.map((y) => y.entry)))
		.catch(console.error);

	return (
		<>
			<h1>Lol</h1>
			<ul>
				<For each={files()}>
					{(item) => <li>{item}</li>}
				</For>
			</ul>
		</>
	);
}

export default App;
