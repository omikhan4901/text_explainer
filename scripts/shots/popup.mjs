// Renders the reading card's demo scenarios to PNGs (development and docs).
// Usage: node scripts/shots/popup.mjs [outDir]   (needs `npx vite` running on :1420)
import { chromium } from "@playwright/test";

const out = process.argv[2] ?? "docs/screenshots";
const browser = await chromium.launch({ executablePath: process.env.PW_CHROMIUM_PATH || undefined });
for (const scheme of ["light", "dark"]) {
  const page = await browser.newPage({ viewport: { width: 520, height: 640 }, deviceScaleFactor: 2, colorScheme: scheme });
  for (const demo of ["passage", "word", "check", "no_model", "loading", "error"]) {
    await page.goto(`http://localhost:1420/popup.html?demo=${demo}`);
    await page.waitForTimeout(demo === "passage" || demo === "check" ? 2600 : 900);
    await page.evaluate((s) => {
      document.documentElement.style.background = s === "dark" ? "#2b2b30" : "#e9e9ec";
      document.body.style.background = "transparent";
    }, scheme);
    const card = page.locator('[role="dialog"]');
    await card.screenshot({ path: `${out}/card-${demo}-${scheme}.png` });
  }
  await page.close();
}
await browser.close();
