import { expect, test, type Page } from "@playwright/test";

test("the card saves a word to the Word book and removes it again", async ({ page }) => {
  await page.goto("/popup.html?demo=word");
  const saveButton = page.getByRole("button", { name: "Save to Words" });
  await expect(saveButton).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByRole("button", { name: "Listen" })).toBeVisible();
  await saveButton.click();
  await expect(saveButton).toHaveAttribute("aria-pressed", "true");
  // The AI's answer arrives after saving; the button stays pressed.
  await page.getByRole("tab", { name: "Explain with AI" }).click();
  await expect(page.getByText(/smartphones are part of nearly every moment/)).toBeVisible({ timeout: 5000 });
  await expect(saveButton).toHaveAttribute("aria-pressed", "true");
  await saveButton.click();
  await expect(saveButton).toHaveAttribute("aria-pressed", "false");
});

test("a long passage can't be saved or read aloud", async ({ page }) => {
  await page.goto("/popup.html?demo=passage");
  await expect(page.getByRole("tab", { name: "Google", selected: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Save to Words" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Listen" })).toHaveCount(0);
});

test("words: search, remove and export saved words", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=words");
  await expect(page.getByRole("heading", { name: "4 words" })).toBeVisible();
  const search = page.getByRole("searchbox", { name: "Search saved words" });
  await search.fill("CARTE");
  await expect(page.getByText("carte blanche", { exact: true })).toBeVisible();
  await expect(page.getByText("accrue", { exact: true })).toHaveCount(0);
  await search.fill("50%");
  await expect(page.getByText("Nothing saved matches “50%”.")).toBeVisible();
  await search.fill("");
  await expect(page.getByText("অভিধান", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Remove accrue" }).click();
  await expect(page.getByText("accrue", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "3 words" })).toBeVisible();
  await page.getByRole("button", { name: "Export CSV" }).click();
  await expect(page.getByRole("status")).toHaveText("Saved 3 words to words.csv.");
});

test("words: history can be cleared after a confirm step, and turned off", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=words");
  await page.getByRole("tab", { name: "History" }).click();
  await expect(page.getByText("lessee", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Clear history" }).click();
  await expect(page.getByText("Clear all history? This can't be undone.")).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByText("lessee", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Clear history" }).click();
  await page.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(page.getByText("Nothing here yet.", { exact: false })).toBeVisible();
  await page.getByLabel("Keep history").click();
  await expect(page.getByLabel("Keep history")).not.toBeChecked();
  await expect(page.getByText("History is off, so nothing new is kept.")).toBeVisible();
  await expect(page.getByLabel("Keep for")).toHaveCount(0);
});

test("words: turning history off offers to clear it", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=words&tab=history");
  await expect(page.getByText("lessee", { exact: true })).toBeVisible();
  await page.getByLabel("Keep for").selectOption("90");
  await expect(page.getByLabel("Keep for")).toHaveValue("90");
  await page.getByLabel("Keep history").click();
  await expect(page.getByText("History is off. Clear what's already kept?")).toBeVisible();
  await page.getByRole("button", { name: "Clear history" }).first().click();
  await expect(page.getByText("History is off, so nothing new is kept.")).toBeVisible();
});

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

test("the AI tab shows what you selected and translates it, with the language a search away", async ({ page }) => {
  await page.goto("/popup.html?demo=passage&tab=explain&lang=bn");
  // What you selected, in full, above the plain version.
  const original = page.getByRole("region", { name: "What you selected" });
  await expect(original).toContainText("failing which a late fee equivalent to 5%");
  await expect(original).toContainText("English");
  await expect(page.getByText(/Whatever else this agreement says/)).toBeVisible({ timeout: 5000 });

  await page.getByRole("radio", { name: "Translate" }).click();
  await expect(page.getByText(/ভাড়াটিয়াকে/)).toBeVisible({ timeout: 5000 });
  await expect(original).toContainText("Notwithstanding any provision");
  await expect(page.getByRole("radio", { name: "Simpler" })).toHaveCount(0);

  // One click compares it with the web translator.
  await page.getByRole("button", { name: "Compare in the Translate tab" }).click();
  await expect(page.getByRole("tab", { name: "Translate", exact: true, selected: true })).toBeVisible();
  await page.getByRole("tab", { name: "Explain with AI" }).click();

  // The language is a searchable list: type, Enter.
  await page.getByRole("button", { name: /Translating into Bangla/ }).click();
  await page.getByRole("combobox", { name: "Search languages" }).fill("span");
  await expect(page.getByRole("option")).toHaveCount(1);
  await page.keyboard.press("Enter");
  await expect(page.getByText(/Sin perjuicio/)).toBeVisible({ timeout: 5000 });
  await expect(page.getByRole("button", { name: /Translating into Spanish/ })).toBeVisible();
  // That pick is for this card only: the main translation language isn't changed.
  const saved = await page.evaluate(() =>
    (window as unknown as { __teMockCalls: { command: string }[] }).__teMockCalls.filter((c) => c.command === "save_settings"),
  );
  expect(saved).toHaveLength(0);

  // English text with English as the language: Translate asks where to, and English is off.
  await page.goto("/popup.html?demo=passage&tab=explain");
  await page.getByRole("radio", { name: "Translate" }).click();
  await expect(page.getByRole("listbox", { name: "Languages to translate into" })).toBeVisible();
  await expect(page.getByRole("option", { name: /English/ })).toHaveAttribute("aria-disabled", "true");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("listbox")).toHaveCount(0);

  // A language only the web can do points to the web translator.
  await page.goto("/popup.html?demo=passage&tab=explain&lang=th");
  await page.getByRole("button", { name: "Use the Translate tab" }).click();
  await expect(page.getByRole("tab", { name: "Translate", exact: true, selected: true })).toBeVisible();
});

