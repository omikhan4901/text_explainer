// Loads the built composition headlessly and saves the sound-effect cues it logs while it
// builds its timeline (window.__sfx), so the mix always matches the picture.
// Usage (from the repo root): node docs/marketing/video/scripts/cues.mjs
import { chromium } from "@playwright/test";
import { existsSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const ROOT = resolve("docs/marketing/video/composition");
const browser = await chromium.launch({ channel: "chrome" });
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
// Serve the composition folder from disk under a made-up origin.
await page.route("http://composition.local/**", (route) => {
  const rel = decodeURIComponent(new URL(route.request().url()).pathname).replace(/^\/+/, "") || "index.html";
  const file = resolve(join(ROOT, rel));
  if (!file.startsWith(ROOT) || !existsSync(file)) return route.fulfill({ status: 404, body: "" });
  return route.fulfill({ path: file });
});
await page.goto("http://composition.local/index.html");
await page
  .waitForFunction(() => !!(window.__timelines && window.__timelines.plainly), null, { timeout: 60000, polling: 250 })
  .catch((e) => {
    throw new Error(`${e.message}\n${errors.join("\n")}`);
  });
const cues = await page.evaluate(() => window.__sfx);
await browser.close();
if (errors.length) throw new Error(errors.join("\n"));
writeFileSync("docs/marketing/video/work/sfx.json", JSON.stringify(cues));
console.log(`${cues.length} cues`);
