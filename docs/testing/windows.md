# Testing on a real Windows machine

CI builds, lints and unit-tests the app on Windows, but some things only a person at a
real desktop can check. This takes about 15 minutes. Note anything that fails (with the
app's log: tray icon → Settings → About → Open logs folder) and it will be fixed.

## Install

1. Open the latest **CI** run on GitHub (Actions tab), scroll to **Artifacts**, download
   `text-explainer-windows-installer`, unzip it and run the `.exe`.
2. Windows SmartScreen will say "Windows protected your PC" because the installer isn't
   signed (signing costs money). Click **More info → Run anyway**.

- [ ] It installs without asking for administrator rights.
- [ ] The main window opens with "Make anything readable".

## First run

- [ ] Step 2 says how much memory the computer has and recommends a model that "Fits easily".
- [ ] **Download** shows progress and speed. Closing the window mid-download and reopening
      it from the tray shows **Resume**, which continues instead of starting over.
- [ ] After the download, step 3 lets you click **Explain this paragraph**; a card appears.

## The card (the main thing)

Try each in **Notepad**, **Edge or Chrome**, **Microsoft Word** (if you have it), and a
**PDF** in Edge:

- [ ] Select a paragraph, press **Ctrl+Shift+Space**: the card appears next to the
      selection within a moment and the rewrite streams in.
- [ ] While the card is open, the app you were in **keeps focus**: you can keep typing
      in Notepad and the text goes into Notepad, not the card.
- [ ] **Esc** closes the card. Clicking anywhere else closes it. The X closes it.
- [ ] The **pin** keeps it open when you click elsewhere.
- [ ] **Simpler / Plain / Clearer** rewrites the same text again.
- [ ] **Copy** puts the rewrite on the clipboard (paste it into Notepad).
- [ ] Press **Ctrl+C twice quickly** on selected text: the card appears for that text.
- [ ] Select a single word (e.g. "ubiquitous"): the card shows the dictionary entry at
      once, and "In this text" with the meaning in context.
- [ ] Copy something to the clipboard first, then use the shortcut in an app where the
      card still works: afterwards, pasting gives your **original** clipboard content
      (the app borrowed the clipboard and put it back).
- [ ] With nothing selected, the shortcut shows "Nothing selected".
- [ ] On a second monitor (if you have one), the card appears on that monitor, fully visible.
- [ ] Near the bottom of the screen, the card appears above the selection instead.

## Settings

- [ ] **Shortcuts → Change**: press a new combination; it works immediately.
- [ ] **Reading**: changing font, size and theme changes the next card.
- [ ] **Model → Fine-tuning → Free memory now**: Task Manager shows `llama-server.exe`
      disappear; the next shortcut press loads it again ("Loading the model…").
- [ ] Quit from the tray: `llama-server.exe` is gone from Task Manager too.

## Performance (write the numbers down)

- Time from pressing the shortcut to the card appearing: ____
- Time to the first words of the rewrite (model already loaded): ____
- Memory of `llama-server.exe` in Task Manager: ____
- Laptop model, CPU, RAM: ____
