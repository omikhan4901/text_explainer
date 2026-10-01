import { expect, test } from "@playwright/test";

test("first run: download the recommended model, then try it", async ({ page }) => {
  await page.goto("/index.html");
  await page.getByRole("button", { name: "Get started" }).click();
  await expect(page.getByText("This computer has 8 GB of memory.")).toBeVisible();
  await page.getByRole("button", { name: /Download \(2\.6 GB\)/ }).click();
  await expect(page.getByRole("progressbar")).toBeVisible();
  await expect(page.getByText("In use")).toBeVisible({ timeout: 10_000 });
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByRole("heading", { name: "Try it" })).toBeVisible();
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.getByRole("heading", { name: "Home" })).toBeVisible();
  await expect(page.getByText("Ready", { exact: true })).toBeVisible();
});

test("the reading level switch redoes the text", async ({ page }) => {
  await page.goto("/popup.html?demo=passage");
  await expect(page.getByText(/Whatever else this agreement says/)).toBeVisible({ timeout: 5000 });
  await page.getByRole("radio", { name: "Simpler" }).click();
  await expect(page.getByText(/The tenant \(the Lessee\) must pay \$1,200 rent every month/)).toBeVisible({ timeout: 8000 });
});

test("the meaning check is shown when a fact is missing", async ({ page }) => {
  await page.goto("/popup.html?demo=check");
  await expect(page.getByText("Not found in the rewrite:")).toBeVisible({ timeout: 5000 });
  await expect(page.getByText("5%", { exact: true })).toBeVisible();
});

test("settings: changing the theme applies it at once", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=reading");
  await page.getByRole("radio", { name: "Dark" }).click();
  await expect(page.locator("html")).toHaveClass(/dark/);
  await page.getByRole("radio", { name: "Light" }).click();
  await expect(page.locator("html")).not.toHaveClass(/dark/);
});

test("settings: a new shortcut is recorded from the keyboard", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=shortcuts");
  await page.getByRole("button", { name: "Change" }).click();
  await expect(page.getByText("Press the new shortcut…")).toBeVisible();
  await page.keyboard.press("Control+Alt+KeyR");
  await expect(page.getByText("R", { exact: true })).toBeVisible();
});
