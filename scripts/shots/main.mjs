// Renders the main window's screens to PNGs (development and docs).
// Usage: node scripts/shots/main.mjs [outDir]   (needs `npx vite` running on :1420)
import { chromium } from "@playwright/test";

const out = process.argv[2] ?? "docs/screenshots";
const browser = await chromium.launch({ executablePath: process.env.PW_CHROMIUM_PATH || undefined });
for (const scheme of ["light", "dark"]) {
  const page = await browser.newPage({ viewport: { width: 1100, height: 760 }, deviceScaleFactor: 1.5, colorScheme: scheme });
  await page.goto("http://localhost:1420/index.html");
  await page.waitForTimeout(500);
  await page.screenshot({ path: `${out}/main-welcome-${scheme}.png` });
  await page.getByRole("button", { name: "Get started" }).click();
  await page.waitForTimeout(200);
  await page.screenshot({ path: `${out}/main-firstrun-model-${scheme}.png` });
  for (const p of ["home", "model", "reading", "shortcuts", "about"]) {
    await page.goto(`http://localhost:1420/index.html?state=ready&page=${p}`);
    await page.waitForTimeout(500);
    await page.screenshot({ path: `${out}/main-${p}-${scheme}.png`, fullPage: p === "model" });
  }
  await page.close();
}
await browser.close();