test("the reading level switch redoes the text", async ({ page }) => {
  await page.goto("/popup.html?demo=passage&tab=explain");
  await expect(page.getByText(/Whatever else this agreement says/)).toBeVisible({ timeout: 5000 });
  await expect(page.getByRole("radio", { name: "Plain", exact: true })).toBeChecked();
  await page.getByRole("radio", { name: "Simpler" }).click();
  await expect(page.getByText(/The tenant \(the Lessee\) must pay \$1,200 rent every month/)).toBeVisible({ timeout: 8000 });
  // The switch and the answer's heading follow the new level.
  await expect(page.getByRole("radio", { name: "Simpler" })).toBeChecked();
  await expect(page.getByRole("radio", { name: "Plain", exact: true })).not.toBeChecked();
  await expect(page.getByText("Simpler English", { exact: true })).toBeVisible();
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

test("about: check for updates now", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=about");
  await expect(page.getByRole("switch", { name: "Check for updates" })).toBeChecked();
  await page.getByRole("button", { name: "Check now" }).click();
  await expect(page.getByRole("status").filter({ hasText: "You have the newest version." })).toBeVisible();
});

test("the card moves by its header and resizes from its corner; a double-click fits it again", async ({ page }) => {
  await page.setViewportSize({ width: 700, height: 640 });
  await page.goto("/popup.html?demo=passage");
  await expect(page.getByText("google.com")).toBeVisible();
  const card = page.getByRole("dialog");
  const drags = async () => (await mockCalls(page)).filter((c) => c.command === "popup_drag").map((c) => c.args.kind);
  expect((await card.boundingBox())?.width).toBe(480);

  // Pressing the header hands the drag to the app; its tabs and buttons still just work.
  await page.getByText(/Notwithstanding/).first().click();
  await page.getByRole("tab", { name: "Bing" }).click();
  await expect(page.getByText("bing.com")).toBeVisible();
  expect(await drags()).toEqual(["move"]);

  // Pressing the corner hands the resize to Windows (the stand-in app saves the window's
  // size): the card fills its window from then on.
  await page.getByTestId("resize-grip").click();
  expect(await drags()).toEqual(["move", "resize"]);
  await expect.poll(async () => (await card.boundingBox())?.width).toBe(700 - 24);
  await expect
    .poll(async () => ((await mockCalls(page)).filter((c) => c.command === "web_show").at(-1)?.args.bounds as { width: number }).width)
    .toBe(700 - 24);

  // Double-click fits it to its content again, and forgets the size (without a resize).
  await page.waitForTimeout(450);
  await page.getByTestId("resize-grip").dblclick();
  expect(await drags()).toEqual(["move", "resize", "resize"]);
  await expect.poll(async () => (await card.boundingBox())?.width).toBe(480);
  expect((await mockCalls(page)).filter((c) => c.command === "set_card_size").at(-1)?.args).toEqual({ size: null });
});

test("Esc closes the card once it has the keyboard, but first closes the language search", async ({ page }) => {
  await page.goto("/popup.html?demo=passage&tab=explain&lang=bn");
  await page.getByRole("radio", { name: "Translate" }).click();
  await expect(page.getByText(/ভাড়াটিয়াকে/)).toBeVisible({ timeout: 5000 });
  const closes = async () => (await mockCalls(page)).filter((c) => c.command === "popup_close").length;

  await page.getByRole("button", { name: /Translating into Bangla/ }).click();
  await page.getByRole("combobox", { name: "Search languages" }).press("Escape");
  await expect(page.getByRole("combobox")).toHaveCount(0);
  expect(await closes()).toBe(0);

  await page.keyboard.press("Escape");
  await expect.poll(closes).toBe(1);
});

test("a card the person sized fills its window on every tab", async ({ page }) => {
  await page.setViewportSize({ width: 640, height: 600 });
  await page.goto("/popup.html?demo=passage");
  await page.getByTestId("resize-grip").click();
  const card = page.getByRole("dialog");
  await expect.poll(async () => (await card.boundingBox())?.height).toBe(600 - 24);
  await page.getByRole("tab", { name: "Explain with AI" }).click();
  await expect(page.getByText(/Whatever else this agreement says/)).toBeVisible({ timeout: 5000 });
  expect((await card.boundingBox())?.height).toBe(600 - 24);
  expect((await card.boundingBox())?.width).toBe(640 - 24);
});

test("model: the model's memory is freed when the card closes, unless chosen otherwise", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=model");
  const free = page.getByRole("combobox", { name: "Free memory after" });
  await expect(free).toHaveValue("close");
  await free.selectOption({ label: "10 min idle" });
  await expect.poll(async () => (await mockCalls(page)).filter((c) => c.command === "save_settings").at(-1)?.args.settings).toMatchObject({
    unload_on_close: false,
    idle_unload_minutes: 10,
  });
  await free.selectOption({ label: "The card closes" });
  await expect(free).toHaveValue("close");
});

