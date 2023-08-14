/* @refresh reload */

import "virtual:auth-info";
import "virtual:uno.css";

import { init } from "@neutralinojs/lib";
import { render } from "solid-js/web";
import "@unocss/reset/tailwind.css";

import "@/index.css";
import App from "@/App";

const root = document.getElementById("root");

render(() => <App />, root!);

init();
