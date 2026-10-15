import { resolve } from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Three pages: the main window (settings, models, first run), the reading card popup and
// the Look up box.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022",
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, "index.html"),
        popup: resolve(import.meta.dirname, "popup.html"),
        lookup: resolve(import.meta.dirname, "lookup.html"),
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["src/test/setup.ts"],
    css: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
