# Launch video

About 80 s, 1920 × 1080, 60 fps, built in HyperFrames from the app's real interface, with
narration. The narration sets the pace: each scene lasts as long as its line plus time to
read what is on screen, and from the beat drop on every scene starts on a beat.

- Voice: Kokoro v1.0 (Apache-2.0, runs locally), voice `af_heart`, speed 0.94, each
  sentence voiced separately and joined with a 0.34 s pause. faster-whisper times every
  word, so the picture lands on the spoken words, and checks the final mix is clearly heard.
- Music: "Happy Beats & Business Moves Vol. 12" by Sascha Ende (ende.app, CC BY 4.0,
  credited in the last frame). It starts under the cold open so its drop lands on
  "Plainly." It ducks under the voice (sidechain).
- Effects: CC0 (Kenney; keyboard by unicae_games), logged by the composition itself so they
  always match the picture.

## The angle

From dark to plain. It opens in the dark, flying through a tunnel of jargon. On the beat
drop "Plainly" slams in, a saffron highlighter wipes the dark away, and the real card takes
over. After that everything is light and readable, which is the product's promise.

The signature move is the site's saffron highlighter (the logo's highlighted word): it marks
every selection, sweeps under each headline's key words as they are spoken, wipes between
scenes, strikes through the Wi-Fi mark and draws the logo. No glows, gradients or neon.

## Script and storyboard

| Scene | Narration | On screen |
| --- | --- | --- |
| Cold open | Some writing seems designed to keep you out. | A 3D tunnel of jargon; three real lines land in front |
| Re-read | You read it once. Then again. And you're still not sure what it says. | A lease clause; a marker runs along it, faster each time; "Read 1×, 2×, 3×"; *Still not sure what it says?* |
| The drop | Plainly fixes that. | "Plainly" slams in on the beat |
| Select | Select the text, in any app, and press one shortcut. | The clause is selected; Ctrl, Shift, Space land on "press one shortcut" |
| The card | A small card opens right beside it, and explains it in plain words. | The real card opens on its Google tab; Explain with AI streams the plain version |
| Morph | Same meaning. Every number, date and name kept. Just a lot easier to read. | The clause's surviving words fly into the plain version; $1,200, 5th and Lessee light up on "number, date and name"; the grade counts 17 to 9 on "easier" |
| Levels | Need it simpler? Or just untangled? You pick how plain. | Three real cards on a 3D carousel, turned by the app's own switch; *Pick how plain.* |
| Every app | It works wherever you can select text. A paper in your browser. A letter in your inbox. | A paper in a browser (Ctrl+C twice), then a letter in mail (the shortcut), each opening the card |
| Translate | It translates, too. Into Bangla, Spanish, Hindi, and many more. | Walls of language names drift behind; the card switches to Bangla; the names change as they are spoken |
| Private | The AI runs on your own computer, so what you read with it stays there. It even works offline. | A Wi-Fi mark draws itself and is struck through on "offline"; *Private by design.* |
| Words | Look up any word, and keep the ones worth remembering. | Look up types "ubiquitous", the word card opens, Save on "keep", the Words page |
| Check | And it checks its own work. If a number goes missing, the card tells you. | Facts traced from the original to the rewrite; 5% has nowhere to land; the card's real warning |
| Outro | Plainly. Read anything, plainly. | The logo draws itself; *Free preview for Windows 10 and 11.* plainly.omikhan.com |

## Truth check

Every claim is in the README or on the site and is built today. "What you read with it
stays there" is said of the AI (the site: "What you read with them never leaves it"), not of
the web tabs. The cards are the app's real markup captured from its mock; their texts and
grades are the mock's. Written for the video: the jargon, the browser and mail windows and
their plain rewrites (faithful, shown without grades), the morph's excerpt of the clause and
the check scene's rewrite, which really leaves out 5% so the warning is true.

## Rebuild

From the repo root. Once: `npx vite build`, `npx vite` on :1420, then
`node docs/marketing/video/scripts/capture-ui.mjs`. The voice tools live in
`docs/marketing/video/.venv` (`uv venv .venv --python 3.12`, then
`uv pip install kokoro-onnx soundfile faster-whisper`) with the Kokoro model in `models/`
(`gh release download model-files-v1.0 -R thewh1teagle/kokoro-onnx -p kokoro-v1.0.onnx -p voices-v1.0.bin -D docs/marketing/video/models`).

```bash
docs/marketing/video/.venv/Scripts/python.exe -I docs/marketing/video/scripts/voice.py
```

```bash
python docs/marketing/video/scripts/timing.py && node docs/marketing/video/scripts/build.mjs
```

```bash
node docs/marketing/video/scripts/cues.mjs && python docs/marketing/video/scripts/mix.py
```

```bash
cd docs/marketing/video/composition && HYPERFRAMES_NO_TELEMETRY=1 npx hyperframes@0.8.99 render -f 60 -q delivery -o ../out/plainly-launch.mp4
```

Edit `narration.json` for the words, `src/index.html` for the picture; `composition/index.html`
is generated.