test("an accepted update shows its download, and a failed one can be dismissed", async ({ page }) => {
  await page.goto("/index.html?state=ready&update=downloading");
  const dialog = page.getByRole("dialog", { name: "Updating to Plainly 0.1.22" });
  await expect(dialog.getByRole("progressbar", { name: "Downloading" })).toHaveAttribute("aria-valuenow", "40");
  await expect(dialog.getByRole("status")).toHaveText("Downloading · 4.0 MB of 10.0 MB");

  await page.goto("/index.html?state=ready&update=failed");
  await expect(page.getByText("The download was interrupted.")).toBeVisible();
  await page.getByRole("button", { name: "Close" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

test("sources: web pages are in English unless another language is picked", async ({ page }) => {
  await page.goto("/index.html?state=ready&page=sources");
  const lang = page.getByRole("combobox", { name: "Web pages in" });
  await expect(lang).toHaveValue("en");
  const saved = async () => (await mockCalls(page)).filter((c) => c.command === "save_settings").at(-1)?.args.settings as { web_language: unknown } | undefined;

  await lang.selectOption({ label: "বাংলা (Bangla)" });
  await expect.poll(async () => (await saved())?.web_language).toEqual({ mode: "fixed", code: "bn" });
  await lang.selectOption({ label: "Automatic (the site decides)" });
  await expect.poll(async () => (await saved())?.web_language).toEqual({ mode: "automatic" });
  await lang.selectOption({ label: "Same as the translation language (English)" });
  await expect.poll(async () => (await saved())?.web_language).toEqual({ mode: "same_as_translation" });
});
