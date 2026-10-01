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

| Model | File | Peak memory | Speed | Whole answer (median) | First words (median) | Reading grade drop (Plain / Simpler) | Answers missing a fact | Answers with an invented number | Chatter | Wrong language without the grammar |
|---|---|---|---|---|---|---|---|---|---|---|
| **Gemma 4 E2B** (default) | 2.44 GB | 3.93 GB | 16.9 tok/s | 6.0 s | 2.2 s | 9.1 / 12.7 | 4 of 56 | 0 | 0 | 0 of 12 |
| Qwen3.5 4B | 2.55 GB | 4.73 GB | 8.4 tok/s | 11.5 s | 4.1 s | 9.6 / 11.9 | 14 of 56 | 0 | 0 | 0 of 12 |
| Qwen3.5 2B | 1.19 GB | 2.14 GB | 17.0 tok/s | 4.6 s | 1.5 s | 7.1 / 11.9 | 14 of 56 | 0 | 0 | 0 of 12 |
| LFM2.5 1.2B | 0.68 GB | 1.34 GB | 45.0 tok/s | 3.2 s | 2.3 s | 10.6 / 12.6 | 36 of 56 | 2 | 0 | 0 of 12 |

"Answers missing a fact" is the app's meaning check (numbers, dates, money, acronyms and
names that didn't survive the rewrite). It errs towards flagging, so not every flag is a
real loss, but the relative numbers are telling. Reading grades are Flesch–Kincaid, English
passages only, capped at 18.

## What the numbers mean

- **Gemma 4 E2B is the default.** It is as fast as the 2B Qwen model on a CPU (it
  computes like a 2B model), twice as fast as Qwen3.5 4B, and it kept facts far better
  than any other model: it was the only one to keep every drug dose in the discharge
  letter. Its rewrites are slightly wordier ("This… This…") but faithful.
- **Qwen3.5 4B** writes well but is half as fast on a CPU, uses the most memory, and
  dropped the "1 g" and "500 mg" doses from the discharge letter. Offered as an option.
- **Qwen3.5 2B** is the pick for machines with little memory (it needs about 2 GB). It
  sometimes drops whole clauses (the Sale of Goods Act 1979 reference, for example).
- **LFM2.5 1.2B** is remarkably fast, but not trustworthy for this job: it dropped facts in
  most answers, copied "$1,200 rent" from the prompt's worked example into an insurance
  clause (the meaning check caught it as an invented number), and wrote broken Bangla and
  Hindi. It stays in the catalog, clearly labelled, for people who want speed above all.

## The language guard

None of the four models drifted into another language in these runs, even with the
grammar turned off (12 trials each). The grammar costs nothing measurable, so it stays on:
drift in small models is rare but real, and a reader seeing a paragraph of Chinese in the
middle of a lease is the kind of failure that loses trust for good.

## What changed because of this evaluation

- The default model moved from Qwen3.5 4B (the initial guess) to Gemma 4 E2B.
- The meaning check stopped flagging capitalised contract terms ("Lessee", "Agreement"),
  shortened names ("United Kingdom" → "UK", "University of Cambridge" → "Cambridge") and
  idiomatic number words ("first", "zero").
- The model engine stopped keeping an extra prompt cache in memory (`--cache-ram 0`).
- The model catalog's "fits your memory" hints use these measured peaks.

## Re-running it

Actions → **Model evaluation** → Run workflow, with a list of catalog model ids. Each model
runs on its own free runner in parallel; results appear in the run summary and as
artifacts (every answer, for reading them yourself).
