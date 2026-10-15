import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

async function expectAccessible(page: Page) {
  const results = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"]).analyze();
  expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
}

for (const demo of [
  "passage",
  "passage&tab=explain",
  "passage&offline=1",
  "check&tab=explain",
  "word",
  "word&tab=dictionary",
  "word&tab=explain",
  "word_offline",
  "word_offline&tab=explain",
  "no_model",
  "no_selection",
  "loading&tab=explain",
  "error&tab=explain",
]) {
  test(`reading card (${demo}) is accessible`, async ({ page }) => {
    await page.goto(`/popup.html?demo=${demo}`);
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.waitForTimeout(demo.includes("explain") || demo.includes("offline=1") ? 2500 : 700);
    await expectAccessible(page);
  });
}

for (const p of ["home", "sources", "model", "reading", "shortcuts", "about"]) {
  test(`main window: ${p} is accessible`, async ({ page }) => {
    await page.goto(`/index.html?state=ready&page=${p}`);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expectAccessible(page);
  });
}

test("first run is accessible", async ({ page }) => {
  await page.goto("/index.html");
  await expectAccessible(page);
  await page.getByRole("button", { name: "Get started" }).click();
  await expectAccessible(page);
});
