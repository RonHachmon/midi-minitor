import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./index.css";

/**
 * Mounts the monitor window.
 *
 * The root element is created in `index.html`, so its absence means the document
 * itself is wrong — worth failing loudly rather than rendering into nothing.
 */
const container = document.getElementById("root");
if (container === null) {
  throw new Error("index.html is missing its #root element");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
