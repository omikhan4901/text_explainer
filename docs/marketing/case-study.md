# Case study: making any text readable, offline

**Mehboob Ehsan Khan** · 2026 · [github.com/omikhan4901/text_explainer](https://github.com/omikhan4901/text_explainer)

> Status: in development. The Windows MVP is being built in the open; this page is
> updated at each milestone and only describes what exists.

## The problem

Contracts, research papers, medical letters and government forms are written for
specialists. Looking things up one word at a time breaks your focus, and pasting a
private document into a cloud chatbot isn't always possible or wise. Most people also
read on ordinary laptops: 8 GB of memory, no graphics card.

## The goal

Select hard text in any app, press one key, and get a clear version right next to it,
written at the reading level you choose, without changing the meaning and without
anything leaving the computer. It must not get in the way of the work you are doing.

## What I did

- **Chose the model by evidence, not habit.** The first idea was Phi-3 Mini; by 2026 it
  was two generations behind. I built an evaluation harness that runs 28 realistic
  passages through each candidate (Qwen3.5, Gemma 4, LFM2.5) exactly as the app does and
  measures speed, memory, reading-grade drop, facts kept, chatter and language drift, on
  free GitHub runners. A probe workflow pins every model file's size and SHA-256 from
  Hugging Face, so the catalog and every download are verified.
- **Stopped the model drifting into another language at the token level.** Small models,
  Qwen in particular, sometimes switch to Chinese mid-answer. Instead of only asking
  nicely in the prompt, every request carries a generated GBNF grammar that makes
  letters from unexpected writing systems impossible to sample. The answer is checked
  again afterwards (for servers that ignore grammars) and retried once with a stricter
  instruction.
- **Checked that rewrites keep the facts.** A deterministic "meaning guard" compares the
  numbers, dates, money, acronyms and names in the original with the rewrite, forgiving
  about form ("15%" = "15 percent", "1,200,000" = "1.2 million") but strict about
  content, and flags anything missing or invented right on the card.
- **Made it invisible until needed.** The card opens beside the selection without taking
  keyboard focus from the app you're in, grows away from your text as the answer streams
  in, and closes with Esc or a click elsewhere. Two triggers: a shortcut that reads the
  selection through Windows UI Automation (no clipboard), or pressing Ctrl+C twice. When
  an app hides its selection, a fallback copies it and restores the clipboard exactly,
  every format, kept out of clipboard history.
- **Repaired PDF text first.** Copied PDF text is split by hyphens and hard line breaks;
  the app rejoins it before anything else, keeping lists and paragraphs.
- **Gave people full control.** A model catalog with "fits your memory" hints, resumable
  downloads, any GGUF file, a local Ollama or LM Studio server, sampling settings, and
  editable instructions per reading level.

## Results so far

- **The default model was chosen by measurement.** On a 4-core CPU with no GPU, Gemma 4
  E2B rewrote passages in a median of about 6 seconds (first words after about 2) at 17
  tokens per second using 3.9 GB of memory, lowered the reading grade by 9 to 12 levels,
  and kept facts best of the four models tested. The first guess, Qwen3.5 4B, was slower,
  used more memory and, in the first run, dropped drug doses from a discharge letter
  ([full results](../models.md)).
- **The evaluation changed the product**: the default model, the meaning check's rules
  (fewer false alarms on contract terms and shortened names), and the engine's prompt
  cache, sized by measurement after a memory-saving idea made answers 4 to 7 seconds
  slower.
- 111 Rust tests, 10 front-end unit tests and 38 Playwright checks (journeys plus WCAG
  2.2 AA scans in light and dark), run on every push on Linux and on a real Windows
  runner that also builds the installer.
- The Windows app is type-checked and linted from Linux with a small cross-check script,
  so most mistakes are caught before the Windows build runs.

## What I learned

- Guarantees beat instructions. A grammar that removes the wrong scripts from the
  model's choices is more reliable than any prompt asking it to stay in English.
- The best interface for a reading tool is almost none: one card, one row of controls,
  and the app you're reading keeps its focus.
