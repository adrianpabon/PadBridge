import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// No browser context menu or reload in a desktop app.
document.addEventListener("contextmenu", (e) => e.preventDefault());

export default mount(App, { target: document.getElementById("app")! });
