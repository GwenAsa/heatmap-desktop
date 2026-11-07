import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import Quick from "./quick/Quick";
import { MonthWidget, RingWidget, SheetWidget, StripWidget } from "./widgets/Widgets";
import "./styles.css";
import "./widgets/widgets.css";

// Same bundle, different windows: #/w/<kind> = desktop widget, #/quick = quick add, otherwise the main app.
const hash = window.location.hash;
const widgets: Record<string, () => JSX.Element> = { ring: RingWidget, month: MonthWidget, strip: StripWidget, sheet: SheetWidget };
let Root: () => JSX.Element = App;
if (hash.startsWith("#/w/") && widgets[hash.slice(4)]) Root = widgets[hash.slice(4)];
else if (hash === "#/quick") Root = Quick;
if (Root !== App) document.documentElement.classList.add("widget");

createRoot(document.getElementById("root")!).render(<StrictMode><Root /></StrictMode>);
