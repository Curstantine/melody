import { filesystem } from "@neutralinojs/lib";
import "./App.css";

function App() {
	filesystem.readDirectory("./")
		.then(console.log)
		.catch(console.error);

	return (
		<>
			<h1>Lol</h1>
		</>
	);
}

export default App;
