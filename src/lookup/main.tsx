import "@fontsource-variable/geist";
import "../styles/app.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Lookup } from "./Lookup";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Lookup />
  </StrictMode>,
);
