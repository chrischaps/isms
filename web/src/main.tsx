import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
// The two faces (docs/style.md §4.3), self-hosted: the game makes no third-party request.
import "@fontsource/outfit/500.css";
import "@fontsource/outfit/700.css";
import "@fontsource/atkinson-hyperlegible/400.css";
import "@fontsource/atkinson-hyperlegible/400-italic.css";
import "@fontsource/atkinson-hyperlegible/700.css";
import "./styles/tokens.css";
import "./styles/components.css";
import { App } from "./router";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
