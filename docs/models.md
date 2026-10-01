# Choosing the default model

The default model was chosen by measurement, not by benchmark tables. `crates/eval` runs
`eval/corpus.jsonl` (28 passages: legal, medical, academic, technical, government,
finance, news, plus Spanish, Bangla, Hindi and French) through each model **exactly as the
app does**: same text repair, prompts, language grammar, output filter and checks. Each
passage is rewritten at two levels (Plain and Simpler), so 56 answers per model.

It runs in `.github/workflows/eval.yml` on a standard GitHub runner (4 vCPU, 16 GB, no
GPU), which is close to an ordinary laptop CPU. llama.cpp b11323, CPU build, 4,096-token
context, temperature 0.3.

## Results (1 October 2026)

With the app's engine settings (prompt cache capped at 1024 MiB, see below). Each row is
one model on its own runner; the last column says which run it comes from.

| Model | File | Peak memory | Speed | Whole answer (median) | First words (median) | Reading grade drop (Plain / Simpler) | Answers missing a fact | Answers with an invented number | Chatter | Wrong language without the grammar | Run |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **Gemma 4 E2B** (default) | 2.44 GB | 3.93 GB | 17.3 tok/s | 5.8 s | 2.2 s | 9.2 / 12.2 | 1 of 56 | 0 | 0 | 0 of 12 | 5 |
| Qwen3.5 4B | 2.55 GB | 4.74 GB | 10.8 tok/s | 7.9 s | 2.4 s | 9.6 / 12.1 | 2 of 56 | 0 | 0 | 0 of 12 | 5 |
| Qwen3.5 2B | 1.19 GB | 2.13 GB | 12.9 tok/s | 5.4 s | 1.3 s | 7.6 / 11.3 | 8 of 56 | 2 | 0 | 0 of 12 | 4 |
| LFM2.5 1.2B | 0.68 GB | 1.34 GB | 45.0 tok/s | 3.2 s | 2.3 s | 10.6 / 12.6 | 36 of 56 | 2 | 0 | 0 of 12 | 2 |

Run 2 used the engine's default cache and the first version of the meaning check; runs 4
and 5 used the improved check (fewer false alarms), with a 256 MiB and a 1024 MiB cache.
Qwen3.5 2B's speed is fine at 256 MiB, so it wasn't re-run.

"Answers missing a fact" is the app's meaning check (numbers, dates, money, acronyms and
names that didn't survive the rewrite). It errs towards flagging, so not every flag is a
real loss, but the relative numbers are telling. Reading grades are Flesch–Kincaid, English
passages only, capped at 18.

**Runs vary.** Free runners differ in speed: the same model's tokens per second moved by
up to a quarter between runs (Gemma 4 E2B: 16.9 to 20.3; Qwen3.5 2B: 12.9 to 17.0), so
compare speeds within a run, not across runs. Answers are sampled (temperature 0.3), so
the fact counts move a little too: across the four runs with the improved meaning check,
Gemma 4 E2B was flagged in 1 to 3 answers of 56 and Qwen3.5 4B in 2 to 6.

## What the numbers mean

- **Gemma 4 E2B is the default.** It computes like a 2B model, so it is as fast as the
  2B Qwen model on a CPU, and it kept facts better than any other model: it was the only
  one to keep every drug dose in the discharge letter in the first run. Its rewrites are
  slightly wordier ("This… This…") but faithful.
- **Qwen3.5 4B** writes well and keeps facts nearly as well, but generates about a third
  slower on a CPU and needs the most memory. In the first run it dropped the "1 g" and
  "500 mg" doses from the discharge letter. Offered as an option.
- **Qwen3.5 2B** is the pick for machines with little memory (it needs about 2 GB). It
  sometimes drops whole clauses (the Sale of Goods Act 1979 reference, for example).
- **LFM2.5 1.2B** is remarkably fast, but not trustworthy for this job: it dropped facts in
  most answers, copied "$1,200 rent" from the prompt's worked example into an insurance
  clause (the meaning check caught it as an invented number), and wrote broken Bangla and
  Hindi. It stays in the catalog, clearly labelled, for people who want speed above all.

## The language guard

None of the four models drifted into another language in any run, even with the
grammar turned off (12 trials each per run). The grammar costs nothing measurable, so it stays on:
drift in small models is rare but real, and a reader seeing a paragraph of Chinese in the
middle of a lease is the kind of failure that loses trust for good.

## What changed because of this evaluation

- The default model moved from Qwen3.5 4B (the initial guess) to Gemma 4 E2B.
- The meaning check stopped flagging capitalised contract terms ("Lessee", "Agreement"),
  shortened names ("United Kingdom" → "UK", "University of Cambridge" → "Cambridge") and
  idiomatic number words ("first", "zero").
- The engine's prompt cache is capped at 1024 MiB. Turning it off (run 3) made every
  answer re-read the instructions: Gemma 4 E2B's first words took 9.1 s instead of 2.2 s,
  for no memory saved. A 256 MiB cap (run 4) suited the 2B models but not Qwen3.5 4B
  (16.7 s to the first words); 1024 MiB brought it to 2.4 s, and 2048 MiB gained little
  more. The cap keeps the engine's 8 GB default from growing on an 8 GB laptop.
- The model catalog's "fits your memory" hints use these measured peaks.

## Re-running it

Actions → **Model evaluation** → Run workflow, with a list of catalog model ids. Each model
runs on its own free runner in parallel; results appear in the run summary and as
artifacts (every answer, for reading them yourself). "Extra llama-server arguments" are
appended to the app's own, to compare engine settings on the same passages.
