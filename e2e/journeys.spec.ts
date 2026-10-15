import { expect, test, type Page } from "@playwright/test";

test("first run: download the recommended model, then try it", async ({ page }) => {
  await page.goto("/index.html");
  await page.getByRole("button", { name: "Get started" }).click();
  await expect(page.getByText("This computer has 8 GB of memory.")).toBeVisible();
  await page.getByRole("button", { name: /Download \(2\.4 GB\)/ }).click();
  await expect(page.getByRole("progressbar")).toBeVisible();
  await expect(page.getByText("In use")).toBeVisible({ timeout: 10_000 });
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByRole("heading", { name: "Try it" })).toBeVisible();
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.getByRole("heading", { name: "Home" })).toBeVisible();
  await expect(page.getByText("Ready", { exact: true })).toBeVisible();
});

test("the card opens on a web search; Explain with AI runs only when opened", async ({ page }) => {
  await page.goto("/popup.html?demo=passage");
  await expect(page.getByRole("tab", { name: "Google", selected: true })).toBeVisible();
  await expect(page.getByText("google.com")).toBeVisible();
  await expect(page.getByText(/Whatever else this agreement says/)).toHaveCount(0);
  await page.getByRole("tab", { name: "Bing" }).click();
  await expect(page.getByText("bing.com")).toBeVisible();
  await page.getByRole("tab", { name: "Explain with AI" }).click();
  await expect(page.getByText(/Whatever else this agreement says/)).toBeVisible({ timeout: 5000 });
});

test("offline, a passage is explained at once", async ({ page }) => {
  await page.goto("/popup.html?demo=passage&offline=1");
  await expect(page.getByRole("tab")).toHaveCount(0);
  await expect(page.getByText(/Whatever else this agreement says/)).toBeVisible({ timeout: 5000 });
});

test("the reading level switch redoes the text", async ({ page }) => {
  await page.goto("/popup.html?demo=passage&tab=explain");
  await expect(page.getByText(/Whatever else this agreement says/)).toBeVisible({ timeout: 5000 });
  await page.getByRole("radio", { name: "Simpler" }).click();
  await expect(page.getByText(/The tenant \(the Lessee\) must pay \$1,200 rent every month/)).toBeVisible({ timeout: 8000 });
});

test("the meaning check is shown when a fact is missing", async ({ page }) => {
  await page.goto("/popup.html?demo=check&tab=explain");
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
  await page.getByRole("button", { name: "Change", exact: true }).click();
  await expect(page.getByText("Press the new shortcut…")).toBeVisible();
  await page.keyboard.press("Control+Alt+KeyR");
  await expect(page.getByRole("main").getByText("R", { exact: true })).toBeVisible();
  // The sidebar shows the new shortcut too.
  await expect(page.getByRole("complementary").getByText("R", { exact: true })).toBeVisible();
});

test("home warns when another app owns the shortcut", async ({ page }) => {
  await page.goto("/index.html?state=ready&hotkey=taken");
  await expect(page.getByRole("alert")).toContainText("Another app is already using Ctrl+Shift+Space.");
  await page.getByRole("button", { name: "Choose another shortcut" }).click();
  await expect(page.getByRole("heading", { name: "Shortcuts" })).toBeVisible();
});

test("a word has its dictionary tab; Explain with AI starts the model", async ({ page }) => {
  await page.goto("/popup.html?demo=word");
  await expect(page.getByRole("tab", { name: "Google", selected: true })).toBeVisible();
  await page.getByRole("tab", { name: "Dictionary" }).click();
  await expect(page.getByText("being present everywhere at once")).toBeVisible();
  await page.getByRole("tab", { name: "Explain with AI" }).click();
  await expect(page.getByText(/smartphones are part of nearly every moment/)).toBeVisible({ timeout: 5000 });
  await page.getByRole("tab", { name: "Dictionary" }).click();
  await expect(page.getByText("being present everywhere at once")).toBeVisible();
});

test("offline and without a model, a word opens on its entry and the AI tab says how to set up", async ({ page }) => {
  await page.goto("/popup.html?demo=word_offline");
  await expect(page.getByText("being present everywhere at once")).toBeVisible();
  await expect(page.getByRole("tab")).toHaveCount(2);
  await page.getByRole("tab", { name: "Explain with AI" }).click();
  await expect(page.getByRole("button", { name: "Set up" })).toBeVisible();
});

