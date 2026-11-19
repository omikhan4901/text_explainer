// Captures the real interface for the launch video: the markup of the card, the Look up
// box and the Words page in the states the video shows, rendered by the app against its
// mock. The video then styles that markup with the app's own built CSS, so the UI stays
// sharp at any zoom.
//
// Usage (from the repo root, with `npx vite` running on :1420 and `npx vite build` done):
//   node docs/marketing/video/scripts/capture-ui.mjs
import { chromium } from "@playwright/test";
import { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const VIDEO = "docs/marketing/video";
const SNAP = join(VIDEO, "composition", "ui");
const APP = join(VIDEO, "composition", "assets", "app");
mkdirSync(SNAP, { recursive: true });
mkdirSync(APP, { recursive: true });

// The app's built stylesheets and the fonts they load. The CSS points at `/assets/`;
// here the fonts sit next to it.
for (const f of readdirSync("dist/assets")) {
  if (f.endsWith(".woff2")) copyFileSync(join("dist/assets", f), join(APP, f));
  if (f.endsWith(".css")) {
    const css = readFileSync(join("dist/assets", f), "utf8").replaceAll("url(/assets/", "url(./");
    writeFileSync(join(APP, f), css);
  }
}

const base = "http://localhost:1420";
// The installed Chrome, so no Playwright browser download is needed.
const browser = await chromium.launch({ channel: "chrome" });
const page = await browser.newPage({ viewport: { width: 520, height: 700 }, colorScheme: "light", deviceScaleFactor: 2 });

/** Markup of `selector`, made safe to place many times in one page: no ids, no Radix
 * cross-references, no mount animations. */
async function grab(selector) {
  return page.evaluate((sel) => {
    const el = document.querySelector(sel).cloneNode(true);
    for (const n of [el, ...el.querySelectorAll("*")]) {
      for (const a of ["id", "aria-controls", "aria-labelledby", "tabindex"]) n.removeAttribute?.(a);
      if (n.classList?.contains("card-in")) n.classList.remove("card-in");
    }
    return el.outerHTML;
  }, selector);
}

async function card(name, url, { wait = 1200, then } = {}) {
  await page.goto(`${base}/popup.html?${url}`);
  await page.waitForTimeout(wait);
  if (then) await then();
  const html = await grab('[role="dialog"]');
  writeFileSync(join(SNAP, `${name}.html`), html);
  await page.locator('[role="dialog"]').screenshot({ path: join(SNAP, `${name}.png`) });
  console.log("card", name, html.length);
}

const radio = (label) => async () => {
  await page.getByRole("radio", { name: label }).click();
  await page.waitForTimeout(900);
};

await card("card-google", "demo=passage");
await card("card-plain", "demo=still&tab=explain", { wait: 1500 });
await card("card-simpler", "demo=still&tab=explain", { wait: 1500, then: radio("Simpler") });
await card("card-clearer", "demo=still&tab=explain", { wait: 1500, then: radio("Clearer") });
await card("card-bangla", "demo=still&tab=explain&lang=bn", { wait: 1500, then: radio("Translate") });
await card("card-word", "demo=word&tab=dictionary");
await card("card-check", "demo=check&tab=explain", { wait: 4500 });

// The Look up box, empty (the video types into it).
await page.goto(`${base}/lookup.html`);
await page.waitForTimeout(800);
writeFileSync(join(SNAP, "lookup.html"), await grab("#root > *"));
console.log("lookup");

// The Words page of the main window.
await page.setViewportSize({ width: 1040, height: 680 });
await page.goto(`${base}/index.html?state=ready&page=words`);
await page.waitForTimeout(1200);
writeFileSync(join(SNAP, "main-words.html"), await grab("#root > *"));
await page.screenshot({ path: join(SNAP, "main-words.png") });
console.log("words");

await browser.close();
