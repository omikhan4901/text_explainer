import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "@fontsource/atkinson-hyperlegible/400.css";
import "@fontsource/source-serif-4/400.css";
import "../styles/app.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
