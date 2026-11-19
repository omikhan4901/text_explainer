// Records the README's animated GIFs from the website's own animations (site/app/demo).
// The page clock is frozen and stepped one frame at a time, so every frame is exact and the
// GIFs come out smooth and the same on every run.
// Usage: node scripts/shots/readme-gifs.mjs [outDir]   (needs the site on :3000:
//        `npm --prefix site run dev`, and ffmpeg)
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

const OUT = process.argv[2] ?? "docs/screenshots";
const FPS = 15;
const SHOTS = [
  // [file, heading to scroll to (null: the hero), element to film, seconds, width, replay first]
  ["demo.gif", null, ".stage-frame", 11.2, 640, false],
  ["morph.gif", /Same meaning/, "section#how", 7, 720, true],
  ["levels.gif", /Pick how/, "section:has(h2)", 14, 720, false],
];

const browser = await chromium.launch({ channel: "chrome" });
for (const [file, heading, selector, seconds, width, replay] of SHOTS) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 860 }, deviceScaleFactor: 1.5, colorScheme: "light" });
  await page.clock.install();
  await page.goto("http://localhost:3000/");
  await page.evaluate(() => document.fonts.ready);
  // The sticky header and the dev server's badge would sit on top of the sections.
  await page.addStyleTag({ content: "header, nextjs-portal { display: none !important; }" });
  await page.clock.runFor(500);
  let target = page.locator(selector).first();
  if (heading) {
    const h = page.getByRole("heading", { name: heading }).first();
    await h.evaluate((e) => e.scrollIntoView({ block: "start" }));
    target = h.locator(`xpath=ancestor::section[1]`);
  } else {
    await target.evaluate((e) => e.scrollIntoView({ block: "center" }));
  }
  await page.clock.runFor(100);
  if (replay) {
    // Scenes that play once start the moment they are seen: let it finish, then replay it
    // so the GIF opens on the start.
    await page.clock.runFor(7000);
    await target.getByRole("button", { name: /Replay/ }).click();
  }
  const frames = join(tmpdir(), `readme-gif-${file}`);
  rmSync(frames, { recursive: true, force: true });
  mkdirSync(frames, { recursive: true });
  const n = Math.round(seconds * FPS);
  for (let i = 0; i < n; i++) {
    await page.clock.runFor(1000 / FPS);
    await target.screenshot({ path: join(frames, `f${String(i).padStart(4, "0")}.png`) });
  }
  await page.close();
  execFileSync("ffmpeg", [
    "-hide_banner", "-loglevel", "error", "-y", "-framerate", String(FPS), "-i", join(frames, "f%04d.png"),
    // Replayed scenes hold their first frame a moment, so the "before" can be read.
    "-vf", `${replay ? "tpad=start_mode=clone:start_duration=1.2," : ""}scale=${width}:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=96:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle`,
    "-loop", "0", join(OUT, file),
  ]);
  console.log(file, n, "frames");
}
await browser.close();
