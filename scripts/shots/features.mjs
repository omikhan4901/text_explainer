// Renders the README's feature shots of the reading card (1560 x 1200, the card centred on a
// quiet background). Usage: node scripts/shots/features.mjs [outDir]
// (needs `npx vite` running on :1420)
import { chromium } from "@playwright/test";

const out = process.argv[2] ?? "docs/screenshots";
const browser = await chromium.launch({ executablePath: process.env.PW_CHROMIUM_PATH || undefined });
const page = await browser.newPage({ viewport: { width: 780, height: 600 }, deviceScaleFactor: 2, colorScheme: "light" });

async function shoot(name, demo, then) {
  await page.goto(`http://localhost:1420/popup.html?demo=${demo}`);
  await page.waitForTimeout(2600);
  if (then) await then();
  await page.evaluate(() => {
    document.documentElement.style.background = "#ececee";
    Object.assign(document.body.style, { background: "transparent", minHeight: "100vh", display: "flex", alignItems: "center", justifyContent: "center", margin: "0" });
  });
  await page.waitForTimeout(200);
  await page.screenshot({ path: `${out}/feature-${name}.png` });
}

await shoot("explain", "passage&tab=explain");
await shoot("translate", "passage&tab=explain&lang=bn", async () => {
  await page.getByRole("radio", { name: "Translate" }).click();
  await page.getByText(/ভাড়াটিয়াকে/).first().waitFor({ timeout: 8000 });
  await page.waitForTimeout(1500);
});
await browser.close();
