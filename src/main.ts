import "@fontsource-variable/schibsted-grotesk";
import "open-props/style";
import "open-props/normalize";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app")! });
