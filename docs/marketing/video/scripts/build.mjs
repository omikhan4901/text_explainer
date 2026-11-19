// Assembles the HyperFrames composition: src/index.html with the captured interface
// (composition/ui/*.html) inlined as <template>s and the app's built CSS linked.
// Usage: node docs/marketing/video/scripts/build.mjs
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const VIDEO = "docs/marketing/video";
const COMP = join(VIDEO, "composition");

const templates = readdirSync(join(COMP, "ui"))
  .filter((f) => f.endsWith(".html"))
  .map((f) => `<template id="ui-${f.slice(0, -5)}">${readFileSync(join(COMP, "ui", f), "utf8")}</template>`)
  .join("\n");

const css = readdirSync(join(COMP, "assets", "app"))
  .filter((f) => f.endsWith(".css"))
  // The shared stylesheet (tokens, Tailwind) first, then the page-specific ones.
  .sort((a, b) => Number(b.startsWith("ipc-")) - Number(a.startsWith("ipc-")))
  .map((f) => `<link rel="stylesheet" href="assets/app/${f}" />`)
  .join("\n    ");

// Scene times and spoken-word times (scripts/timing.py).
const timing = JSON.parse(readFileSync(join(VIDEO, "work", "timing.json"), "utf8"));
const src = readFileSync(join(VIDEO, "src", "index.html"), "utf8");
const out = src
  .replace("<!-- @app-css -->", css)
  .replace("<!-- @ui -->", templates)
  .replace("<!-- @timing -->", `<script>window.TIMING = ${JSON.stringify(timing)};</script>`)
  .replaceAll("{{total}}", String(timing.total))
  .replace(/\{\{(\w+)\.(start|dur)\}\}/g, (_, id, k) => {
    const scene = timing.scenes[id];
    if (!scene) throw new Error(`no scene "${id}" in timing.json`);
    return String(scene[k]);
  });
if (out.includes("{{")) throw new Error("unfilled placeholder in src/index.html");
writeFileSync(join(COMP, "index.html"), out);
console.log(`wrote ${join(COMP, "index.html")} (${(out.length / 1024).toFixed(0)} KB)`);