test("sources: offline only, and adding a site checks its address", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=sources");
  await page.getByLabel("Offline only").click();
  await expect(page.getByLabel("Offline only")).toBeChecked();
  await page.getByLabel("Name").fill("Merriam-Webster");
  await page.getByLabel("Address").fill("https://www.merriam-webster.com/dictionary/");
  await page.getByRole("button", { name: "Add" }).click();
  await expect(page.getByRole("alert")).toContainText("{query}");
  await page.getByLabel("Address").fill("https://www.merriam-webster.com/dictionary/{query}");
  await page.getByRole("button", { name: "Add" }).click();
  await expect(page.getByText("merriam-webster.com", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Move Merriam-Webster up" }).click();
  await page.getByRole("button", { name: "Remove Merriam-Webster" }).click();
  await expect(page.getByText("merriam-webster.com", { exact: true })).toHaveCount(0);
});

test("models already on the computer are found and can be used", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=model");
  await expect(page.getByText("llama3.2:3b")).toBeVisible();
  await expect(page.getByRole("button", { name: "Use Mistral-Small-24B-Q4_K_M" })).toBeDisabled();
  await page.getByRole("button", { name: "Use llama3.2:3b" }).click();
  await expect(page.getByRole("button", { name: "Use llama3.2:3b" })).toHaveCount(0);
  await page.getByRole("button", { name: "Look again" }).click();
  await expect(page.getByText("Qwen3-4B-Instruct-2507-Q4_K_M")).toBeVisible();
});

type MockCall = { command: string; args: Record<string, unknown> };
const mockCalls = (page: Page) => page.evaluate(() => (window as unknown as { __teMockCalls: MockCall[] }).__teMockCalls);

test("look up box: Enter looks the text up, Esc hides it", async ({ page }) => {
  await page.goto("/lookup.html");
  const box = page.getByRole("textbox", { name: "Look up a word or phrase" });
  await expect(box).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("alert")).toHaveText("Type a word or phrase first.");
  await box.fill("ubiquitous");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await box.press("Enter");
  await expect(box).toHaveValue("");
  expect((await mockCalls(page)).filter((c) => c.command === "look_up")).toEqual([{ command: "look_up", args: { text: "ubiquitous" } }]);
  await box.fill("half typed");
  await page.keyboard.press("Escape");
  await expect(box).toHaveValue("");
  expect((await mockCalls(page)).map((c) => c.command)).toContain("lookup_hide");
  expect((await mockCalls(page)).filter((c) => c.command === "look_up")).toHaveLength(1);
});

test("settings: the Look up box gets its own shortcut, never the main one", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=shortcuts");
  await page.getByRole("button", { name: "Set Look up shortcut" }).click();
  await page.keyboard.press("Control+Shift+Space");
  await expect(page.getByText("That's already used by the other shortcut.")).toBeVisible();
  await page.getByRole("button", { name: "Set Look up shortcut" }).click();
  await expect(page.getByText("Press the new shortcut…")).toBeVisible();
  await page.keyboard.press("Control+Alt+KeyL");
  await expect(page.getByRole("button", { name: "Change Look up shortcut" })).toBeVisible();
  await expect(page.getByRole("main").getByText("L", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Turn off" }).click();
  await expect(page.getByRole("button", { name: "Set Look up shortcut" })).toBeVisible();
});

test("settings: apps the card doesn't open in", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=shortcuts");
  const list = page.getByRole("list", { name: "Apps the card doesn't open in" });
  await expect(list.getByText("keepass.exe", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Remove keepass.exe" }).click();
  await expect(list.getByText("keepass.exe", { exact: true })).toHaveCount(0);
  await expect(list.getByText("keepassxc.exe", { exact: true })).toBeVisible();

  const field = page.getByLabel("Program name");
  const add = page.getByRole("button", { name: "Add", exact: true });
  await field.fill("Notes");
  await add.click();
  await expect(page.getByRole("alert")).toContainText(".exe");
  await field.fill("C:\\Apps\\Notes.EXE");
  await add.click();
  await expect(list.getByText("notes.exe", { exact: true })).toBeVisible();
  await expect(field).toHaveValue("");
  await field.fill("NOTES.exe");
  await add.click();
  await expect(page.getByRole("alert")).toHaveText("notes.exe is already on the list.");

  // The app that was in front at the last shortcut is one click away.
  await page.getByRole("button", { name: "Add notepad.exe" }).click();
  await expect(list.getByText("notepad.exe", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Add notepad.exe" })).toHaveCount(0);
});
